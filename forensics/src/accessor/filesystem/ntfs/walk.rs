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
                attributes::list_ads_names,
                data::{
                    display_ntfs_path, inner_to_ntfs_path, merge_ntfs_times, ntfs_filename_times,
                    ntfs_standard_times,
                },
                volume::NtfsVolume,
                wof::is_wof_file,
            },
        },
        io::reader::{directory_from_display, extension_from_filename},
        location::path::InnerPath,
    },
    utils::time::filetime_to_iso,
};
use common::{
    files::{EntryKind, FileNtfsInfo},
    windows::CompressionType,
};
use ntfs::{
    Ntfs, NtfsFile, NtfsIndexEntryFlags, indexes::NtfsFileNameIndex,
    structured_values::NtfsFileNamespace,
};
use std::{
    collections::{HashMap, HashSet},
    io::{Read, Seek},
};
use tracing::{error, warn};

/// File entry data we return when walking the
/// NTFS filesystem
pub(crate) struct NtfsWalkEntry {
    /// All metadata associated with a NTFS entry
    pub(crate) info: FileNtfsInfo,
    /// `ItemHandle` for an entry
    pub(crate) handle: ItemHandle,
}

/// Walk the NTFS fielsystem and return `NtfsWalkEntry`
pub(crate) fn walk_ntfs<T: Read + Seek + Send>(
    volume: &NtfsVolume<T>,
    drive: char,
    inner: &InnerPath,
    max_depth: u32,
    exclude: &HashSet<String>,
    visit: &mut dyn FnMut(NtfsWalkEntry) -> AccessorResult<()>,
) -> AccessorResult<()> {
    let (inner_path, _) = inner_to_ntfs_path(inner, drive);
    let display = display_ntfs_path(drive, &inner_path);

    volume.with_reader(|ntfs, reader| {
        let dir = resolve_directory(ntfs, reader, &inner_path)?;
        let mut walk_values = NtfsWalkDir {
            depth: 1,
            max_depth,
            drive,
        };

        walk_ntfs_dir(
            ntfs,
            reader,
            &dir,
            &display,
            exclude,
            volume.sids(),
            visit,
            &mut walk_values,
        )
    })
}

/// Parameters when walking the NTFS filesystem
struct NtfsWalkDir {
    /// Current depth
    depth: u32,
    /// Max depth we descend
    max_depth: u32,
    /// Drive we are walking
    drive: char,
}

/// Recursively walk the the NTFS filesystem
fn walk_ntfs_dir<R: Read + Seek + Send>(
    ntfs: &Ntfs,
    reader: &mut R,
    dir: &NtfsFile<'_>,
    parent_display: &str,
    exclude: &HashSet<String>,
    sids: &HashMap<u32, (String, String)>,
    visit: &mut dyn FnMut(NtfsWalkEntry) -> AccessorResult<()>,
    ntfs_walk_dir: &mut NtfsWalkDir,
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

        let display_path = if parent_display.is_empty() || parent_display.ends_with('\\') {
            if parent_display.is_empty() {
                format!("{}:\\{name}", ntfs_walk_dir.drive)
            } else {
                format!("{parent_display}{name}")
            }
        } else {
            format!("{parent_display}\\{name}")
        };

        if exclude.contains(&display_path) {
            continue;
        }

        let file = entry
            .file_reference()
            .to_file(ntfs, reader)
            .map_err(ntfs_err)?;
        let file_ref = NtfsEntryRef::from_reference(entry.file_reference());

        let walk_entry = fill_ntfs_entry(
            ntfs,
            reader,
            &file,
            file_ref,
            &name,
            &display_path,
            sids,
            ntfs_walk_dir,
        )?;

        visit(walk_entry)?;

        if key.is_directory() && ntfs_walk_dir.depth < ntfs_walk_dir.max_depth {
            ntfs_walk_dir.depth += 1;
            walk_ntfs_dir(
                ntfs,
                reader,
                &file,
                &display_path,
                exclude,
                sids,
                visit,
                ntfs_walk_dir,
            )?;
            ntfs_walk_dir.depth -= 1;
        }
    }

    Ok(())
}

/// Return `NtfsWalkEntry` for each file
fn fill_ntfs_entry<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    file: &NtfsFile<'_>,
    file_ref: NtfsEntryRef,
    name: &str,
    display_path: &str,
    sids: &HashMap<u32, (String, String)>,
    ntfs_walk_dir: &NtfsWalkDir,
) -> AccessorResult<NtfsWalkEntry> {
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
        depth: ntfs_walk_dir.depth as usize,
        display_path: scheme_path,
        compressed_size,
        compression_type,
        inode: file.file_record_number(),
        sequence_number: filename.parent_sequence,
        parent_mft_reference: filename.parent_file_record,
        owner: standard.owner_id().unwrap_or_default(),
        namespace: filename.namespace,
        ads_info,
        usn: standard.usn().unwrap_or_default(),
        sid,
        user_sid,
        group_sid,
        drive: format!("{}:", ntfs_walk_dir.drive),
        ..Default::default()
    };

    let handle = if info.kind == EntryKind::Directory {
        ItemHandle::Directory(DirHandle {
            locator: DirLocator::Ntfs {
                drive: ntfs_walk_dir.drive,
                dir_ref: file_ref,
                display_path: display_path.to_string(),
            },
        })
    } else {
        ItemHandle::File(FileHandle::new(FileLocator::Ntfs {
            drive: ntfs_walk_dir.drive,
            file_ref,
            display_path: display_path.to_string(),
        }))
    };

    Ok(NtfsWalkEntry { info, handle })
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
    use crate::accessor::filesystem::ntfs::{volume::NtfsVolume, walk::list_children};
    use common::files::EntryKind;
    use std::path::PathBuf;

    #[test]
    fn test_ntfs_volume() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/filesystems/ntfs/test.raw");
        let volume = NtfsVolume::open_image(test_location).unwrap();
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
}
