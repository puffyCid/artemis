use tracing::error;

use crate::accessor::{
    access::Accessor,
    entry::{
        handle::{DirEntry, DirHandle, EntryKind, FileHandle, ItemHandle},
        locator::{DirLocator, FileLocator, NtfsEntryRef, SourceId},
    },
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
    started: bool,
    stack: Vec<WalkStack>,
    pending_error: Option<AccessorError>,
    /// Default firmlinks on macOS we ignore
    firmlinks: HashSet<String>,
}

struct WalkStack {
    depth: u32,
    child: Vec<DirEntry>,
}

/// Entry returned by `WalkAccessor`
pub(crate) struct WalkEntry {
    entry: DirEntry,
    depth: u32,
}

impl WalkEntry {
    pub(crate) fn depth(&self) -> u32 {
        self.depth
    }

    pub(crate) fn filename(&self) -> &str {
        &self.entry.meta.filename
    }

    pub(crate) fn full_path(&self) -> &str {
        &self.entry.meta.full_path
    }

    pub(crate) fn display_path(&self) -> &str {
        &self.entry.meta.display_path
    }

    pub(crate) fn is_file(&self) -> bool {
        self.entry.is_file()
    }

    pub(crate) fn is_directory(&self) -> bool {
        self.entry.is_directory()
    }

    pub(crate) fn as_file(&self) -> Option<&FileHandle> {
        self.entry.handle.as_file()
    }

    pub(crate) fn as_directory(&self) -> Option<&DirHandle> {
        self.entry.handle.as_directory()
    }

    pub(crate) fn handle(&self) -> &ItemHandle {
        &self.entry.handle
    }

    pub(crate) fn entry(&self) -> &DirEntry {
        &self.entry
    }
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

    pub(crate) fn max_depth(mut self, depth: u32) -> Self {
        self.max_depth = depth;
        self
    }

    pub(crate) fn next(&mut self, accessor: &Accessor) -> Option<AccessorResult<WalkEntry>> {
        if let Some(err) = self.pending_error.take() {
            return Some(Err(err));
        }

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

    fn start_walk(&mut self, accessor: &Accessor) -> AccessorResult<()> {
        let mut child = accessor.source_read_dir(&self.source, &self.start.display())?;
        child.reverse();

        self.stack.push(WalkStack { depth: 0, child });

        Ok(())
    }

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

        match accessor.source_read_dir_handle(&self.source, handle) {
            Ok(mut child) => {
                child.reverse();
                self.stack.push(WalkStack { depth, child })
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

    fn load_firmlinks(&mut self, accessor: &Accessor) {
        if !cfg!(target_os = "macos") || self.source.id() != &SourceId::Host {
            return;
        }

        let firmlinks = "/usr/share/firmlinks";
        let bytes = match accessor.source_read_file(&self.source, firmlinks) {
            Ok(results) => results,
            Err(err) => {
                error!("Could not read '{firmlinks}' on macOS: {err:?}");
                return;
            }
        };

        for line in String::from_utf8_lossy(&bytes).lines() {
            if let Some(path) = line.split_whitespace().next() {
                if !path.is_empty() {
                    self.firmlinks.insert(path.to_string());
                }
            }
        }
    }

    fn is_firmlink(&self, full_path: &str) -> bool {
        self.firmlinks.contains(full_path)
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::{access::Accessor, walk::WalkAccessor};
    use std::path::PathBuf;

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

            assert!(!entry.full_path().is_empty());
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
            assert!(!entry.full_path().is_empty());
        }

        assert!(count > 10, "{}", count);
    }
}
