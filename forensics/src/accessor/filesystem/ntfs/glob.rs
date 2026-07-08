use crate::accessor::{
    entry::handle::{EntryKind, GlobMatch},
    error::{AccessorError, AccessorResult},
    filesystem::ntfs::{
        data::{NtfsFs, display_ntfs_path, inner_to_ntfs_path},
        walk::list_children,
    },
    location::path::InnerPath,
};
use glob::Pattern;
use std::io::{Read, Seek};

impl<R: Read + Seek + Send> NtfsFs<R> {
    /// Apply a glob pattern and return matches
    pub(crate) fn globfs(
        &self,
        directory: &InnerPath,
        pattern: &str,
    ) -> AccessorResult<Vec<GlobMatch>> {
        let inner_path = inner_to_ntfs_path(directory, self.drive);
        let display = display_ntfs_path(self.drive, &inner_path);

        let normalized = normalize_glob_pattern(pattern);

        let glob_pattern = Pattern::new(&normalized)
            .map_err(|err| AccessorError::bad_glob(pattern, err.to_string()))?;

        if normalized.contains('/') {
            let components = path_component_count(&normalized);
            let mut matches = Vec::new();

            glob_path_pattern(
                self,
                &inner_path,
                &display,
                &glob_pattern,
                "",
                components,
                &mut matches,
            )?;

            return Ok(matches);
        }

        let entries = list_children(&self.volume, self.drive, &display, &inner_path)?;

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

/// List child files and directories and check if they match our glob pattern
fn glob_path_pattern<R: Read + Seek + Send>(
    fs: &NtfsFs<R>,
    inner_path: &str,
    display: &str,
    pattern: &Pattern,
    relative_prefix: &str,
    components: usize,
    matches: &mut Vec<GlobMatch>,
) -> AccessorResult<()> {
    let entries = list_children(&fs.volume, fs.drive, display, inner_path)?;

    for entry in entries {
        let relative = join_relative(relative_prefix, &entry.name);
        let depth = path_component_count(&relative);

        match entry.meta.kind {
            EntryKind::File | EntryKind::Unsupported => {
                if pattern.matches(&relative) {
                    matches.push(GlobMatch::new(entry.handle, entry.meta));
                }
            }
            EntryKind::Directory => {
                if pattern.matches(&relative) {
                    matches.push(GlobMatch::new(entry.handle.clone(), entry.meta.clone()));
                }

                if depth < components {
                    let child_inner = join_inner(inner_path, &entry.name);
                    let child_display = entry.meta.display_path.clone();
                    glob_path_pattern(
                        fs,
                        &child_inner,
                        &child_display,
                        pattern,
                        &relative,
                        components,
                        matches,
                    )?;
                }
            }
        }
    }

    Ok(())
}

/// Apply a consistent glob separator
fn normalize_glob_pattern(pattern: &str) -> String {
    pattern.replace('\\', "/")
}

/// Combine starting directory with any directory matches from glob
fn join_inner(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_string()
    } else {
        format!("{base}\\{name}")
    }
}

/// Builds the path to compare against our glob pattern
fn join_relative(prefix: &str, name: &str) -> String {
    if prefix.is_empty() {
        name.to_string()
    } else {
        format!("{prefix}/{name}")
    }
}

fn path_component_count(path: &str) -> usize {
    if path.is_empty() {
        0
    } else {
        path.split('/').count()
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        filesystem::ntfs::{data::NtfsFs, volume::NtfsVolume},
        location::path::InnerPath,
    };
    use std::path::PathBuf;

    fn inner(part: &str) -> InnerPath {
        if part.is_empty() {
            InnerPath::empty()
        } else {
            InnerPath::new(PathBuf::from(part))
        }
    }

    fn test_fs() -> NtfsFs<std::io::BufReader<std::fs::File>> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        let volume = NtfsVolume::open_image(path).unwrap();
        NtfsFs::new(volume, 'C')
    }

    #[test]
    fn test_ntfs_globfs_nested_pattern() {
        let fs = test_fs();

        for pattern in ["*/*.txt", "*\\*.txt"] {
            let matches = fs.globfs(&inner(""), pattern).unwrap();
            assert_eq!(matches.len(), 1, "pattern: {pattern}");
            assert_eq!(
                matches[0].handle.display_path(),
                "C:\\hello\\hello world.txt"
            );
        }

        let matches = fs.globfs(&inner(""), "*/*.ts").unwrap();
        assert!(matches.is_empty());
    }
}
