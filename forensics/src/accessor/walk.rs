use tracing::error;

use crate::accessor::{
    access::Accessor,
    entry::{handle::DirEntry, locator::SourceId},
    error::{AccessorError, AccessorResult},
    location::path::InnerPath,
    source::{factory::parse_inner_path, handle::SourceHandle},
};
use std::collections::HashSet;

/// A special accessor that be used to recursively iterator through an `Accessor` `Source`
pub(crate) struct WalkAccessor {
    /// `Source` we should iterate through
    source: SourceHandle,
    /// Start path for the recursion
    start: InnerPath,
    /// Max depth we should descend. Default is 1
    max_depth: u32,
    /// Flag to determine if we need to start at provided start path
    started: bool,
    /// Current iterator stack
    stack: Vec<WalkStack>,
    /// Error we may encounter while iterating
    pending_error: Option<AccessorError>,
    /// Default firmlinks on macOS we ignore
    firmlinks: HashSet<String>,
}

/// Track files and directories we walk
struct WalkStack {
    /// Current depth
    depth: u32,
    /// Array of children from current path in the iterator
    child: Vec<DirEntry>,
}

/// Entry returned by `WalkAccessor`
#[derive(Debug)]
pub(crate) struct WalkEntry {
    /// A File or Directory return by the iterator
    pub(crate) entry: DirEntry,
    /// Current depth
    pub(crate) depth: u32,
}

impl WalkAccessor {
    /// Return a `WalkAccessor` to iterate the `SourceHandle`
    ///
    /// Default **depth** is 1
    pub(crate) fn new(source: &SourceHandle, inner: &str) -> AccessorResult<Self> {
        Ok(Self {
            source: source.clone(),
            start: parse_inner_path(inner)?,
            max_depth: 1,
            started: false,
            stack: Vec::new(),
            pending_error: None,
            firmlinks: HashSet::new(),
        })
    }

    /// Max depth we should descend to
    pub(crate) fn max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    /// Iterator to walk the filesystem
    pub(crate) fn next(&mut self, accessor: &Accessor) -> Option<AccessorResult<WalkEntry>> {
        if let Some(err) = self.pending_error.take() {
            return Some(Err(err));
        }

        // Determine if we need to start the walk
        if !self.started {
            self.started = true;
            self.load_firmlinks(accessor);

            if let Err(err) = self.start_walk(accessor) {
                return Some(Err(err));
            }
        }

        loop {
            let walk_stack = self.stack.last_mut()?;
            if walk_stack.child.is_empty() {
                self.stack.pop();
                continue;
            }

            let child = walk_stack.child.pop()?;
            let depth = walk_stack.depth + 1;

            // On macOS systems we always ignore firmlink paths
            if self.is_firmlink(&child.meta.full_path) {
                continue;
            }

            self.queue_descend(accessor, &child, depth);
            return Some(Ok(WalkEntry {
                entry: child,
                depth,
            }));
        }
    }

    /// Start the iterator by reading the provided start path
    fn start_walk(&mut self, accessor: &Accessor) -> AccessorResult<()> {
        let mut child = accessor.source_read_dir(&self.source, &self.start.display())?;
        child.reverse();

        self.stack.push(WalkStack { depth: 0, child });

        Ok(())
    }

    /// Track paths we need to descend
    fn queue_descend(&mut self, accessor: &Accessor, entry: &DirEntry, depth: u32) {
        if !entry.is_directory() || depth >= self.max_depth {
            return;
        }

        let Some(handle) = entry.handle.as_directory() else {
            error!("Cannot list files for '{}'", entry.meta.full_path);
            self.pending_error = Some(AccessorError::NotADirectory {
                path: entry.meta.full_path.clone(),
            });
            return;
        };

        // We always read directories by `DirHandle`
        match accessor.source_read_dir_handle(&self.source, handle) {
            Ok(mut child) => {
                child.reverse();
                self.stack.push(WalkStack { depth, child });
            }
            Err(err) => {
                error!(
                    "Failed to descend filelisting at '{}': {err:?}",
                    entry.meta.full_path
                );
                self.pending_error = Some(err);
            }
        }
    }

    /// On macOS read the default firmlink paths
    fn load_firmlinks(&mut self, accessor: &Accessor) {
        if !cfg!(target_os = "macos") || self.source.id() != &SourceId::Host {
            return;
        }

        let firmlinks = "/usr/share/firmlinks";
        // Firmlinks appear as normal directories in Rust
        // So we have to skip them otherwise our filelisting doubles in size
        let bytes = match accessor.source_read_file(&self.source, firmlinks) {
            Ok(results) => results,
            Err(err) => {
                error!("Could not read '{firmlinks}' on macOS: {err:?}");
                return;
            }
        };

        for line in String::from_utf8_lossy(&bytes).lines() {
            if let Some(path) = line.split_whitespace().next()
                && !path.is_empty()
            {
                self.firmlinks.insert(path.to_string());
            }
        }
    }

    /// Check if path is a firmlink
    fn is_firmlink(&self, full_path: &str) -> bool {
        self.firmlinks.contains(full_path)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{access::Accessor, error::AccessorError, walk::WalkAccessor};
    use std::{
        fs::{self, File},
        io::Write,
        path::PathBuf,
    };

    fn setup(name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp/walk").join(name);
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_file(dir: &PathBuf, name: &str, contents: &[u8]) {
        let path = dir.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        File::create(path).unwrap().write_all(contents).unwrap();
    }

    fn collect(walk: &mut WalkAccessor, accessor: &Accessor) -> Vec<(u32, String)> {
        let mut out = Vec::new();
        while let Some(item) = walk.next(accessor) {
            let Ok(entry) = item else {
                continue;
            };
            out.push((entry.depth, entry.entry.meta.filename.clone()));
        }
        out
    }

    #[test]
    fn test_walk_accessor() {
        let test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let mut walk = WalkAccessor::new(&source, test_location.to_str().unwrap())
            .unwrap()
            .max_depth(5);

        let mut count = 0;
        while let Some(item) = walk.next(&accessor) {
            let entry = item.unwrap();
            count += 1;

            assert!(!entry.entry.meta.full_path.is_empty());
        }

        assert!(count > 10);
    }

    #[test]
    fn test_walk_accessor_zip() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/archives/document.odt");

        let mut accessor = Accessor::with_defaults();
        let source = accessor
            .open_source(&format!("zip:{}", test_location.display()))
            .unwrap();

        let mut walk = WalkAccessor::new(&source, "").unwrap().max_depth(5);

        let mut count = 0;
        while let Some(item) = walk.next(&accessor) {
            let entry = item.unwrap();
            count += 1;
            assert!(!entry.entry.meta.full_path.is_empty());
        }

        assert!(count > 10, "{}", count);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_walk_accessor_ntfs() {
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source(&"ntfs:C").unwrap();
        let mut walk = WalkAccessor::new(&source, "").unwrap().max_depth(2);

        let mut count = 0;
        while let Some(item) = walk.next(&accessor) {
            let entry = item.unwrap();
            count += 1;
            assert!(!entry.entry.meta.full_path.is_empty());
        }

        assert!(count > 10, "{}", count);
    }

    #[test]
    fn test_walk_max_depth() {
        let dir = setup("max_depth");
        write_file(&dir, "a.txt", b"a");
        write_file(&dir, "nested/b.txt", b"b");
        write_file(&dir, "nested/deep/c.txt", b"c");

        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();
        let start = dir.display().to_string();
        let mut walk = WalkAccessor::new(&source, &start).unwrap().max_depth(1);
        let names: Vec<String> = collect(&mut walk, &accessor)
            .into_iter()
            .map(|(_, name)| name)
            .collect();

        assert!(names.contains(&"a.txt".to_string()));
        assert!(names.contains(&"nested".to_string()));
        assert!(!names.contains(&"b.txt".to_string()));
        assert!(!names.contains(&"c.txt".to_string()));

        let mut walk = WalkAccessor::new(&source, &start).unwrap().max_depth(2);
        let names: Vec<String> = collect(&mut walk, &accessor)
            .into_iter()
            .map(|(_, name)| name)
            .collect();

        assert!(names.contains(&"b.txt".to_string()));
        assert!(!names.contains(&"c.txt".to_string()));
    }

    #[test]
    fn test_walk_stat_during_iteration() {
        let dir = setup("stat_during");
        write_file(&dir, "file.txt", b"hello");

        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();
        let mut walk = WalkAccessor::new(&source, &dir.display().to_string())
            .unwrap()
            .max_depth(1);

        let mut saw_file = false;

        while let Some(item) = walk.next(&accessor) {
            let entry = item.unwrap();
            if let Some(file) = entry.entry.handle.as_file() {
                let stat = accessor.source_stat_handle(&source, file).unwrap();
                assert_eq!(stat.meta.filename, "file.txt");
                assert_eq!(stat.meta.size, 5);
                saw_file = true;
            }

            if let Some(dir_handle) = entry.entry.handle.as_directory() {
                let stat = accessor
                    .source_stat_dir_handle(&source, dir_handle)
                    .unwrap();
                assert_eq!(
                    stat.meta.kind,
                    crate::accessor::entry::handle::EntryKind::Directory
                );
            }
        }

        assert!(saw_file);
    }

    #[test]
    fn test_walk_zip_nested_and_empty_dir() {
        let mut archive = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        archive.push("tests/test_data/archives/document.odt");

        let mut accessor = Accessor::with_defaults();
        let source = accessor
            .open_source(&format!("zip:{}", archive.display()))
            .unwrap();

        let mut walk = WalkAccessor::new(&source, "").unwrap().max_depth(5);
        let names: Vec<String> = collect(&mut walk, &accessor)
            .into_iter()
            .map(|(_, name)| name)
            .collect();

        assert!(names.contains(&"content.xml".to_string()));
        assert!(names.contains(&"Configurations2".to_string()));
        assert!(names.contains(&"thumbnail.png".to_string()));
        assert!(names.contains(&"manifest.xml".to_string()));
    }

    #[test]
    fn test_walk_zip_missing_prefix() {
        let mut archive = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        archive.push("tests/test_data/archives/document.odt");

        let mut accessor = Accessor::with_defaults();
        let source = accessor
            .open_source(&format!("zip:{}", archive.display()))
            .unwrap();

        let mut walk = WalkAccessor::new(&source, "no/such/dir").unwrap();
        let err = walk.next(&accessor).unwrap().unwrap_err();

        assert!(matches!(err, AccessorError::NotFound { .. }));
        assert!(walk.next(&accessor).is_none());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_walk_skips_firmlinks_but_not_start() {
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let mut walk = WalkAccessor::new(&source, "/").unwrap().max_depth(2);
        let paths: Vec<String> = collect(&mut walk, &accessor)
            .into_iter()
            .map(|(_, name)| name)
            .collect();

        assert!(!paths.iter().any(|name| name == "Users"));
        let mut walk = WalkAccessor::new(&source, "/Users").unwrap().max_depth(1);
        let users = collect(&mut walk, &accessor);

        assert!(!users.is_empty());
    }
}
