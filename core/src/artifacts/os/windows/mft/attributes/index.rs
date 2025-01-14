use super::header::AttributeType;

pub(crate) struct IndexRoot {
    root_header: RootHeader,
    node_header: NodeHeader,
    /**INDX records */
    values: Vec<u8>,
}

pub(crate) struct RootHeader {
    /**Same as MFT attributes */
    attribute_type: AttributeType,
    collation_type: CollationType,
    entry_size: u32,
    cluster_block_count: u32,
}

pub(crate) enum CollationType {
    Binary,
    Filename,
    Unicode,
    Int32,
    Sid,
    /**Security hash AND then SID */
    SecurityHashSid,
    Uint32,
}

pub(crate) struct NodeHeader {
    values_offset: u32,
    node_size: u32,
    allocated_size: u32,
    is_branch: bool,
}

impl IndexRoot {
    pub(crate) fn parse_root(data: &[u8]) -> nom::IResult<&[u8], ()> {
        panic!("{data:?}");
        Ok((data, ()))
    }
}
