use crate::accessor::{entry::locator::SourceId, source::dispatch::Source};
use std::collections::HashMap;

/// A small cache that is used to cache reading of a `Source`
///
/// If we want to read the file `zip:./test.zip!./home/test.txt` and `zip:./test.zip!./home/abc.txt`
///
/// Instead of reading the zip file twice. We parse it once and then use the cache `Source` for faster content reads
pub(crate) struct SourceCache {
    /// `HashMap` of `SourceId` and `Source`
    sources: HashMap<SourceId, Source>,
}

impl SourceCache {
    /// Create a `SourceCache` structure
    pub(crate) fn new() -> Self {
        Self {
            sources: HashMap::new(),
        }
    }

    /// Return the `Source` if available
    pub(crate) fn get(&self, id: &SourceId) -> Option<&Source> {
        self.sources.get(id)
    }

    /// Return a mutable `Source` if available
    pub(crate) fn get_mut(&mut self, id: &SourceId) -> Option<&mut Source> {
        self.sources.get_mut(id)
    }

    /// Cache a `Source` and `SourceId`
    pub(crate) fn insert(&mut self, id: SourceId, source: Source) {
        self.sources.insert(id, source);
    }
}
