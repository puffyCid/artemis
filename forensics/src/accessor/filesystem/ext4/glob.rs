use crate::accessor::{
    entry::handle::{DirEntry, GlobMatch},
    error::{AccessorError, AccessorResult},
    filesystem::{
        ext4::fs::Ext4Fs,
        helper::glob::{
            DescendGuard, glob_max_depth, is_recursive, join_relative, normalize_glob_pattern,
            path_component_count,
        },
    },
    location::path::InnerPath,
};
use common::files::EntryKind;
use glob::Pattern;
use std::io::{Read, Seek};
use tracing::warn;

impl<R: Read + Seek + Send + 'static> Ext4Fs<R> {
    pub(crate) fn globfs(
        &self,
        directory: &InnerPath,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        let normalized = normalize_glob_pattern(pattern);
        let glob_pattern = Pattern::new(&normalized)
            .map_err(|err| AccessorError::bad_glob(pattern, err.to_string()))?;

        let entries = self.read_dir(directory)?;

        if normalized.contains('/') || is_recursive(&normalized) {
            let mut matches = Vec::new();
            let guard = DescendGuard::new(&normalized)?;
            glob_children(
                self,
                entries,
                &glob_pattern,
                "",
                glob_max_depth(&normalized),
                &guard,
                &mut matches,
            )?;
            return Ok(matches);
        }

        let mut matches = Vec::new();
        for entry in entries {
            if !glob_pattern.matches(&entry.name) {
                continue;
            }
            matches.push(GlobMatch::new(entry.handle, entry.meta));
        }

        Ok(matches)
    }
}

fn glob_children<R: Read + Seek + Send + 'static>(
    fs: &Ext4Fs<R>,
    entries: Vec<DirEntry>,
    pattern: &Pattern,
    relative_prefix: &str,
    max_depth: Option<usize>,
    guard: &DescendGuard,
    matches: &mut Vec<GlobMatch>,
) -> AccessorResult<()> {
    for entry in entries {
        let relative = join_relative(relative_prefix, &entry.name);
        let depth = path_component_count(&relative);

        match entry.meta.kind {
            EntryKind::Directory => {
                if pattern.matches(&relative) {
                    matches.push(GlobMatch::new(entry.handle.clone(), entry.meta.clone()));
                }

                if !guard.should_descend(&relative, depth, max_depth) {
                    continue;
                }

                let Some(dir_handle) = entry.handle.as_directory() else {
                    continue;
                };

                let children = match fs.read_dir_handle(dir_handle) {
                    Ok(children) => children,
                    Err(err) => {
                        warn!(
                            "Could not glob '{pattern}' for {}: {err:?}",
                            entry.meta.full_path
                        );
                        continue;
                    }
                };

                glob_children(fs, children, pattern, &relative, max_depth, guard, matches)?;
            }
            EntryKind::File
            | EntryKind::Symlink
            | EntryKind::Socket
            | EntryKind::BlockDevice
            | EntryKind::Pipe
            | EntryKind::CharDevice
            | EntryKind::Unsupported => {
                if pattern.matches(&relative) {
                    matches.push(GlobMatch::new(entry.handle, entry.meta));
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::Ext4Fs;
    use crate::accessor::{
        error::AccessorError, filesystem::ext4::volume::Ext4Volume, location::path::InnerPath,
    };
    use std::{fs::File, path::PathBuf};

    fn image() -> Ext4Fs<File> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        let volume = Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap();
        Ext4Fs::new(volume)
    }

    fn inner(path: &str) -> InnerPath {
        InnerPath::new(PathBuf::from(path))
    }

    #[test]
    fn test_globfs_matches_root_names() {
        let fs = image();
        let matches = fs.globfs(&inner("/"), "*").unwrap();
        let mut names: Vec<&str> = matches
            .iter()
            .map(|entry| entry.meta.filename.as_str())
            .collect();
        names.sort();

        assert_eq!(names, ["afile", "lost+found", "test"]);
        let afile = matches
            .iter()
            .find(|entry| entry.meta.filename == "afile")
            .unwrap();
        assert_eq!(afile.meta.inode, 18);
    }

    #[test]
    fn test_globfs_descends_for_a_nested_pattern() {
        let fs = image();
        let matches = fs.globfs(&inner("/"), "*/*.txt").unwrap();

        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].meta.filename, "hello.txt");
        assert_eq!(matches[0].meta.full_path, "/test/hello.txt");
    }

    #[test]
    fn test_globfs_rejects_a_file() {
        let fs = image();

        assert!(matches!(
            fs.globfs(&inner("afile"), "*"),
            Err(AccessorError::NotADirectory { .. })
        ));
    }
}
