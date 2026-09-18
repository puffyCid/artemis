use crate::accessor::{
    entry::{
        handle::{DirEntry, DirHandle, EntryMeta, FileHandle, ItemHandle},
        locator::{DirLocator, FileLocator, NtfsEntryRef},
    },
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::{
        data::{merge_ntfs_times, ntfs_filename_times, ntfs_standard_times},
        volume::NtfsVolume,
    },
};
use common::files::EntryKind;
use ntfs::{
    Ntfs, NtfsFile, NtfsIndexEntryFlags, indexes::NtfsFileNameIndex,
    structured_values::NtfsFileNamespace,
};
use std::{
    collections::HashMap,
    io::{Read, Seek},
};
use tracing::{error, warn};

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

        list_index_children(
            ntfs,
            reader,
            &dir_file,
            drive,
            &parent_display,
            volume.sids(),
        )
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

        list_index_children(
            ntfs,
            reader,
            &dir_file,
            drive,
            &parent_display,
            volume.sids(),
        )
    })
}

/// Extract entries from INDX attribute
fn list_index_children<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
    dir_file: &NtfsFile<'_>,
    drive: char,
    parent_display: &str,
    sids: &HashMap<u32, (String, String)>,
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
            read_file_size(file, reader)?
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

        let mut meta = EntryMeta::new(kind, size, scheme_path);

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
    read_file_size(file, reader)
}

fn read_file_size<T: Read + Seek>(file: NtfsFile<'_>, reader: &mut T) -> AccessorResult<u64> {
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
