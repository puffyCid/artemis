use common::files::{EntryKind, FileHostInfo, Hashes};
use regex::Regex;
use serde_json::Value;
use tracing::{error, info, warn};

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
use std::{collections::HashSet, mem::take, path::PathBuf};

/// Max size of file we read into memory if we need to parse binaries or scan with Yara
const YARA_MAX_SIZE: u64 = 50 * 1024 * 1024;

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
    load_firmlinks(&mut exclude);

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

    if let Err(err) = walk_host_dir(inner, &mut listing) {
        error!("Failed to complete full host filelisting: {err:?}");
    }

    if !listing.batch.is_empty() {
        host_output(take(&mut listing.batch), listing.manager, listing.options);
    }

    Ok(())
}

struct HostListing<'a> {
    options: &'a FileOptions,
    manager: &'a mut OutputManager,
    yara_rule: &'a str,
    evidence: &'a str,
    path_filter: Regex,
    file_filter: Regex,
    hashes: Hashes,
    exclude: &'a HashSet<String>,
    max_list: usize,
    max_depth: u32,
    depth: u32,
    batch: Vec<FileHostInfo>,
}

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

fn row_matches(listing: &HostListing<'_>, info: &FileHostInfo) -> bool {
    (listing.options.path_regex.is_none() || regex_check(&listing.path_filter, &info.full_path))
        && (listing.options.filename_regex.is_none()
            || regex_check(&listing.file_filter, &info.filename))
}

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
    } else {
        if let Some(handle) = entry.handle.as_file()
            && let Ok(mut reader) = HostFs::reader_handle(handle)
        {
            let (md5, sha1, sha256) = hash_reader(&listing.hashes, &mut reader);
            host_info.md5 = md5;
            host_info.sha1 = sha1;
            host_info.sha256 = sha256;
        }
    }

    if want_bin && !bytes.is_empty() {
        host_info.binary_info = parse_host_binary(bytes, &host_info.display_path);
    }

    Ok(true)
}

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

    #[cfg(any(
        target_os = "linux",
        target_os = "freebsd",
        target_os = "openbsd",
        target_os = "dragonfly",
        target_family = "unix"
    ))]
    {
        use crate::artifacts::os::linux::executable::parser::parse_elf_reader;

        parse_elf_reader(&mut reader)
            .ok()
            .and_then(|elf| serde_json::to_value(elf).ok())
            .unwrap_or_default()
    }
}

fn load_firmlinks(exclude: &mut HashSet<String>) {
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

fn push_row(listing: &mut HostListing<'_>, info: FileHostInfo) {
    listing.batch.push(info);

    if listing.batch.len() >= listing.max_list {
        host_output(take(&mut listing.batch), listing.manager, listing.options);
    }
}

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
