use crate::{
    accessor::{
        entry::handle::DirEntry,
        error::{AccessorError, AccessorResult},
        filesystem::host::api::HostFs,
        io::reader::AccessorReader,
        location::path::InnerPath,
    },
    artifacts::os::files::artifact::files_output_name,
    filesystem::files::{hash_file_data, hash_reader},
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::{artifacts::os::files::FileOptions, toml::OutputFormat},
    utils::regex_options::{create_regex, regex_check},
};
use common::files::{EntryKind, FileHostInfo, Hashes};
use regex::Regex;
use serde_json::Value;
use std::{collections::HashSet, mem::take, path::PathBuf};
use tracing::{error, info, warn};

/// Max size of file we read into memory if we need to parse binaries or scan with Yara
const YARA_MAX_SIZE: u64 = 50 * 1024 * 1024;

/// Walk the live filesystem and output results
pub(super) fn walk_host(
    inner: &InnerPath,
    options: &FileOptions,
    manager: &mut OutputManager,
    yara_rule: &str,
    evidence: &str,
) -> AccessorResult<()> {
    let mut exclude: HashSet<String> = options
        .exclude_directories
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();
    load_ignore_paths(&mut exclude);

    let path_filter = create_regex(options.path_regex.as_deref().unwrap_or(""))
        .map_err(|_err| AccessorError::location(&options.start_path, "invalid path_regex"))?;
    let file_filter = create_regex(options.filename_regex.as_deref().unwrap_or(""))
        .map_err(|_err| AccessorError::location(&options.start_path, "invalid filename_regex"))?;

    let max_list =
        if options.metadata.is_some_and(|b| b) || manager.config.format == OutputFormat::Timeline {
            1000
        } else {
            10000
        };

    let mut listing = HostListing {
        options,
        manager,
        yara_rule,
        evidence,
        path_filter,
        file_filter,
        hashes: Hashes {
            md5: options.md5.unwrap_or_default(),
            sha1: options.sha1.unwrap_or_default(),
            sha256: options.sha256.unwrap_or_default(),
        },
        exclude: &exclude,
        max_list,
        max_depth: options.depth.unwrap_or(1),
        depth: 1,
        batch: Vec::new(),
    };

    walk_host_dir(inner, &mut listing)?;

    if !listing.batch.is_empty() {
        host_output(take(&mut listing.batch), listing.manager, listing.options);
    }

    Ok(())
}

/// Parameters when walking a live system
struct HostListing<'a> {
    /// Options for filelisting
    ///
    /// Contains parameters like yara scanning, PE parsing, path
    /// and file filter
    options: &'a FileOptions,
    /// `OutputManager` to send results to
    manager: &'a mut OutputManager,
    /// Yara rule if Yara scanning is enabled
    yara_rule: &'a str,
    /// Source of our filelisting
    ///
    /// Will often be the start path
    evidence: &'a str,
    /// Path filter regex
    path_filter: Regex,
    /// File filter regex
    file_filter: Regex,
    /// Hashes we should use if we want to hash files
    hashes: Hashes,
    /// Directories to ignore when doing a filelisting
    exclude: &'a HashSet<String>,
    /// Max batch size for streaming results
    /// Default is 10k unless PE or timelining
    ///
    /// Then default becomes 1k
    max_list: usize,
    /// Current filelisting depth from the start path in `FileOptions`
    depth: u32,
    /// The max depth we are descending
    max_depth: u32,
    /// Array to stream filelisting results
    batch: Vec<FileHostInfo>,
}

/// Start walking the filesystem
fn walk_host_dir(inner: &InnerPath, listing: &mut HostListing<'_>) -> AccessorResult<()> {
    let entries = HostFs::read_dir(inner)?;

    for entry in entries {
        if listing.exclude.contains(&entry.meta.full_path) {
            continue;
        }

        let mut info = fill_host(&entry, listing.depth, listing.evidence);

        if row_matches(listing, &info) {
            let emit = if info.kind == EntryKind::File {
                match enrich_host_file(&entry, &mut info, listing) {
                    Ok(keep) => keep,
                    Err(err) => {
                        warn!("Failed to read {}: {err:?}", info.display_path);
                        true
                    }
                }
            } else {
                true
            };

            if emit {
                push_row(listing, info);
            }
        }

        if entry.is_directory() && listing.depth < listing.max_depth {
            listing.depth += 1;
            let child = InnerPath::new(PathBuf::from(&entry.meta.full_path));

            if let Err(err) = walk_host_dir(&child, listing) {
                warn!("Could not descend into {}: {err:?}", entry.meta.full_path);
            }
            listing.depth -= 1;
        }
    }

    Ok(())
}

/// Check if we should filter files by regex
fn row_matches(listing: &HostListing<'_>, info: &FileHostInfo) -> bool {
    (listing.options.path_regex.is_none() || regex_check(&listing.path_filter, &info.full_path))
        && (listing.options.filename_regex.is_none()
            || regex_check(&listing.file_filter, &info.filename))
}

/// Create the `FileHostInfo` entry
fn fill_host(entry: &DirEntry, depth: u32, evidence: &str) -> FileHostInfo {
    FileHostInfo {
        full_path: entry.meta.full_path.clone(),
        directory: entry.meta.directory.clone(),
        filename: entry.meta.filename.clone(),
        extension: entry.meta.extension.clone(),
        created: entry.times.created.clone(),
        modified: entry.times.modified.clone(),
        changed: entry.times.changed.clone(),
        accessed: entry.times.accessed.clone(),
        uid: entry.meta.uid.to_string(),
        gid: entry.meta.gid.to_string(),
        inode: entry.meta.inode,
        attributes: entry.meta.attributes.clone(),
        size: entry.meta.size,
        kind: entry.meta.kind.clone(),
        depth: depth as usize,
        display_path: entry.meta.display_path.clone(),
        evidence: evidence.to_string(),
        ..Default::default()
    }
}

/// If binary parsing, Yara scanning
/// or hashing is enabled
///
/// Read each file entry
///
/// File reading only occurs for
/// files that are smaller
/// than `YARA_MAX_SIZE`
fn enrich_host_file(
    entry: &DirEntry,
    host_info: &mut FileHostInfo,
    listing: &HostListing<'_>,
) -> AccessorResult<bool> {
    let want_hash = listing.hashes.md5 || listing.hashes.sha1 || listing.hashes.sha256;
    let want_bin = listing.options.metadata.is_some_and(|b| b);

    #[cfg(feature = "yarax")]
    let want_yara = !listing.yara_rule.is_empty();

    #[cfg(not(feature = "yarax"))]
    let want_yara = false;

    if !want_hash && !want_bin && !want_yara {
        return Ok(true);
    }

    if want_yara && host_info.size > YARA_MAX_SIZE {
        info!(
            "Skipping file {}. File size is {} vs 50MB max scans size",
            host_info.display_path, host_info.size
        );
        return Ok(false);
    }

    let need_bytes = want_yara || (want_bin && host_info.size <= YARA_MAX_SIZE);
    let mut bytes = Vec::new();

    if need_bytes {
        bytes = if let Some(handle) = entry.handle.as_file() {
            HostFs::read_handle(handle, Some(YARA_MAX_SIZE))?
        } else {
            return Ok(false);
        }
    }

    #[cfg(feature = "yarax")]
    if want_yara {
        use crate::utils::yara::scan_bytes;

        match scan_bytes(&bytes, listing.yara_rule) {
            Ok(hits) if !hits.is_empty() => host_info.yara_hits = hits,
            Ok(_) => return Ok(false),
            Err(err) => {
                warn!("Failed to scan with yara: {err:?}");
                return Ok(false);
            }
        }
    }

    if want_hash && !bytes.is_empty() {
        let (md5, sha1, sha256) = hash_file_data(&listing.hashes, &bytes);
        host_info.md5 = md5;
        host_info.sha1 = sha1;
        host_info.sha256 = sha256;
    } else if want_hash
        && let Some(handle) = entry.handle.as_file()
        && let Ok(mut reader) = HostFs::reader_handle(handle)
    {
        let (md5, sha1, sha256) = hash_reader(&listing.hashes, &mut reader);
        host_info.md5 = md5;
        host_info.sha1 = sha1;
        host_info.sha256 = sha256;
    }

    if want_bin && !bytes.is_empty() {
        host_info.binary_info = parse_host_binary(bytes, &host_info.display_path);
    }

    Ok(true)
}

/// Parse each support binary executable format
fn parse_host_binary(bytes: Vec<u8>, display_path: &str) -> Value {
    if cfg!(not(any(
        target_os = "windows",
        target_os = "linux",
        target_os = "macos",
        target_family = "unix"
    ))) {
        return Value::Null;
    }

    let mut reader = AccessorReader::memory(
        bytes,
        crate::accessor::io::reader::ReaderLocation::from_display(display_path),
    );
    #[cfg(target_os = "windows")]
    {
        use crate::artifacts::os::windows::pe::parser::parse_pe_reader;

        parse_pe_reader(&mut reader)
            .ok()
            .and_then(|pe| serde_json::to_value(pe).ok())
            .unwrap_or_default()
    }

    #[cfg(target_os = "macos")]
    {
        use crate::artifacts::os::macos::macho::parser::parse_macho_reader;

        return parse_macho_reader(&mut reader)
            .ok()
            .and_then(|macho| serde_json::to_value(macho).ok())
            .unwrap_or_default();
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        use crate::artifacts::os::linux::executable::parser::parse_elf_reader;

        parse_elf_reader(&mut reader)
            .ok()
            .and_then(|elf| serde_json::to_value(elf).ok())
            .unwrap_or_default()
    }
}

/// There are a few directories we ignore for safety:
///
/// /proc on Unix systems. This is a memory only filesystem
///
/// Firmlinks on macOS
fn load_ignore_paths(exclude: &mut HashSet<String>) {
    #[cfg(target_family = "unix")]
    // Exclude /proc directory to skip memory only filesystem
    exclude.insert(String::from("/proc"));

    if !cfg!(target_os = "macos") {
        return;
    }
    let path = "/usr/share/firmlinks";
    let bytes = match HostFs::read_file(&InnerPath::new(PathBuf::from(path)), None) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not read '{path}' on macOS: {err:?}");
            return;
        }
    };

    for line in String::from_utf8_lossy(&bytes).lines() {
        if let Some(path) = line.split_whitespace().next()
            && !path.is_empty()
        {
            exclude.insert(path.to_string());
        }
    }
}

/// Track the `FileHostInfo` entry for output
fn push_row(listing: &mut HostListing<'_>, info: FileHostInfo) {
    listing.batch.push(info);

    if listing.batch.len() >= listing.max_list {
        host_output(take(&mut listing.batch), listing.manager, listing.options);
    }
}

/// Output batches of data for the live filelisting
fn host_output(entries: Vec<FileHostInfo>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize host filelisting: {err:?}");
            return;
        }
    };

    if let Err(err) =
        manager.write_artifact(files_output_name(&options.source), options, &mut records)
    {
        error!("Failed to output host filelisting: {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::walk_host;
    use crate::{
        accessor::{access::Accessor, error::AccessorError, source::factory::parse_inner_path},
        filesystem::files::hash_file_data,
        output::manager::OutputManager,
        structs::{
            artifacts::os::files::FileOptions,
            toml::{OutputConfig, OutputDestination, OutputFormat},
        },
    };
    use common::files::Hashes;
    use serde_json::Value;
    use std::{
        fs::{self, File, read_dir, read_to_string},
        io::Write,
        path::PathBuf,
    };

    fn setup(test_name: &str) -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("tmp");
        dir = dir.join(test_name);

        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(dir: &PathBuf, name: &str, contents: &[u8]) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        File::create(path).unwrap().write_all(contents).unwrap();
    }

    fn output_manager(name: &str) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            endpoint_id: String::from("test"),
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Jsonl,
            compress: false,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    fn listing_options(start: &PathBuf, depth: u32) -> FileOptions {
        FileOptions {
            start_path: start.display().to_string(),
            depth: Some(depth),
            source: String::from("host:"),
            ..Default::default()
        }
    }

    fn walk_rows(name: &str, options: &FileOptions) -> (OutputManager, Vec<Value>) {
        let output_dir = PathBuf::from("./tmp").join(name);
        let _ = fs::remove_dir_all(&output_dir);

        let mut manager = output_manager(name);
        let inner = parse_inner_path(&options.start_path).unwrap();
        walk_host(&inner, options, &mut manager, "", "host:").unwrap();

        let mut rows = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let filename = path.file_name().unwrap().to_string_lossy();
            if !filename.starts_with("files_host_") || !filename.ends_with(".jsonl") {
                continue;
            }

            let data = read_to_string(&path).unwrap();
            for line in data.lines() {
                rows.push(serde_json::from_str(line).unwrap());
            }
        }

        (manager, rows)
    }

    #[test]
    fn test_walk_host_depth_one() {
        let dir = setup("test_walk_host_depth_one");
        write_file(&dir, "readme.txt", b"root");
        write_file(&dir, "home/test.txt", b"nested");

        let options = listing_options(&dir, 1);
        let (manager, rows) = walk_rows("host_walk_depth_one", &options);

        assert_eq!(manager.artifact_runs[0].name, "files_host");
        assert!(rows.iter().any(|row| row["filename"] == "readme.txt"));
        assert!(
            rows.iter()
                .any(|row| row["filename"] == "home" && row["kind"] == "Directory")
        );

        assert!(rows.iter().all(|row| row["filename"] != "test.txt"));
        assert!(rows.iter().all(|row| row["depth"] == 1));
    }

    #[test]
    fn test_walk_host_depth_includes_child() {
        let dir = setup("test_walk_host_depth_includes_child");
        write_file(&dir, "home/test.txt", b"nested");
        write_file(&dir, "home/nested/other.txt", b"other");

        let options = listing_options(&dir, 2);
        let (_, rows) = walk_rows("host_walk_depth_two", &options);

        assert!(rows.iter().any(|row| row["filename"] == "test.txt"));
        assert!(rows.iter().any(|row| row["filename"] == "nested"));
        assert!(rows.iter().all(|row| row["filename"] != "other.txt"));
    }

    #[test]
    fn test_walk_host_filename_regex() {
        let dir = setup("test_walk_host_filename_regex");
        write_file(&dir, "readme.txt", b"root");
        write_file(&dir, "home/test.txt", b"nested");

        let mut options = listing_options(&dir, 2);
        options.filename_regex = Some(String::from(r"^readme\.txt$"));
        let (_, rows) = walk_rows("host_walk_regex", &options);

        assert!(rows.iter().any(|row| row["filename"] == "readme.txt"));
        assert!(rows.iter().all(|row| row["filename"] != "test.txt"));
    }

    #[test]
    fn test_walk_host_exclude_directory() {
        let dir = setup("test_walk_host_exclude_directory");
        write_file(&dir, "readme.txt", b"root");
        write_file(&dir, "home/test.txt", b"nested");

        let mut options = listing_options(&dir, 2);
        options.exclude_directories = Some(vec![dir.join("home").display().to_string()]);
        let (_, rows) = walk_rows("host_walk_exclude", &options);

        assert!(rows.iter().any(|row| row["filename"] == "readme.txt"));
        assert!(rows.iter().all(|row| row["filename"] != "home"));
        assert!(rows.iter().all(|row| row["filename"] != "test.txt"));
    }

    #[test]
    fn test_walk_host_hashes_match_file_bytes() {
        let dir = setup("test_walk_host_hashes_match_file_bytes");
        write_file(&dir, "hello.txt", b"hello world\n");

        let mut options = listing_options(&dir, 1);
        options.md5 = Some(true);
        options.sha1 = Some(true);
        options.sha256 = Some(true);
        let (_, rows) = walk_rows("host_walk_hash", &options);

        let hello = rows
            .iter()
            .find(|row| row["filename"] == "hello.txt")
            .expect("hello.txt");

        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };
        let (md5, sha1, sha256) = hash_file_data(&hashes, b"hello world\n");

        assert_eq!(hello["md5"], md5);
        assert_eq!(hello["sha1"], sha1);
        assert_eq!(hello["sha256"], sha256);
        assert_eq!(hello["kind"], "File");
    }

    #[test]
    fn test_walk_host_nested_start() {
        let dir = setup("test_walk_host_nested_start");
        write_file(&dir, "readme.txt", b"root");
        write_file(&dir, "home/test.txt", b"nested");
        write_file(&dir, "home/nested/other.txt", b"other");

        let options = listing_options(&dir.join("home"), 1);
        let (_, rows) = walk_rows("host_walk_nested_start", &options);

        assert!(rows.iter().any(|row| row["filename"] == "test.txt"));
        assert!(rows.iter().any(|row| row["filename"] == "nested"));
        assert!(rows.iter().all(|row| row["filename"] != "readme.txt"));
        assert!(rows.iter().all(|row| row["filename"] != "other.txt"));
    }

    #[test]
    fn test_walk_host_missing_start_is_error() {
        let dir = setup("test_walk_host_missing_start_is_error");
        let options = listing_options(&dir.join("no/such/dir"), 1);

        let mut manager = output_manager("host_walk_missing_start");
        let inner = parse_inner_path(&options.start_path).unwrap();
        let err = walk_host(&inner, &options, &mut manager, "", "host:").unwrap_err();

        assert!(matches!(err, AccessorError::NotADirectory { .. }));
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_walk_host_does_not_follow_symlink() {
        let dir = setup("test_walk_host_does_not_follow_symlink");
        write_file(&dir, "home/nested/other.txt", b"other");

        std::os::unix::fs::symlink(dir.join("home"), dir.join("linkdir")).unwrap();

        let options = listing_options(&dir, 3);
        let (_, rows) = walk_rows("host_walk_symlink", &options);

        let link = rows
            .iter()
            .find(|row| row["filename"] == "linkdir")
            .expect("linkdir");
        assert_eq!(link["kind"], "Symlink");
        assert!(rows.iter().any(|row| row["filename"] == "other.txt"));

        assert_eq!(
            rows.iter()
                .filter(|row| row["filename"] == "other.txt")
                .count(),
            1
        );
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_walk_host_skips_proc_from_root() {
        let options = listing_options(&PathBuf::from("/"), 1);
        let (_, rows) = walk_rows("host_walk_skip_proc", &options);

        assert!(rows.iter().all(|row| row["full_path"] != "/proc"));
        assert!(rows.iter().all(|row| row["filename"] != "proc"));
    }

    #[test]
    fn test_source_walk_host() {
        let dir = setup("test_source_walk_host");
        write_file(&dir, "readme.txt", b"root");

        let mut accessor = Accessor::with_defaults();
        let handle = accessor.open_source("host:").unwrap();

        let mut manager = output_manager("host_source_walk");
        let options = listing_options(&dir, 1);
        accessor
            .source_walk_host(&handle, &options, &mut manager, "")
            .unwrap();

        assert_eq!(manager.artifact_runs[0].name, "files_host");
        assert!(manager.artifact_runs[0].record_count >= 1);
    }
}
