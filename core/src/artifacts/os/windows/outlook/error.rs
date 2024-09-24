use std::fmt;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutlookError {
    LeafNode,
    ReadFile,
    Base64Property,
    NameIdMap,
    NoBlocks,
    PropertyContext,
    TableContext,
    MessageCount,
    UnknownPageFormat,
    Systemdrive,
    Serialize,
    OutputData,
    GlobPath,
    Header,
    Xblock,
    RawBlock,
    NodeBtree,
    BlockBtree,
    BadBranch,
}

impl std::error::Error for OutlookError {}

impl fmt::Display for OutlookError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OutlookError::ReadFile => write!(f, "Failed to read outlook file"),
            OutlookError::Systemdrive => write!(f, "Failed to get Windows system drive"),
            OutlookError::UnknownPageFormat => write!(f, "Unknown page format detected"),
            OutlookError::LeafNode => write!(f, "Failed to read leaf node"),
            OutlookError::Base64Property => write!(f, "Failed to base64 decode binary property"),
            OutlookError::NameIdMap => write!(f, "Failed to parse NameMapId"),
            OutlookError::NoBlocks => {
                write!(f, "Missing blocks. Cant parse Property or Table Context")
            }
            OutlookError::PropertyContext => {
                write!(f, "Failed to parse the Property Context table")
            }
            OutlookError::TableContext => {
                write!(f, "Failed to parse the Table Context table")
            }
            OutlookError::MessageCount => {
                write!(f, "Too many messages requested, not enough available")
            }
            OutlookError::Serialize => write!(f, "Failed to serialize outlook messages"),
            OutlookError::OutputData => write!(f, "Failed to output outlook messages"),
            OutlookError::GlobPath => write!(f, "Failed to glob paths"),
            OutlookError::Header => write!(f, "Failed to parser outlook header"),
            OutlookError::Xblock => write!(f, "Failed to parse xblock"),
            OutlookError::RawBlock => write!(f, "Failed to parse raw block"),
            OutlookError::NodeBtree => write!(f, "Failed to parse node btree"),
            OutlookError::BlockBtree => write!(f, "Failed to parse block btree"),
            OutlookError::BadBranch => write!(f, "Failed to parse btree branch"),
        }
    }
}
