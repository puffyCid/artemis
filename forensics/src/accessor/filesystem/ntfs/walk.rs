use crate::{
    accessor::{
        entry::{
            handle::{DirEntry, DirHandle, EntryMeta, FileHandle, ItemHandle},
            locator::{DirLocator, FileLocator, NtfsEntryRef},
        },
        error::{AccessorError, AccessorResult},
        filesystem::{
            helper::attributes::windows_attributes,
            ntfs::{
                attributes::{list_ads_names, read_named_data},
                data::{
                    display_ntfs_path, inner_to_ntfs_path, merge_ntfs_times, ntfs_filename_times,
                    ntfs_standard_times,
                },
                volume::NtfsVolume,
                wof::{decompress_wof, is_wof_file},
            },
        },
        io::reader::{
            AccessorReader, ReaderLocation, directory_from_display, extension_from_filename,
        },
        location::path::InnerPath,
    },
    artifacts::os::{files::artifact::files_output_name, windows::pe::parser::parse_pe_reader},
    filesystem::files::hash_file_data,
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::{artifacts::os::files::FileOptions, toml::OutputFormat},
    utils::{
        regex_options::{create_regex, regex_check},
        time::filetime_to_iso,
    },
};
use base16ct::lower::encode_str;
use common::{
    files::{EntryKind, FileNtfsInfo, Hashes},
    windows::CompressionType,
};
use digest_io::IoWrapper;
use md5::{Digest, Md5};
use ntfs::{
    Ntfs, NtfsFile, NtfsIndexEntryFlags, NtfsReadSeek, attribute_value::NtfsAttributeValue,
    indexes::NtfsFileNameIndex, structured_values::NtfsFileNamespace,
};
use regex::Regex;
use sha1::Sha1;
use sha2::Sha256;
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek, copy},
    mem::take,
};
use tracing::{error, info, warn};

/// Max size of file we read into memory if we need to parse PE or scan with Yara
const YARA_MAX_SIZE: u64 = 50 * 1024 * 1024;

/// Walk the NTFS filesystem and output results
pub(crate) fn walk_ntfs<T: Read + Seek + Send>(
    volume: &NtfsVolume<T>,
    drive: char,
    inner: &InnerPath,
    options: &FileOptions,
    manager: &mut OutputManager,
    yara_rule: &str,
    evidence: &str,
) -> AccessorResult<()> {
    let (inner_path, _) = inner_to_ntfs_path(inner, drive);
    let parent_display = display_ntfs_path(drive, &inner_path);
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

    let max_list =
        if options.metadata.is_some_and(|b| b) || manager.config.format == OutputFormat::Timeline {
            1000
        } else {
            10000
        };

    let mut listing = NtfsListing {
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
        batch: Vec::new(),
        drive,
        depth: 1,
        max_depth: options.depth.unwrap_or(1),
    };

    volume.with_reader(|ntfs, reader| {
        let dir = resolve_directory(ntfs, reader, &inner_path)?;

        walk_ntfs_dir(
            ntfs,
            reader,
            &dir,
            &parent_display,
            volume.sids(),
            &mut listing,
        )?;

        if !listing.batch.is_empty() {
            ntfs_output(take(&mut listing.batch), listing.manager, listing.options);
        }

        Ok(())
    })
}

/// Parameters when walking the NTFS filesystem
struct NtfsListing<'a> {
    /// Options for the filelisting
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
    /// Will often be a drive letter
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
    /// Array to stream filelisting results
    batch: Vec<FileNtfsInfo>,
    /// Drive letter we are targeting
    drive: char,
    /// Current filelisting depth from the start path in `FileOptions`
    depth: u32,
    /// The max depth we are descending
    max_depth: u32,
}

/// Recursively walk the the NTFS filesystem
fn walk_ntfs_dir<R: Read + Seek + Send>(
    ntfs: &Ntfs,
    reader: &mut R,
    dir: &NtfsFile<'_>,
    parent_display: &str,
    sids: &HashMap<u32, (String, String)>,
    listing: &mut NtfsListing<'_>,
) -> AccessorResult<()> {
    let index = dir.directory_index(reader).map_err(ntfs_err)?;
    let mut iter = index.entries();

    while let Some(value) = iter.next(reader) {
        let entry = match value {
            Ok(entry) => entry,
            Err(err) => {
                warn!("Could not iterate index children: {err:?}");
                continue;
            }
        };

        if entry.flags().contains(NtfsIndexEntryFlags::LAST_ENTRY) {
            continue;
        }

        let key = match entry.key() {
            Some(Ok(key)) => key,
            Some(Err(err)) => return Err(ntfs_err(err)),
            None => continue,
        };

        if key.namespace() == NtfsFileNamespace::Dos {
            continue;
        }

        let name = key.name().to_string_lossy();
        // Skip current and parent directories
        if name == "." || name == ".." {
            continue;
        }

        let display_path = if parent_display.is_empty() {
            format!("{}:\\{name}", listing.drive)
        } else if parent_display.ends_with('\\') {
            format!("{parent_display}{name}")
        } else {
            format!("{parent_display}\\{name}",)
        };

        if listing.exclude.contains(&display_path) {
            continue;
        }

        let file = entry
            .file_reference()
            .to_file(ntfs, reader)
            .map_err(ntfs_err)?;

        let mut info =
            match fill_ntfs_entry(ntfs, reader, &file, &name, &display_path, sids, listing) {
                Ok(results) => results,
                Err(err) => {
                    warn!("Could not read NTFS metadata for {display_path}: {err:?}");
                    // Even if we fail to parse an entry
                    // We can still try to descend
                    if key.is_directory() && listing.depth < listing.max_depth {
                        listing.depth += 1;
                        if let Err(err) =
                            walk_ntfs_dir(ntfs, reader, &file, &display_path, sids, listing)
                        {
                            warn!("Could not descend into {display_path}: {err:?}");
                        }
                        listing.depth -= 1;
                    }
                    continue;
                }
            };

        if (listing.options.path_regex.is_none()
            || regex_check(&listing.path_filter, &info.full_path))
            && (listing.options.filename_regex.is_none()
                || regex_check(&listing.file_filter, &info.filename))
        {
            info.evidence = listing.evidence.to_string();

            let emit = if info.kind == EntryKind::File {
                match enrich_ntfs_file(reader, &file, &mut info, listing) {
                    Ok(keep) => keep,
                    Err(err) => {
                        warn!("Failed to read {display_path}: {err:?}");
                        listing.yara_rule.is_empty()
                    }
                }
            } else {
                true
            };

            if emit {
                listing.batch.push(info);
                if listing.batch.len() >= listing.max_list {
                    ntfs_output(take(&mut listing.batch), listing.manager, listing.options);
                }
            }
        }

        if key.is_directory() && listing.depth < listing.max_depth {
            listing.depth += 1;
            if let Err(err) = walk_ntfs_dir(ntfs, reader, &file, &display_path, sids, listing) {
                warn!("Could not descend into {display_path}: {err:?}");
            }
            listing.depth -= 1;
        }
    }

    Ok(())
}

/// Return `FileNtfsInfo` for each file
fn fill_ntfs_entry<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    file: &NtfsFile<'_>,
    name: &str,
    display_path: &str,
    sids: &HashMap<u32, (String, String)>,
    listing: &NtfsListing<'_>,
) -> AccessorResult<FileNtfsInfo> {
    let kind = if file.is_directory() {
        EntryKind::Directory
    } else {
        EntryKind::File
    };

    let standard = file.info().map_err(ntfs_err)?;
    let filename = ntfs_filename_times(file, reader)?;

    let sid = standard.security_id().unwrap_or(0);
    let (user_sid, group_sid) = match sids.get(&sid) {
        Some((user, group)) => (user.clone(), group.clone()),
        None => (String::new(), String::new()),
    };

    let mut size = 0;
    let mut compressed_size = 0;
    let mut compression_type = CompressionType::None;
    let mut ads_info = Vec::new();

    if kind == EntryKind::File {
        size = read_file_size(file, reader)?;
        ads_info = list_ads_names(ntfs, reader, file)?;
        if is_wof_file(reader, file)? {
            compression_type = CompressionType::WofCompressed;
            compressed_size = wof_compressed_size(reader, file)?;
        }
    }

    let scheme_path = format!("ntfs:{display_path}");
    let info = FileNtfsInfo {
        full_path: display_path.to_string(),
        directory: directory_from_display(&scheme_path),
        filename: name.to_string(),
        extension: extension_from_filename(name),
        created: filetime_to_iso(standard.creation_time().nt_timestamp()),
        modified: filetime_to_iso(standard.modification_time().nt_timestamp()),
        changed: filetime_to_iso(standard.mft_record_modification_time().nt_timestamp()),
        accessed: filetime_to_iso(standard.access_time().nt_timestamp()),
        filename_created: filename.created,
        filename_modified: filename.modified,
        filename_changed: filename.changed,
        filename_accessed: filename.accessed,
        attributes: windows_attributes(standard.file_attributes().bits()),
        size,
        kind,
        depth: listing.depth as usize,
        display_path: scheme_path,
        compressed_size,
        compression_type,
        inode: file.file_record_number(),
        sequence_number: file.sequence_number(),
        parent_sequence_number: filename.parent_sequence,
        parent_mft_reference: filename.parent_file_record,
        owner: standard.owner_id().unwrap_or_default(),
        namespace: filename.namespace,
        ads_info,
        usn: standard.usn().unwrap_or_default(),
        sid,
        user_sid,
        group_sid,
        drive: format!("{}:", listing.drive),
        ..Default::default()
    };

    Ok(info)
}

/// Check if we have compressed data
fn wof_compressed_size<R: Read + Seek>(reader: &mut R, file: &NtfsFile<'_>) -> AccessorResult<u64> {
    let Some(item) = file.data(reader, "WofCompressedData") else {
        return Ok(0);
    };

    Ok(item
        .map_err(ntfs_err)?
        .to_attribute()
        .map_err(ntfs_err)?
        .value_length())
}

fn enrich_ntfs_file<R: Read + Seek>(
    reader: &mut R,
    file: &NtfsFile<'_>,
    ntfs_info: &mut FileNtfsInfo,
    listing: &NtfsListing<'_>,
) -> AccessorResult<bool> {
    let want_hash = listing.hashes.md5 || listing.hashes.sha1 || listing.hashes.sha256;
    let want_pe = listing.options.metadata.is_some_and(|b| b);

    #[cfg(feature = "yarax")]
    let want_yara = !listing.yara_rule.is_empty();
    #[cfg(not(feature = "yarax"))]
    let want_yara = false;

    if !want_hash && !want_pe && !want_yara {
        return Ok(true);
    }

    if want_yara && ntfs_info.size > YARA_MAX_SIZE {
        info!(
            "Skipping file {}. File size is {} vs 50MB max scans size",
            ntfs_info.display_path, ntfs_info.size
        );
        return Ok(false);
    }

    let need_bytes = want_yara || (want_pe && ntfs_info.size < YARA_MAX_SIZE);
    let mut bytes = Vec::new();

    // File is small enough to be read into memory
    if need_bytes {
        bytes = match read_listing_bytes(reader, file, ntfs_info) {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not read file {}: {err:?}", ntfs_info.display_path);

                if want_yara {
                    return Ok(false);
                }
                Vec::new()
            }
        };
    }

    #[cfg(feature = "yarax")]
    if want_yara {
        use crate::utils::yara::scan_bytes;

        match scan_bytes(&bytes, listing.yara_rule) {
            Ok(hits) if !hits.is_empty() => ntfs_info.yara_hits = hits,
            Ok(_) => return Ok(false),
            Err(err) => {
                warn!("Failed to scan with yara: {err:?}");
                return Ok(false);
            }
        }
    }

    // If we read a small file for yara scanning. We can just hash those bytes immediately
    if want_hash && !bytes.is_empty() {
        let (md5, sha1, sha256) = hash_file_data(&listing.hashes, &bytes);
        ntfs_info.md5 = md5;
        ntfs_info.sha1 = sha1;
        ntfs_info.sha256 = sha256;
    }

    // For small files we read straight into memory
    if want_pe && ntfs_info.size < YARA_MAX_SIZE {
        // Bytes might be empty if yara scanning is not enabled
        // Since we yara to scan and filter first
        if bytes.is_empty() {
            bytes = match read_listing_bytes(reader, file, ntfs_info) {
                Ok(result) => result,
                Err(err) => {
                    warn!("Could not read file {}: {err:?}", ntfs_info.display_path);
                    Vec::new()
                }
            };
        }

        // Read and parse the small PE file
        if !bytes.is_empty() {
            let mut pe_reader = AccessorReader::memory(
                bytes,
                ReaderLocation::from_display(&ntfs_info.display_path),
            );
            ntfs_info.binary_info = parse_pe_reader(&mut pe_reader)
                .ok()
                .and_then(|pe| serde_json::to_value(pe).ok())
                .unwrap_or_default();
        }
    } else if want_hash && bytes.is_empty() {
        // If file is larger than `YARA_MAX_SIZE`
        // We stream and hash it
        let (md5, sha1, sha256) = hash_live_data(reader, file, ntfs_info, &listing.hashes)?;
        ntfs_info.md5 = md5;
        ntfs_info.sha1 = sha1;
        ntfs_info.sha256 = sha256;
    }

    Ok(true)
}

/// Read the $DATA attribute if smaller the `YARA_MAX_SIZE`
///
/// Will decompress WOF data if required
fn read_listing_bytes<R: Read + Seek>(
    reader: &mut R,
    file: &NtfsFile<'_>,
    info: &FileNtfsInfo,
) -> AccessorResult<Vec<u8>> {
    if info.compression_type == CompressionType::WofCompressed {
        return decompress_wof(reader, file);
    }

    match read_named_data(reader, file, "") {
        Ok(bytes) => Ok(bytes),
        Err(_) => Ok(Vec::new()),
    }
}

/// If we want to hash large files
///
/// We cannot read into memory so we stream and hash in chunks
fn hash_live_data<R: Read + Seek>(
    reader: &mut R,
    file: &NtfsFile<'_>,
    info: &FileNtfsInfo,
    hashes: &Hashes,
) -> AccessorResult<(String, String, String)> {
    if info.compression_type == CompressionType::WofCompressed {
        let bytes = decompress_wof(reader, file)?;
        return Ok(hash_file_data(hashes, &bytes));
    }

    let Some(item) = file.data(reader, "") else {
        return Ok((String::new(), String::new(), String::new()));
    };

    let attr_item = item.map_err(ntfs_err)?;
    let attr = attr_item.to_attribute().map_err(ntfs_err)?;
    let mut value = attr.value(reader).map_err(ntfs_err)?;
    Ok(hash_attribute_value(&mut value, reader, hashes))
}

/// Hash `$DATA` in 64KiB chunks from the already-locked volume reader
fn hash_attribute_value<R: Read + Seek>(
    data_attr_value: &mut NtfsAttributeValue<'_, '_>,
    reader: &mut R,
    hashes: &Hashes,
) -> (String, String, String) {
    let mut md5 = IoWrapper(Md5::new());
    let mut sha1 = IoWrapper(Sha1::new());
    let mut sha256 = IoWrapper(Sha256::new());
    let temp_buff_size = 65536;
    let mut temp_buff: Vec<u8> = vec![0u8; temp_buff_size];

    loop {
        let bytes_result = data_attr_value.read(reader, &mut temp_buff);
        let bytes = match bytes_result {
            Ok(0) => break,
            Ok(result) => result,
            Err(err) => {
                error!("Failed to read data for hashing: {err:?}");
                break;
            }
        };

        if hashes.md5 {
            let _ = copy(&mut &temp_buff[..bytes], &mut md5);
        }

        if hashes.sha1 {
            let _ = copy(&mut &temp_buff[..bytes], &mut sha1);
        }

        if hashes.sha256 {
            let _ = copy(&mut &temp_buff[..bytes], &mut sha256);
        }
    }

    let mut md5_string = String::new();
    let mut sha1_string = String::new();
    let mut sha256_string = String::new();

    if hashes.md5 {
        let hash = md5.0.finalize();
        let mut buf = [0u8; 32];
        md5_string = encode_str(&hash, &mut buf).unwrap_or_default().to_string();
    }

    if hashes.sha1 {
        let hash = sha1.0.finalize();
        let mut buf = [0u8; 40];
        sha1_string = encode_str(&hash, &mut buf).unwrap_or_default().to_string();
    }

    if hashes.sha256 {
        let hash = sha256.0.finalize();
        let mut buf = [0u8; 64];
        sha256_string = encode_str(&hash, &mut buf).unwrap_or_default().to_string();
    }

    (md5_string, sha1_string, sha256_string)
}

/// Output batches of data for the NTFS filelisting
fn ntfs_output(entries: Vec<FileNtfsInfo>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize NTFS filelisting: {err:?}");
            return;
        }
    };
    if let Err(err) =
        manager.write_artifact(files_output_name(&options.source), options, &mut records)
    {
        error!("Failed to output NTFS filelisting: {err:?}");
    }
}

/// List files and directories from provided path
///
/// `display` is the human readable directory path. `inner_path` is the directory that that we should target for listing files and directories
pub(crate) fn list_children<T: Read + Seek + Send>(
    volume: &NtfsVolume<T>,
    drive: char,
    display: &str,
    inner_path: &str,
) -> AccessorResult<Vec<DirEntry>> {
    volume.with_reader(|ntfs, reader| {
        // Make sure the directory we are reading does not end with slash
        let parent_display = normalize_display_path(display);
        let dir_file = resolve_directory(ntfs, reader, inner_path)?;

        list_index_children(ntfs, reader, &dir_file, drive, &parent_display)
    })
}

/// List files and directories from provided directory file reference
pub(crate) fn list_children_handle<T: Read + Seek + Send>(
    volume: &NtfsVolume<T>,
    file_ref: &NtfsEntryRef,
    display: &str,
    drive: char,
) -> AccessorResult<Vec<DirEntry>> {
    volume.with_reader(|ntfs, reader| {
        // Make sure the directory we are reading does not end with slash
        let parent_display = normalize_display_path(display);
        let dir_file = open_by_ref(ntfs, reader, file_ref)?;

        list_index_children(ntfs, reader, &dir_file, drive, &parent_display)
    })
}

/// Extract entries from INDX attribute
fn list_index_children<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    dir_file: &NtfsFile<'_>,
    drive: char,
    parent_display: &str,
) -> AccessorResult<Vec<DirEntry>> {
    let index = dir_file.directory_index(reader).map_err(ntfs_err)?;
    let mut iter = index.entries();
    let mut entries = Vec::new();

    while let Some(value) = iter.next(reader) {
        let entry = match value {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not iterate index children: {err:?}");
                continue;
            }
        };
        if entry.flags().contains(NtfsIndexEntryFlags::LAST_ENTRY) {
            continue;
        }

        let key = match entry.key() {
            Some(Ok(key)) => key,
            Some(Err(err)) => return Err(ntfs_err(err)),
            None => continue,
        };

        if key.namespace() == NtfsFileNamespace::Dos {
            continue;
        }

        let name = key.name().to_string_lossy();
        if name == "." || name == ".." {
            continue;
        }

        let kind = if key.is_directory() {
            EntryKind::Directory
        } else {
            EntryKind::File
        };

        let file_ref = NtfsEntryRef::from_reference(entry.file_reference());
        let display_path = if parent_display.is_empty() {
            format!("{drive}:\\{name}")
        } else {
            format!("{parent_display}\\{name}")
        };

        let file = entry
            .file_reference()
            .to_file(ntfs, reader)
            .map_err(ntfs_err)?;

        let times = merge_ntfs_times(
            ntfs_standard_times(&file)?,
            ntfs_filename_times(&file, reader)?,
        );

        let size = if kind == EntryKind::File {
            read_file_size(&file, reader)?
        } else {
            0
        };

        let scheme_path = format!("ntfs:{display_path}");
        let handle = match &kind {
            EntryKind::Directory => ItemHandle::Directory(DirHandle::new(DirLocator::Ntfs {
                drive,
                dir_ref: file_ref,
                display_path,
            })),
            EntryKind::File => ItemHandle::File(FileHandle::new(FileLocator::Ntfs {
                drive,
                file_ref,
                display_path,
            })),
            _ => continue,
        };

        let meta = EntryMeta::new(kind, size, scheme_path);

        entries.push(DirEntry::new(name, handle, meta, times));
    }

    Ok(entries)
}

/// Return a `NtfsFile` from provided file path
pub(crate) fn resolve_file<'a, R: Read + Seek>(
    ntfs: &'a Ntfs,
    reader: &mut R,
    inner_path: &str,
) -> AccessorResult<NtfsFile<'a>> {
    let entry = resolve_entry(ntfs, reader, inner_path)?;
    if entry.is_directory() {
        return Err(AccessorError::not_a_file(inner_path));
    }

    Ok(entry)
}

/// Return a `NtfsFile` for a file or directory
pub(crate) fn resolve_entry<'a, R: Read + Seek>(
    ntfs: &'a Ntfs,
    reader: &mut R,
    inner_path: &str,
) -> AccessorResult<NtfsFile<'a>> {
    let components = split_inner_path(inner_path);
    let mut current = ntfs.root_directory(reader).map_err(ntfs_err)?;

    for (component_index, component) in components.iter().enumerate() {
        let index = current.directory_index(reader).map_err(ntfs_err)?;
        let mut finder = index.finder();
        let entry = match NtfsFileNameIndex::find(&mut finder, ntfs, reader, component) {
            Some(Ok(entry)) => entry,
            Some(Err(err)) => return Err(ntfs_err(err)),
            None => {
                error!(
                    "Failed to find '{}' from '{inner_path}'",
                    components[..component_index].join("\\")
                );
                return Err(AccessorError::NotFound {
                    path: components[..component_index].join("\\"),
                });
            }
        };

        current = entry.to_file(ntfs, reader).map_err(ntfs_err)?;
    }

    Ok(current)
}

/// Returns a `NtfsFile` by its file reference
pub(crate) fn open_by_ref<'a, R: Read + Seek>(
    ntfs: &'a ntfs::Ntfs,
    reader: &mut R,
    file_ref: &NtfsEntryRef,
) -> AccessorResult<NtfsFile<'a>> {
    ntfs.file(reader, file_ref.file_record_number)
        .map_err(ntfs_err)
}

/// Return the `NtfsFile` object associated with a target directory we want to read
fn resolve_directory<'n, R: Read + Seek>(
    ntfs: &'n Ntfs,
    reader: &mut R,
    inner_path: &str,
) -> AccessorResult<NtfsFile<'n>> {
    // Components of the directory to walk to
    let components = split_inner_path(inner_path);
    let mut current = ntfs.root_directory(reader).map_err(ntfs_err)?;

    // Loop through components of the directory we want to read
    for component in components {
        let index = current.directory_index(reader).map_err(ntfs_err)?;
        let mut finder = index.finder();

        // Descend into next directory component
        let entry = match NtfsFileNameIndex::find(&mut finder, ntfs, reader, &component) {
            Some(Ok(entry)) => entry,
            Some(Err(err)) => return Err(ntfs_err(err)),
            None => {
                return Err(AccessorError::NotFound { path: component });
            }
        };

        // Continue until we arrive at final directory component
        current = entry.to_file(ntfs, reader).map_err(ntfs_err)?;
    }

    if !current.is_directory() {
        return Err(AccessorError::NotADirectory {
            path: inner_path.to_string(),
        });
    }

    Ok(current)
}

/// Return the size of a file
pub(crate) fn get_file_size<T: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut T,
    record_number: u64,
) -> AccessorResult<u64> {
    // Get direct access to the file via file reference
    let file = ntfs.file(reader, record_number).map_err(ntfs_err)?;
    read_file_size(&file, reader)
}

fn read_file_size<T: Read + Seek>(file: &NtfsFile<'_>, reader: &mut T) -> AccessorResult<u64> {
    match file.data(reader, "") {
        Some(Ok(item)) => Ok(item.to_attribute().map_err(ntfs_err)?.value_length()),
        Some(Err(err)) => Err(ntfs_err(err)),
        None => Ok(0),
    }
}

/// Split the target directory we want to read into array of strings
pub(crate) fn split_inner_path(inner_path: &str) -> Vec<String> {
    inner_path
        .trim_matches(['\\', '/'])
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// Remove any slashes at end of directory we want to read
fn normalize_display_path(path: &str) -> String {
    path.trim_end_matches(['\\', '/']).to_string()
}

/// Handle `NTFSError` to `AccessorError`
pub(crate) fn ntfs_err(err: ntfs::NtfsError) -> AccessorError {
    AccessorError::Ntfs {
        path: None,
        reason: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::walk_ntfs;
    use crate::{
        accessor::{
            filesystem::ntfs::{volume::NtfsVolume, walk::list_children},
            location::path::InnerPath,
        },
        filesystem::files::hash_file_data,
        output::manager::OutputManager,
        structs::{
            artifacts::os::files::FileOptions,
            toml::{OutputConfig, OutputDestination, OutputFormat},
        },
    };
    use common::files::{EntryKind, Hashes};
    use serde_json::Value;
    use std::{
        fs::{read_dir, read_to_string},
        path::PathBuf,
    };

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        path
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
            start_path: String::new(),
            depth: Some(depth),
            source: String::from("ntfs:C:"),
            ..Default::default()
        }
    }

    fn walk_test_image(name: &str, options: &FileOptions) -> (OutputManager, Vec<Value>) {
        let volume = NtfsVolume::open_image(test_image()).unwrap();
        let inner = InnerPath::empty();
        let mut manager = output_manager(name);
        walk_ntfs(&volume, 'C', &inner, options, &mut manager, "", "ntfs:C:").unwrap();

        let output_dir = PathBuf::from("./tmp").join(name);
        let mut rows = Vec::new();

        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let filename = path.file_name().unwrap().to_string_lossy();
            if !filename.starts_with("files_ntfs_") || !filename.ends_with(".jsonl") {
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
    fn test_ntfs_volume() {
        let volume = NtfsVolume::open_image(test_image()).unwrap();
        let result = list_children(&volume, 'C', &"", &"").unwrap();
        assert_eq!(result.len(), 15);

        let main = result
            .iter()
            .find(|entry| entry.name == "main.ts")
            .expect("main.ts");

        assert_eq!(main.meta.kind, EntryKind::File);
        assert_eq!(main.meta.size, 514);
        assert!(main.times.created.is_some());
        assert!(main.times.modified.is_some());
        assert!(main.times.accessed.is_some());
        assert!(main.times.changed.is_some());

        assert!(main.times.filename_created.is_some());
        assert!(main.times.filename_modified.is_some());
        assert!(main.times.filename_accessed.is_some());
        assert!(main.times.filename_changed.is_some());

        let hello_dir = result
            .iter()
            .find(|entry| entry.name == "hello")
            .expect("hello");

        assert_eq!(hello_dir.meta.kind, EntryKind::Directory);
        assert_eq!(hello_dir.meta.size, 0);

        let result = list_children(&volume, 'c', &"C:\\hello", &"hello").unwrap();
        let hello = result
            .iter()
            .find(|entry| entry.name == "hello world.txt")
            .expect("hello world.txt");
        assert_eq!(hello.meta.size, 12);
    }

    #[test]
    fn test_walk_ntfs_root() {
        let options = listing_options(1);
        let (manager, rows) = walk_test_image("ntfs_walk_root", &options);
        assert!(!rows.is_empty());

        assert_eq!(manager.artifact_runs[0].name, "files_ntfs");
        assert_eq!(manager.artifact_runs[0].status, "completed");
        assert_eq!(manager.artifact_runs[0].record_count, 15);

        let main = rows
            .iter()
            .find(|row| row["filename"] == "main.ts")
            .expect("main.ts");

        assert_eq!(main["kind"], "File");
        assert_eq!(main["size"], 514);
        assert_eq!(main["full_path"], "C:\\main.ts");

        assert_eq!(main["evidence"], "ntfs:C:");
        assert_eq!(main["collection_metadata"]["artifact_name"], "files_ntfs");
        assert_ne!(main["sequence_number"], Value::from(0));
        assert!(main["sequence_number"] != main["parent_sequence_number"]);

        let hello = rows
            .iter()
            .find(|row| row["filename"] == "hello")
            .expect("hello");
        assert_eq!(hello["kind"], "Directory");
        assert!(rows.iter().all(|row| row["filename"] != "hello world.txt"));
    }

    #[test]
    fn test_walk_ntfs_depth_includes_child() {
        let options = listing_options(2);
        let (_, rows) = walk_test_image("ntfs_walk_depth", &options);
        let hello = rows
            .iter()
            .find(|row| row["filename"] == "hello world.txt")
            .expect("hello world.txt");

        assert_eq!(hello["size"], 12);
        assert_eq!(hello["full_path"], "C:\\hello\\hello world.txt");
        assert_eq!(hello["depth"], 2);
    }

    #[test]
    fn test_walk_ntfs_filename_regex() {
        let mut options = listing_options(2);
        options.filename_regex = Some(String::from(r"^main\.ts$"));
        let (_, rows) = walk_test_image("ntfs_walk_regex", &options);

        assert!(rows.iter().any(|row| row["filename"] == "main.ts"));
        assert!(rows.iter().all(|row| row["filename"] != "hello world.txt"));
    }

    #[test]
    fn test_walk_ntfs_exclude_directory() {
        let mut options = listing_options(2);
        options.exclude_directories = Some(vec![String::from("C:\\hello")]);
        let (_, rows) = walk_test_image("ntfs_walk_exclude", &options);

        assert!(rows.iter().any(|row| row["filename"] == "main.ts"));
        assert!(rows.iter().all(|row| row["filename"] != "hello"));
        assert!(rows.iter().all(|row| row["filename"] != "hello world.txt"));
    }

    #[test]
    fn test_walk_ntfs_hashes_match_file_bytes() {
        let mut options = listing_options(2);
        options.md5 = Some(true);
        options.sha1 = Some(true);
        options.sha256 = Some(true);

        let (_, rows) = walk_test_image("ntfs_walk_hash", &options);
        let hello = rows
            .iter()
            .find(|row| row["filename"] == "hello world.txt")
            .expect("hello world.txt");

        let hashes = Hashes {
            md5: true,
            sha1: true,
            sha256: true,
        };

        let (md5, sha1, sha256) = hash_file_data(&hashes, b"hello world\n");

        assert_eq!(hello["md5"], md5);
        assert_eq!(hello["sha1"], sha1);
        assert_eq!(hello["sha256"], sha256);
        assert_ne!(hello["sha1"], hello["md5"]);
        assert_ne!(hello["sha256"], hello["md5"]);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_walk_live_ntfs() {
        let options = listing_options(1);
        let drive = 'C';
        let inner = InnerPath::empty();
        let mut manager = output_manager("live_ntfs");

        let volume = NtfsVolume::open_live_drive(drive).unwrap();
        walk_ntfs(&volume, drive, &inner, &options, &mut manager, "", "ntfs:c").unwrap();
    }
}
