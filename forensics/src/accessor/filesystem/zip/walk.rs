use crate::{
    accessor::{
        error::{AccessorError, AccessorResult},
        filesystem::zip::zip_archive::{ZipEntryRecord, ZipFs},
        io::reader::{
            AccessorReader, ReaderLocation, directory_from_display, extension_from_filename,
        },
        location::path::InnerPath,
    },
    artifacts::os::{
        files::artifact::files_output_name, linux::executable::parser::parse_elf_reader,
        macos::macho::parser::parse_macho_reader, windows::pe::parser::parse_pe_reader,
    },
    filesystem::files::hash_file_data,
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::{artifacts::os::files::FileOptions, toml::OutputFormat},
    utils::regex_options::{create_regex, regex_check},
};
use common::files::{EntryKind, FilesZipInfo, Hashes};
use regex::Regex;
use serde_json::Value;
use std::{collections::HashSet, mem::take};
use tracing::{error, info, warn};

/// Max size of file we read into memory if we need to parse PE or scan with Yara
const YARA_MAX_SIZE: u64 = 50 * 1024 * 1024;

/// Walk the ZIP file and output results
pub(super) fn walk_zip(
    fs: &ZipFs,
    inner: &InnerPath,
    options: &FileOptions,
    manager: &mut OutputManager,
    yara_rule: &str,
    evidence: &str,
) -> AccessorResult<()> {
    let start = ZipFs::inner_to_prefix(inner);

    let exclude: HashSet<String> = options
        .exclude_directories
        .clone()
        .unwrap_or_default()
        .into_iter()
        .map(|path| ZipFs::normalize_zip_path(&path))
        .collect();

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

    let mut listing = ZipListing {
        fs,
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
        start: start.clone(),
        seen_dirs: HashSet::new(),
        batch: Vec::new(),
    };

    for record in &fs.index.entries {
        if record.path.is_empty() {
            continue;
        }

        let Some(relative) = relative_to_start(&record.path, &listing.start) else {
            continue;
        };

        emit_parent(&mut listing, relative);

        let depth = path_depth(relative);
        if depth == 0 || depth > listing.max_depth {
            continue;
        }

        if path_excluded(&record.path, listing.exclude) {
            continue;
        }

        if record.is_dir {
            maybe_emit_dir(&mut listing, &record.path, depth, Some(record));
            continue;
        }

        let mut info = fill_zip(
            listing.fs,
            &record.path,
            depth,
            listing.evidence,
            Some(record),
            EntryKind::File,
        );
        if !row_matches(&listing, &info) {
            continue;
        }

        let emit = match enrich_zip_file(listing.fs, record, &mut info, &listing) {
            Ok(keep) => keep,
            Err(err) => {
                warn!("Failed to read {}: {err:?}", info.display_path);
                listing.yara_rule.is_empty()
            }
        };

        if emit {
            push_row(&mut listing, info);
        }
    }

    if !listing.batch.is_empty() {
        zip_output(take(&mut listing.batch), listing.manager, listing.options);
    }

    Ok(())
}

/// Parameters for a ZIP filelisting
struct ZipListing<'a> {
    /// Our ZIP accessor
    fs: &'a ZipFs,
    /// Options for the filelisting
    options: &'a FileOptions,
    /// The `OutputManager` to send results to
    manager: &'a mut OutputManager,
    /// Yara rule to filter results
    yara_rule: &'a str,
    /// ZIP file source
    evidence: &'a str,
    /// Path filtering
    path_filter: Regex,
    /// File filtering
    file_filter: Regex,
    /// Hashes we should if hashing files
    hashes: Hashes,
    /// Paths we should exclude
    exclude: &'a HashSet<String>,
    /// Max batch size
    max_list: usize,
    /// Max paths we should descend
    max_depth: u32,
    /// Start path
    start: String,
    /// Directories we have descended
    seen_dirs: HashSet<String>,
    /// Filelisting batches we stream
    batch: Vec<FilesZipInfo>,
}

/// Check if our start path is at root of the ZIP source
fn relative_to_start<'a>(path: &'a str, start: &str) -> Option<&'a str> {
    if start.is_empty() {
        return Some(path);
    }

    if path == start {
        return None;
    }

    path.strip_prefix(start)
        .and_then(|rest| rest.strip_prefix('/'))
}

/// Determine current ZIP depth
fn path_depth(relative: &str) -> u32 {
    relative.split('/').filter(|part| !part.is_empty()).count() as u32
}

/// Ignore directories if user wants to exclude any
fn path_excluded(path: &str, exclude: &HashSet<String>) -> bool {
    exclude.iter().any(|raw| {
        if raw.is_empty() {
            return false;
        }

        let exclude_path = ZipFs::normalize_zip_path(raw);
        path == exclude_path || path.starts_with(&format!("{exclude_path}/")) || raw == path
    })
}

/// Track parent directories
fn emit_parent(listing: &mut ZipListing<'_>, relative: &str) {
    let parts: Vec<&str> = relative
        .split('/')
        .filter(|part| !part.is_empty())
        .collect();

    if parts.len() < 2 {
        return;
    }

    let mut value = String::new();

    // Loop through parents
    for (index, part) in parts.iter().enumerate() {
        if index + 1 == parts.len() {
            break;
        }

        if !value.is_empty() {
            value.push('/');
        }

        value.push_str(part);

        let depth = (index + 1) as u32;
        if depth > listing.max_depth {
            break;
        }

        let inner = if listing.start.is_empty() {
            value.clone()
        } else {
            format!("{}/{value}", listing.start)
        };

        let record = listing.fs.index.record_for_path(&inner);
        maybe_emit_dir(listing, &inner, depth, record);
    }
}

/// Try to output directory ZIP entries
fn maybe_emit_dir(
    listing: &mut ZipListing<'_>,
    path: &str,
    depth: u32,
    record: Option<&ZipEntryRecord>,
) {
    if !listing.seen_dirs.insert(path.to_string()) {
        return;
    }

    // Check first if we should exclude
    if path_excluded(path, listing.exclude) {
        return;
    }

    let info = fill_zip(
        listing.fs,
        path,
        depth,
        listing.evidence,
        record,
        EntryKind::Directory,
    );
    if !row_matches(listing, &info) {
        return;
    }

    push_row(listing, info);
}

/// Check if we should filter ZIP files by regex
fn row_matches(listing: &ZipListing<'_>, info: &FilesZipInfo) -> bool {
    (listing.options.path_regex.is_none() || regex_check(&listing.path_filter, &info.full_path))
        && (listing.options.filename_regex.is_none()
            || regex_check(&listing.file_filter, &info.filename))
}

/// Create a `FilesZipInfo` value
fn fill_zip(
    fs: &ZipFs,
    path: &str,
    depth: u32,
    evidence: &str,
    record: Option<&ZipEntryRecord>,
    kind: EntryKind,
) -> FilesZipInfo {
    let display_path = fs.display_entry_path(path);
    let filename = filename_from_inner(path);
    let full_path = display_path
        .strip_prefix("zip:")
        .unwrap_or(&display_path)
        .to_string();

    FilesZipInfo {
        full_path,
        directory: directory_from_display(&display_path),
        extension: extension_from_filename(&filename),
        filename,
        modified: record
            .and_then(|entry| entry.modified.clone())
            .unwrap_or(String::from("1970-01-01T00:00:00.000Z")),
        size: record.map_or(0, |entry| entry.size),
        compressed_size: record.map_or(0, |entry| entry.compressed_size),
        compression: record.map_or(String::new(), |entry| entry.compression.clone()),
        crc32: record.map_or(0, |entry| entry.crc32),
        encrypted: record.is_some_and(|entry| entry.encrypted),
        kind,
        depth: depth as usize,
        display_path,
        evidence: evidence.to_string(),
        ..Default::default()
    }
}

/// Return filename from a ZIP entry path
fn filename_from_inner(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

/// If binary parsing, Yara scanning
/// or hashing is enabled
///
/// Read each ZIP file entry
///
/// File reading only occurs for
/// files that are smaller
/// than `YARA_MAX_SIZE`
fn enrich_zip_file(
    fs: &ZipFs,
    record: &ZipEntryRecord,
    zip_info: &mut FilesZipInfo,
    listing: &ZipListing<'_>,
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

    if zip_info.size > YARA_MAX_SIZE {
        info!(
            "Skipping file {}. File size is {} vs 50MB max scans size",
            zip_info.display_path, zip_info.size
        );
        if want_yara {
            return Ok(false);
        }
        return Ok(true);
    }

    let bytes = match fs.read_entry_bytes(record.index) {
        Ok(result) => result,
        Err(err) => {
            warn!("Could not read file {}: {err:?}", zip_info.display_path);
            if want_yara {
                return Ok(false);
            }
            Vec::new()
        }
    };

    #[cfg(feature = "yarax")]
    if want_yara {
        use crate::utils::yara::scan_bytes;

        match scan_bytes(&bytes, listing.yara_rule) {
            Ok(hits) if !hits.is_empty() => zip_info.yara_hits = hits,
            Ok(_) => return Ok(false),
            Err(err) => {
                warn!("Failed to scan with yara: {err:?}");
                return Ok(false);
            }
        }
    }

    if want_hash && !bytes.is_empty() {
        let (md5, sha1, sha256) = hash_file_data(&listing.hashes, &bytes);
        zip_info.md5 = md5;
        zip_info.sha1 = sha1;
        zip_info.sha256 = sha256;
    }

    if want_bin && !bytes.is_empty() {
        zip_info.binary_info = parse_zip_binary(bytes, &zip_info.display_path);
    }
    Ok(true)
}

/// The ZIP accessor is unique
/// It supports all 3 binary types regardless of platform
fn parse_zip_binary(bytes: Vec<u8>, display_path: &str) -> Value {
    if bytes.len() < 4 {
        return Value::Null;
    }

    let magic_sig = bytes[..4].to_vec();
    let mut reader = AccessorReader::memory(bytes, ReaderLocation::from_display(display_path));

    let mz = [0x4d, 0x5a];
    if magic_sig.starts_with(&mz) {
        return parse_pe_reader(&mut reader)
            .ok()
            .and_then(|pe| serde_json::to_value(pe).ok())
            .unwrap_or_default();
    }

    let elf = [0x7f, 0x45, 0x4c, 0x46];
    if magic_sig == elf {
        return parse_elf_reader(&mut reader)
            .ok()
            .and_then(|elf| serde_json::to_value(elf).ok())
            .unwrap_or_default();
    }

    let fat = [0xCA, 0xFE, 0xBA, 0xBE];
    if magic_sig == fat || magic_sig.ends_with(&[0xFA, 0xED, 0xFE]) {
        return parse_macho_reader(&mut reader)
            .ok()
            .and_then(|macho| serde_json::to_value(macho).ok())
            .unwrap_or_default();
    }

    Value::Null
}

/// Track the `FilesZipInfo` batch entries
/// Once we hit the max limit we output our results
fn push_row(listing: &mut ZipListing<'_>, info: FilesZipInfo) {
    info!(
        "ZIP listing start '{}'. Current Directory: '{}'",
        listing.start, info.directory
    );

    listing.batch.push(info);

    if listing.batch.len() >= listing.max_list {
        zip_output(take(&mut listing.batch), listing.manager, listing.options);
    }
}

/// Write `FilesZipInfo` output based on te `OutputManager`
fn zip_output(entries: Vec<FilesZipInfo>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize zip filelisting: {err:?}");
            return;
        }
    };
    if let Err(err) =
        manager.write_artifact(files_output_name(&options.source), options, &mut records)
    {
        error!("Failed to output zip filelisting: {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::walk_zip;
    use crate::{
        accessor::{filesystem::zip::zip_archive::ZipFs, location::path::InnerPath},
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
    use zip::{ZipWriter, write::SimpleFileOptions};

    fn setup(test_name: &str) -> PathBuf {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("tmp");
        dir = dir.join(test_name);

        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_zip(path: &PathBuf, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut writer = ZipWriter::new(file);
        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
        for (name, contents) in entries {
            writer.start_file(*name, options).unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap();
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

    fn listing_options(archive: &PathBuf, depth: u32) -> FileOptions {
        FileOptions {
            start_path: String::new(),
            depth: Some(depth),
            source: format!("zip:{}", archive.display()),
            ..Default::default()
        }
    }

    fn walk_rows(
        name: &str,
        archive: &PathBuf,
        options: &FileOptions,
    ) -> (OutputManager, Vec<Value>) {
        let fs = ZipFs::new(archive.clone()).unwrap();
        let mut manager = output_manager(name);
        walk_zip(
            &fs,
            &InnerPath::empty(),
            options,
            &mut manager,
            "",
            &format!("zip:{}", archive.display()),
        )
        .unwrap();

        let output_dir = PathBuf::from("./tmp").join(name);
        let mut rows = Vec::new();

        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let filename = path.file_name().unwrap().to_string_lossy();
            if !filename.starts_with("files_zip_") || !filename.ends_with(".jsonl") {
                continue;
            }

            if filename.starts_with("artemis_") {
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
    fn test_walk_zip_root_synthesizes_dirs() {
        let dir = setup("test_walk_zip_root_synthesizes_dirs");
        let archive = dir.join("archive.zip");

        write_zip(
            &archive,
            &[
                ("home/test.txt", b"zip payload"),
                ("home/nested/other.txt", b"other"),
                ("readme.txt", b"root"),
            ],
        );

        let options = listing_options(&archive, 1);
        let (manager, rows) = walk_rows("zip_walk_root", &archive, &options);
        assert_eq!(manager.artifact_runs[0].name, "files_zip");

        assert!(rows.iter().any(|row| row["filename"] == "readme.txt"));
        assert!(
            rows.iter()
                .any(|row| row["filename"] == "home" && row["kind"] == "Directory")
        );
        assert!(rows.iter().all(|row| row["filename"] != "test.txt"));
        assert!(rows.iter().all(|row| row["filename"] != "other.txt"));
        assert!(rows.iter().all(|row| row["depth"] == 1));
    }

    #[test]
    fn test_walk_zip_depth_includes_child() {
        let dir = setup("test_walk_zip_depth_includes_child");
        let archive = dir.join("archive.zip");

        write_zip(
            &archive,
            &[
                ("home/test.txt", b"zip payload"),
                ("home/nested/other.txt", b"other"),
            ],
        );

        let options = listing_options(&archive, 2);
        let (_, rows) = walk_rows("zip_walk_depth", &archive, &options);
        let file = rows
            .iter()
            .find(|row| row["filename"] == "test.txt")
            .expect("test.txt");

        assert_eq!(file["kind"], "File");
        assert_eq!(file["size"], 11);
        assert_eq!(file["depth"], 2);
        assert!(rows.iter().any(|row| row["filename"] == "nested"));
        assert!(rows.iter().all(|row| row["filename"] != "other.txt"));
    }

    #[test]
    fn test_walk_zip_filename_regex() {
        let dir = setup("test_walk_zip_filename_regex");
        let archive = dir.join("archive.zip");
        write_zip(
            &archive,
            &[("home/test.txt", b"zip payload"), ("readme.txt", b"root")],
        );

        let mut options = listing_options(&archive, 2);
        options.filename_regex = Some(String::from(r"^readme\.txt$"));
        let (_, rows) = walk_rows("zip_walk_regex", &archive, &options);
        assert!(rows.iter().any(|row| row["filename"] == "readme.txt"));
        assert!(rows.iter().all(|row| row["filename"] != "test.txt"));
    }

    #[test]
    fn test_walk_zip_exclude_directory() {
        let dir = setup("test_walk_zip_exclude_directory");
        let archive = dir.join("archive.zip");

        write_zip(
            &archive,
            &[("home/test.txt", b"zip payload"), ("readme.txt", b"root")],
        );

        let mut options = listing_options(&archive, 2);
        options.exclude_directories = Some(vec![String::from("home")]);

        let (_, rows) = walk_rows("zip_walk_exclude", &archive, &options);
        assert!(rows.iter().any(|row| row["filename"] == "readme.txt"));
        assert!(rows.iter().all(|row| row["filename"] != "home"));
        assert!(rows.iter().all(|row| row["filename"] != "test.txt"));
    }

    #[test]
    fn test_walk_zip_hashes_match_file_bytes() {
        let dir = setup("test_walk_zip_hashes_match_file_bytes");
        let archive = dir.join("archive.zip");
        write_zip(&archive, &[("hello.txt", b"hello world\n")]);

        let mut options = listing_options(&archive, 1);
        options.md5 = Some(true);
        options.sha1 = Some(true);
        options.sha256 = Some(true);

        let (_, rows) = walk_rows("zip_walk_hash", &archive, &options);
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
        assert_eq!(hello["compression"], "Stored");
        assert_eq!(hello["kind"], "File");
    }
}
