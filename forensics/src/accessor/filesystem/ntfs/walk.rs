use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, EntryKind, EntryMeta, FileHandle, ItemHandle},
        locator::{DirLocator, FileLocator, NtfsEntryRef},
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::volume::NtfsVolume,
};
use ntfs::{
    Ntfs, NtfsAttributeType, NtfsFile, NtfsIndexEntryFlags, indexes::NtfsFileNameIndex,
    structured_values::NtfsFileNamespace,
};
use std::io::{Read, Seek};

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

pub(crate) fn list_children<R: Read + Seek + Send>(
    volume: &NtfsVolume<R>,
    drive: char,
    dir_display_path: &str,
    inner_path: &str,
) -> AccessorResult<Vec<DirEntry>> {
    volume.with_reader(|ntfs, reader| {
        // Make sure the directory we are reading does not end with slash
        let parent_display = normalize_display_path(dir_display_path);

        // Children walk only. Gets all files and directories in provided directory
        let pending = collect_index_children(ntfs, reader, drive, inner_path, &parent_display)?;

        // Now get the size for files in the directory
        let mut entries = Vec::with_capacity(pending.len());
        for child in pending {
            let size = match child.kind {
                EntryKind::Directory => 0,
                // Only files have sizes
                EntryKind::File => get_file_size(ntfs, reader, child.file_ref.file_record_number)?,
                EntryKind::Unsupported => 0,
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
    })
}

/// Walk the directory index and return children
///
/// Required before any `$DATA` size lookup
fn collect_index_children<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    drive: char,
    inner_path: &str,
    parent_display: &str,
) -> AccessorResult<Vec<PendingChild>> {
    let dir_file = resolve_directory(ntfs, reader, inner_path)?;
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
                    None => return Ok(pending),
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
fn get_file_size<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    record_number: u64,
) -> AccessorResult<u64> {
    // Get direct access to the file via file reference
    let file = ntfs.file(reader, record_number).map_err(ntfs_err)?;

    for attr_result in file.attributes_raw() {
        let attr = attr_result.map_err(ntfs_err)?;
        if attr.ty().map_err(ntfs_err)? != NtfsAttributeType::Data {
            continue;
        }
        // We want the size of the $DATA attribute
        let name = attr.name().map_err(ntfs_err)?;
        if name.is_empty() {
            return Ok(attr.value_length());
        }
    }
    Ok(0)
}

/// Split the target directory we want to read into array of strings
fn split_inner_path(inner_path: &str) -> Vec<String> {
    inner_path
        .trim_matches(['\\', '/'])
        .split(['\\', '/'])
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

/// Remove any slashes at end of directoy we want to read
fn normalize_display_path(path: &str) -> String {
    path.trim_end_matches(['\\', '/']).to_string()
}

/// Handle `NTFSError` to `AccessorError`
fn ntfs_err(err: ntfs::NtfsError) -> AccessorError {
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

        let reader = NtfsVolume::open_image(test_location).unwrap();
        let result = list_children(&reader, 'C', &"", &"").unwrap();
        println!("{result:?}");

        assert_eq!(result.len(), 15);

        for entry in result {
            if entry.name == "main.ts" {
                assert_eq!(entry.meta.kind, EntryKind::File);
            }
        }
    }
}
