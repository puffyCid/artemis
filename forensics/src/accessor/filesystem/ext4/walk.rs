use crate::{
    accessor::{
        entry::{
            handle::{DirEntry, FileHandle, ItemHandle},
            locator::{DirLocator, Ext4EntryRef, FileLocator},
        },
        error::{AccessorError, AccessorResult},
        filesystem::{
            ext4::{directory::read_symlink, format::inode::Inode, fs::Ext4Fs},
            helper::label::{LabeledPath, PathLabel},
        },
        io::reader::{
            AccessorReader, ReaderLocation, directory_from_display, extension_from_filename,
        },
        location::path::InnerPath,
    },
    artifacts::os::{
        files::artifact::files_output_name, linux::executable::parser::parse_elf_reader,
    },
    filesystem::files::{hash_file_data, hash_reader},
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::{artifacts::os::files::FileOptions, toml::OutputFormat},
    utils::{
        regex_options::{create_regex, regex_check},
        strings::extract_utf8_string_lossy,
    },
};
use common::files::{EntryKind, FileExt4Info, Hashes};
use regex::Regex;
use serde_json::Value;
use std::{
    collections::HashSet,
    io::{Read, Seek},
    mem::take,
};
use tracing::{error, info, warn};

/// Max size of a file read into memory for binary parsing or Yara
const YARA_MAX_SIZE: u64 = 50 * 1024 * 1024;

impl<R: Read + Seek + Send + 'static> Ext4Fs<R> {
    /// Walk the ext4 filesystem and output `FileExt4Info`
    pub(crate) fn walk(
        &self,
        inner: &InnerPath,
        options: &FileOptions,
        manager: &mut OutputManager,
        yara_rule: &str,
        evidence: &str,
    ) -> AccessorResult<()> {
        let label = self.volume().target_path().to_string();
        let paths = PathLabel::custom(move |filesystem_path| {
            let display_path = format!("ext4:{filesystem_path}");
            LabeledPath {
                full_path: filesystem_path.to_string(),
                directory: directory_from_display(&display_path),
                display_path: display_path.clone(),
                drive: label.clone(),
            }
        });

        self.walk_labeled(inner, options, manager, yara_rule, evidence, paths)
    }

    /// Walk the ext4 filesystem regardless if live system or disk image
    ///
    /// `PathLabel` includes path disk image and partition path
    pub(crate) fn walk_labeled(
        &self,
        inner: &InnerPath,
        options: &FileOptions,
        manager: &mut OutputManager,
        yara_rule: &str,
        evidence: &str,
        paths: PathLabel,
    ) -> AccessorResult<()> {
        walk_ext4(self, inner, options, manager, yara_rule, evidence, paths)
    }
}

/// Parameters for a ext4 filelisting
struct Ext4Listing<'a, R: Read + Seek + Send> {
    /// Opened ext4 filesystem
    fs: &'a Ext4Fs<R>,
    /// Filelisting options
    options: &'a FileOptions,
    /// Output manager for the filelisting
    manager: &'a mut OutputManager,
    /// Option Yara Rule to scan files
    yara_rule: &'a str,
    /// Source of the filelisting
    evidence: &'a str,
    /// Optional path filter
    path_filter: Regex,
    /// Optional filename filter
    file_filter: Regex,
    /// Hashing options for files
    hashes: Hashes,
    /// Directories to skip
    exclude: &'a HashSet<String>,
    /// How many entries to keep before writing output
    max_list: usize,
    /// How far the filelisting should descend
    max_depth: u32,
    /// How to represent the file paths for the filelisting
    paths: PathLabel,
    /// Current batch of entries for the filelisting
    batch: Vec<FileExt4Info>,
}

/// Start the filelisting at provide `InnerPath`
fn walk_ext4<R: Read + Seek + Send + 'static>(
    fs: &Ext4Fs<R>,
    inner: &InnerPath,
    options: &FileOptions,
    manager: &mut OutputManager,
    yara_rule: &str,
    evidence: &str,
    paths: PathLabel,
) -> AccessorResult<()> {
    let exclude: HashSet<String> = options
        .exclude_directories
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let path_filter = create_regex(options.path_regex.as_deref().unwrap_or(""))
        .map_err(|_err| AccessorError::location(&options.start_path, "invalid path_regex"))?;
    let file_filter = create_regex(options.filename_regex.as_deref().unwrap_or(""))
        .map_err(|_err| AccessorError::location(&options.start_path, "invalid filename_regex"))?;

    let max_list = if options.metadata || manager.config.format == OutputFormat::Timeline {
        1000
    } else {
        10000
    };

    let entries = fs.read_dir(inner)?;
    let mut listing = Ext4Listing {
        fs,
        options,
        manager,
        yara_rule,
        evidence,
        path_filter,
        file_filter,
        hashes: Hashes {
            md5: options.md5,
            sha1: options.sha1,
            sha256: options.sha256,
        },
        exclude: &exclude,
        max_list,
        max_depth: options.depth.unwrap_or(1),
        paths,
        batch: Vec::new(),
    };

    walk_entries(&mut listing, entries, 1)?;

    if !listing.batch.is_empty() {
        ext4_output(take(&mut listing.batch), listing.manager, listing.options);
    }

    Ok(())
}

/// Descend into directory
fn walk_entries<R: Read + Seek + Send + 'static>(
    listing: &mut Ext4Listing<'_, R>,
    entries: Vec<DirEntry>,
    depth: u32,
) -> AccessorResult<()> {
    for entry in entries {
        let labeled = listing.paths.render(&entry.meta.full_path);
        if listing.exclude.contains(&entry.meta.full_path)
            || listing.exclude.contains(&labeled.full_path)
        {
            continue;
        }

        let (label, reference) = entry_reference(&entry.handle)?;
        let inode = match listing.fs.open_inode(label, reference) {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not read inode for {}: {err:?}", entry.meta.full_path);
                continue;
            }
        };

        let mut info = fill_ext4(&entry, &inode, &labeled, depth, listing.evidence);
        if inode.kind() == EntryKind::Symlink {
            info.symlink = symlink_target(listing.fs, reference.inode, &inode, &info.full_path);
        }

        info.extended_attributes = match listing.fs.inode_attributes(reference.inode, &inode) {
            Ok(results) => results,
            Err(err) => {
                warn!(
                    "Could not read extended attributes for {}: {err:?}",
                    info.full_path
                );
                Vec::new()
            }
        };

        if row_matches(listing, &info) {
            let emit = if info.kind == EntryKind::File {
                match entry.handle.as_file() {
                    Some(handle) => {
                        match enrich_ext4_file(listing.fs, handle, &mut info, listing) {
                            Ok(keep) => keep,
                            Err(err) => {
                                warn!("Failed to read {}: {err:?}", info.display_path);
                                true
                            }
                        }
                    }
                    None => true,
                }
            } else {
                true
            };

            if emit {
                push_row(listing, info);
            }
        }

        if inode.kind() == EntryKind::Directory && depth < listing.max_depth {
            let Some(dir_handle) = entry.handle.as_directory() else {
                warn!(
                    "ext4 directory {} has no directory handle",
                    entry.meta.full_path
                );
                continue;
            };

            match listing.fs.read_dir_handle(dir_handle) {
                Ok(children) => walk_entries(listing, children, depth + 1)?,
                Err(err) => warn!("Could not descend into {}: {err:?}", entry.meta.full_path),
            }
        }
    }

    Ok(())
}

/// Return the volume label and inode reference for the `ItemHandle`
fn entry_reference(handle: &ItemHandle) -> AccessorResult<(&str, Ext4EntryRef)> {
    match handle {
        ItemHandle::File(handle) | ItemHandle::Unsupported(handle) => match &handle.locator {
            FileLocator::Ext4 { label, entry, .. } => Ok((label.as_str(), *entry)),
            _ => Err(AccessorError::invalid_handle(
                "ext4 walk expected an ext4 file handle",
            )),
        },
        ItemHandle::Directory(handle) => match &handle.locator {
            DirLocator::Ext4 { label, entry, .. } => Ok((label.as_str(), *entry)),
            _ => Err(AccessorError::invalid_handle(
                "ext4 walk expected an ext4 directory handle",
            )),
        },
    }
}

/// Assemble the `FileExt4Info` value
fn fill_ext4(
    entry: &DirEntry,
    inode: &Inode,
    labeled: &LabeledPath,
    depth: u32,
    evidence: &str,
) -> FileExt4Info {
    let (major, minor) = inode
        .device()
        .map_or((0, 0), |device| (device.major, device.minor));

    FileExt4Info {
        full_path: labeled.full_path.clone(),
        directory: labeled.directory.clone(),
        filename: entry.meta.filename.clone(),
        extension: extension_from_filename(&entry.meta.filename),
        created: entry.times.created.clone(),
        modified: entry.times.modified.clone(),
        changed: entry.times.changed.clone(),
        accessed: entry.times.accessed.clone(),
        uid: entry.meta.uid.to_string(),
        gid: entry.meta.gid.to_string(),
        inode: entry.meta.inode,
        generation: inode.generation,
        links: inode.hardlinks,
        mode: inode.mode,
        flags: inode.flags,
        major,
        minor,
        symlink: String::new(),
        attributes: entry.meta.attributes.clone(),
        size: entry.meta.size,
        kind: inode.kind(),
        depth: depth as usize,
        display_path: labeled.display_path.clone(),
        label: labeled.drive.clone(),
        evidence: evidence.to_string(),
        ..Default::default()
    }
}

/// Hash, scan, or parse a regular file
///
/// Returns false when Yara is enabled and the file is larger than 50MB, has no hits, or cannot be read.
fn enrich_ext4_file<R: Read + Seek + Send + 'static>(
    fs: &Ext4Fs<R>,
    handle: &FileHandle,
    file_info: &mut FileExt4Info,
    listing: &Ext4Listing<'_, R>,
) -> AccessorResult<bool> {
    let want_hash = listing.hashes.md5 || listing.hashes.sha1 || listing.hashes.sha256;
    let want_bin = listing.options.metadata;

    #[cfg(feature = "yarax")]
    let want_yara = !listing.yara_rule.is_empty();
    #[cfg(not(feature = "yarax"))]
    let want_yara = false;

    if !want_hash && !want_bin && !want_yara {
        return Ok(true);
    }

    if want_yara && file_info.size > YARA_MAX_SIZE {
        info!(
            "Skipping file {}. File size is {} vs 50MB max scans size",
            file_info.display_path, file_info.size
        );
        return Ok(false);
    }

    let need_bytes = want_yara || (want_bin && file_info.size <= YARA_MAX_SIZE);
    let mut bytes = Vec::new();

    if need_bytes {
        match fs.read_handle(handle, Some(YARA_MAX_SIZE)) {
            Ok(read) => bytes = read,
            Err(err) => {
                warn!("Could not read file {}: {err:?}", file_info.display_path);
                if want_yara {
                    return Ok(false);
                }
            }
        }
    }

    #[cfg(feature = "yarax")]
    if want_yara {
        use crate::utils::yara::scan_bytes;

        match scan_bytes(&bytes, listing.yara_rule) {
            Ok(hits) if !hits.is_empty() => file_info.yara_hits = hits,
            Ok(_) => return Ok(false),
            Err(err) => {
                warn!("Failed to scan with yara: {err:?}");
                return Ok(false);
            }
        }
    }

    if want_hash && !bytes.is_empty() {
        let (md5, sha1, sha256) = hash_file_data(&listing.hashes, &bytes);
        file_info.md5 = md5;
        file_info.sha1 = sha1;
        file_info.sha256 = sha256;
    } else if want_hash {
        match fs.reader_handle(handle) {
            Ok(mut reader) => {
                let (md5, sha1, sha256) = hash_reader(&listing.hashes, &mut reader);
                file_info.md5 = md5;
                file_info.sha1 = sha1;
                file_info.sha256 = sha256;
            }
            Err(err) => warn!("Could not hash {}: {err:?}", file_info.display_path),
        }
    }

    if want_bin && !bytes.is_empty() {
        file_info.binary_info = parse_ext4_binary(bytes, &file_info.display_path);
    }

    Ok(true)
}

/// Parse a ELF binary
fn parse_ext4_binary(bytes: Vec<u8>, display_path: &str) -> Value {
    if bytes.len() < 4 {
        return Value::Null;
    }

    let magic = [bytes[0], bytes[1], bytes[2], bytes[3]];
    let mut reader = AccessorReader::memory(bytes, ReaderLocation::from_display(display_path));

    let elf = [0x7f, 0x45, 0x4c, 0x46];
    if magic == elf {
        return parse_elf_reader(&mut reader)
            .ok()
            .and_then(|elf| serde_json::to_value(elf).ok())
            .unwrap_or_default();
    }

    Value::Null
}

/// Read a symlink target without following it
fn symlink_target<R: Read + Seek + Send + 'static>(
    fs: &Ext4Fs<R>,
    inode_number: u64,
    inode: &Inode,
    path: &str,
) -> String {
    match read_symlink(fs.volume(), inode_number, inode) {
        Ok(bytes) => extract_utf8_string_lossy(&bytes),
        Err(err) => {
            warn!("Could not read symlink target for {path}: {err:?}");
            String::new()
        }
    }
}

/// Keep a value when the path and filename filters match
fn row_matches<R: Read + Seek + Send>(listing: &Ext4Listing<'_, R>, info: &FileExt4Info) -> bool {
    (listing.options.path_regex.is_none() || regex_check(&listing.path_filter, &info.full_path))
        && (listing.options.filename_regex.is_none()
            || regex_check(&listing.file_filter, &info.filename))
}

/// Track the `FileExt4Info` value
fn push_row<R: Read + Seek + Send>(listing: &mut Ext4Listing<'_, R>, info: FileExt4Info) {
    listing.batch.push(info);

    // If we hit our limit, then output the results
    if listing.batch.len() >= listing.max_list {
        ext4_output(take(&mut listing.batch), listing.manager, listing.options);
    }
}

/// Output one batch of ext4 listing rows
fn ext4_output(entries: Vec<FileExt4Info>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize ext4 filelisting: {err:?}");
            return;
        }
    };

    if let Err(err) =
        manager.write_artifact(files_output_name(&options.source), options, &mut records)
    {
        error!("Failed to output ext4 filelisting: {err:?}");
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        accessor::{
            filesystem::{
                ext4::{fs::Ext4Fs, volume::Ext4Volume},
                helper::label::{LabeledPath, PathLabel},
            },
            location::path::InnerPath,
        },
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
        path::PathBuf,
    };

    fn image() -> Ext4Fs<File> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        let volume = Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap();
        Ext4Fs::new(volume)
    }

    fn inode_image() -> Ext4Volume<File> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap()
    }

    fn inner(path: &str) -> InnerPath {
        InnerPath::new(PathBuf::from(path))
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

    fn listing_options(depth: u32) -> FileOptions {
        FileOptions {
            start_path: String::from("/"),
            depth: Some(depth),
            source: String::from("ext4:test.img"),
            ..Default::default()
        }
    }

    fn walk_rows(
        name: &str,
        options: &FileOptions,
        paths: Option<PathLabel>,
    ) -> (OutputManager, Vec<Value>) {
        let output_dir = PathBuf::from("./tmp").join(name);
        let _ = fs::remove_dir_all(&output_dir);

        let fs = image();
        let mut manager = output_manager(name);
        let result = match paths {
            Some(paths) => {
                fs.walk_labeled(&inner("/"), options, &mut manager, "", "test.img", paths)
            }
            None => fs.walk(&inner("/"), options, &mut manager, "", "test.img"),
        };
        result.unwrap();

        let mut rows = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let filename = path.file_name().unwrap().to_string_lossy();
            if !filename.starts_with("files_") || !filename.ends_with(".jsonl") {
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
    fn test_walk_ext4_root() {
        let options = listing_options(1);
        let (manager, rows) = walk_rows("ext4_walk_root", &options, None);

        assert!(matches!(
            manager.artifact_runs[0].name.as_str(),
            "files" | "files_ext4"
        ));
        assert_eq!(manager.artifact_runs[0].record_count, 3);

        let afile = rows.iter().find(|row| row["filename"] == "afile").unwrap();
        let inode = inode_image().read_inode(18).unwrap();

        assert_eq!(afile["kind"], "File");
        assert_eq!(afile["size"], 13);
        assert_eq!(afile["inode"], 18);
        assert_eq!(afile["full_path"], "/afile");
        assert_eq!(afile["display_path"], "ext4:/afile");
        assert_eq!(afile["directory"], "/");

        assert_eq!(afile["label"], "test.img");
        assert_eq!(afile["evidence"], "test.img");
        assert_eq!(afile["symlink"], "");
        assert_eq!(afile["major"], 0);
        assert_eq!(afile["minor"], 0);
        assert_eq!(afile["depth"], 1);
        assert_eq!(
            afile["generation"].as_u64(),
            Some(u64::from(inode.generation))
        );
        assert_eq!(afile["links"].as_u64(), Some(u64::from(inode.hardlinks)));
        assert_eq!(afile["mode"].as_u64(), Some(u64::from(inode.mode)));
        assert_eq!(afile["flags"].as_u64(), Some(u64::from(inode.flags)));

        assert!(rows.iter().all(|row| row["filename"] != "hello.txt"));
        assert_eq!(afile["extended_attributes"][0]["name"], "security.selinux");
        assert_eq!(
            afile["extended_attributes"][0]["value"],
            "unconfined_u:object_r:unlabeled_t:s0"
        );
    }

    #[test]
    fn test_walk_ext4_depth_includes_hello_txt() {
        let options = listing_options(2);
        let (_, rows) = walk_rows("ext4_walk_depth", &options, None);
        let hello = rows
            .iter()
            .find(|row| row["filename"] == "hello.txt")
            .unwrap();

        assert_eq!(hello["size"], 13);
        assert_eq!(hello["inode"], 14);
        assert_eq!(hello["full_path"], "/test/hello.txt");
        assert_eq!(hello["display_path"], "ext4:/test/hello.txt");
        assert_eq!(hello["directory"], "/test");
        assert_eq!(hello["depth"], 2);
    }

    #[test]
    fn test_walk_ext4_filename_regex() {
        let mut options = listing_options(2);
        options.filename_regex = Some(String::from(r"^hello\.txt$"));
        let (_, rows) = walk_rows("ext4_walk_regex", &options, None);

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["filename"], "hello.txt");
    }

    #[test]
    fn test_walk_ext4_exclude_directory() {
        let mut options = listing_options(2);
        options.exclude_directories = Some(vec![String::from("/test")]);
        let (_, rows) = walk_rows("ext4_walk_exclude", &options, None);

        assert!(rows.iter().any(|row| row["filename"] == "afile"));
        assert!(rows.iter().all(|row| row["filename"] != "test"));
        assert!(rows.iter().all(|row| row["filename"] != "hello.txt"));
    }

    #[test]
    fn test_walk_ext4_hashes_match_afile() {
        let mut options = listing_options(1);
        options.md5 = true;
        options.sha1 = true;
        options.sha256 = true;

        let (_, rows) = walk_rows("ext4_walk_hash", &options, None);
        let afile = rows.iter().find(|row| row["filename"] == "afile").unwrap();
        let (md5, sha1, sha256) = hash_file_data(
            &Hashes {
                md5: true,
                sha1: true,
                sha256: true,
            },
            b"I like Rust!\n",
        );

        assert_eq!(afile["md5"], md5);
        assert_eq!(afile["sha1"], sha1);
        assert_eq!(afile["sha256"], sha256);
    }

    #[test]
    fn test_walk_labeled_replaces_the_display_path() {
        let options = listing_options(1);
        let paths = PathLabel::custom(|filesystem_path| LabeledPath {
            full_path: filesystem_path.to_string(),
            display_path: format!("disk:/image!{filesystem_path}"),
            directory: String::from("/"),
            drive: String::from("Partition0"),
        });
        let (_, rows) = walk_rows("ext4_walk_labeled", &options, Some(paths));
        let afile = rows.iter().find(|row| row["filename"] == "afile").unwrap();

        assert_eq!(afile["full_path"], "/afile");
        assert_eq!(afile["display_path"], "disk:/image!/afile");
        assert_eq!(afile["label"], "Partition0");
    }
}
