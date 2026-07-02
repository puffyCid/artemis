use crate::accessor::entry::locator::SourceId;

/// Handle to an opened accessor source returned by opening a file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SourceHandle {
    pub(crate) id: SourceId,
}

impl SourceHandle {
    /// Create a `SourceHandle` structure
    pub(crate) fn new(id: SourceId) -> Self {
        Self { id }
    }

    /// Return the `SourceId` for a `SourceHandle`
    pub(crate) fn id(&self) -> &SourceId {
        &self.id
    }

    /// Return the `SourceId` as a String
    pub(crate) fn display(&self) -> String {
        self.id.display()
    }
}
