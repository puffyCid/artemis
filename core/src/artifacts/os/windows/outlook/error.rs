use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutlookError {
    NodeBtree,
    BlockBtree,
    LeafNode,
    LeafBlock,
    ReadFile,
}

impl std::error::Error for OutlookError {}

impl fmt::Display for OutlookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutlookError::ReadFile => write!(f, "Failed to read outlook file"),
            OutlookError::NodeBtree => write!(f, "Failed to read node btree"),
            OutlookError::BlockBtree => write!(f, "Failed to read block btree"),
            OutlookError::LeafNode => write!(f, "Failed to read leaf node"),
            OutlookError::LeafBlock => write!(f, "Failed to read leaf block"),
        }
    }
}
