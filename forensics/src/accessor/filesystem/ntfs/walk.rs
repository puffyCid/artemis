use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, EntryKind, EntryMeta, FileHandle, ItemHandle},
        locator::{DirLocator, FileLocator, NtfsEntryRef},
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::volume::NtfsVolume,
};
use ntfs::{
    Ntfs, NtfsFile, NtfsIndexEntryFlags, indexes::NtfsFileNameIndex,
    structured_values::NtfsFileNamespace,
};
use std::io::{Read, Seek};
use tracing::error;

/// Files and directories associated with directory we are reading
#[derive(Debug)]
struct PendingChild {
    /// Name of child
    name: String,
    /// NTFS file reference on the disk
    file_ref: NtfsEntryRef,
    /// Type of child. File, Directory, or Unsupported
    kind: EntryKind,
    /// Full path to the child
    display_path: String,
}

/// List files and directories from provided path
///
/// `display` is the human readable directory path. `inner_path` is the directory that that we should target for listing files and directories
pub(crate) fn list_children<R: Read + Seek + Send>(
    volume: &NtfsVolume<R>,
    drive: char,
    display: &str,
    inner_path: &str,
) -> AccessorResult<Vec<DirEntry>> {
    volume.with_reader(|ntfs, reader| {
        // Make sure the directory we are reading does not end with slash
        let parent_display = normalize_display_path(display);
        let dir_file = resolve_directory(ntfs, reader, inner_path)?;

        // Children walk only. Gets all files and directories in provided directory
        let pending = collect_index_children(reader, drive, &dir_file, &parent_display)?;
        process_child_entries(ntfs, reader, pending, drive)
    })
}

/// List files and directories from provided directory file reference
///
/// `display` is the parent path to the directory
pub(crate) fn list_children_handle<R: Read + Seek + Send>(
    volume: &NtfsVolume<R>,
    file_ref: &NtfsEntryRef,
    display: &str,
    drive: char,
) -> AccessorResult<Vec<DirEntry>> {
    volume.with_reader(|ntfs, reader| {
        // Make sure the directory we are reading does not end with slash
        let parent_display = normalize_display_path(display);
        let dir_file = open_by_ref(ntfs, reader, file_ref)?;

        // Children walk only. Gets all files and directories in provided directory
        let pending = collect_index_children(reader, drive, &dir_file, &parent_display)?;
        process_child_entries(ntfs, reader, pending, drive)
    })
}

/// Process children of the directory we just read
fn process_child_entries<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    pending: Vec<PendingChild>,
    drive: char,
) -> AccessorResult<Vec<DirEntry>> {
    // Now get the size for files in the directory
    let mut entries = Vec::with_capacity(pending.len());
    for child in pending {
        let size = match child.kind {
            EntryKind::Directory | EntryKind::Unsupported => 0,
            // Only files have sizes
            EntryKind::File => get_file_size(ntfs, reader, child.file_ref.file_record_number)?,
        };
        let meta = EntryMeta::new(child.kind.clone(), size, child.display_path.clone());
        let handle = match child.kind {
            EntryKind::Directory => ItemHandle::Directory(DirHandle::new(DirLocator::Ntfs {
                drive,
                dir_ref: child.file_ref,
                display_path: child.display_path,
            })),
            EntryKind::File => ItemHandle::File(FileHandle::new(FileLocator::Ntfs {
                drive,
                file_ref: child.file_ref,
                display_path: child.display_path,
            })),
            EntryKind::Unsupported => continue,
        };
        entries.push(DirEntry::new(child.name, handle, meta));
    }

    Ok(entries)
}

/// Return a `NtfsFile` from provided file path
pub(crate) fn resolve_file<'a, R: Read + Seek>(
    ntfs: &'a Ntfs,
    reader: &mut R,
    inner_path: &str,
) -> AccessorResult<NtfsFile<'a>> {
    let components = split_inner_path(inner_path);
    if components.is_empty() {
        return Err(AccessorError::not_a_file(inner_path));
    }

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

    if current.is_directory() {
        return Err(AccessorError::not_a_file(inner_path));
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

/// Walk the directory index and return children
///
/// Required before any `$DATA` size lookup
fn collect_index_children<'a, R: Read + Seek>(
    reader: &mut R,
    drive: char,
    dir_file: &NtfsFile<'a>,
    parent_display: &str,
) -> AccessorResult<Vec<PendingChild>> {
    let index = dir_file.directory_index(reader).map_err(ntfs_err)?;
    let mut iter = index.entries();
    let mut pending = Vec::new();

    while let Some(Ok(entry)) = iter.next(reader) {
        let child = {
            if entry.flags().contains(NtfsIndexEntryFlags::LAST_ENTRY) {
                None
            } else {
                let key = match entry.key() {
                    Some(Ok(key)) => key,
                    Some(Err(err)) => return Err(ntfs_err(err)),
                    None => continue,
                };
                // Skip DOS names
                if key.namespace() == NtfsFileNamespace::Dos {
                    None
                } else {
                    let name = key.name().to_string_lossy();
                    // Special directories
                    if name == "." || name == ".." {
                        None
                    } else {
                        let file_ref = NtfsEntryRef::from_reference(entry.file_reference());
                        let kind = if key.is_directory() {
                            EntryKind::Directory
                        } else {
                            EntryKind::File
                        };
                        let display_path = if parent_display.is_empty() {
                            format!("{drive}:\\{name}")
                        } else {
                            format!("{parent_display}\\{name}")
                        };
                        Some(PendingChild {
                            name,
                            file_ref,
                            kind,
                            display_path,
                        })
                    }
                }
            }
        };

        // Track files and directories found
        if let Some(child) = child {
            pending.push(child);
        }
    }

    Ok(pending)
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
pub(crate) fn get_file_size<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    record_number: u64,
) -> AccessorResult<u64> {
    // Get direct access to the file via file reference
    let file = ntfs.file(reader, record_number).map_err(ntfs_err)?;

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
    use crate::accessor::{
        entry::handle::EntryKind,
        filesystem::ntfs::{volume::NtfsVolume, walk::list_children},
    };
    use std::path::PathBuf;

    #[test]
    fn test_ntfs_volume() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/filesystems/ntfs/test.raw");

        let volume = NtfsVolume::open_image(test_location).unwrap();
        let result = list_children(&volume, 'C', &"", &"").unwrap();

        assert_eq!(result.len(), 15);

        for entry in result {
            if entry.name == "main.ts" {
                assert_eq!(entry.meta.kind, EntryKind::File);
            }
        }

        let result = list_children(&volume, 'c', &"C:\\hello", &"hello").unwrap();
        assert_eq!(result[0].name, "hello world.txt");
    }
}
