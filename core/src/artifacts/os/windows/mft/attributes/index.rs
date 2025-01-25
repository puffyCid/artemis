use super::{
    filename::Filename,
    header::{AttributeHeader, AttributeType},
};
use crate::utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian};
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct IndexRoot {
    root_header: RootHeader,
    node_header: NodeHeader,
    /**May contain FILENAME or other attributes?*/
    values: Value,
}

#[derive(Debug)]
pub(crate) struct RootHeader {
    /**Same as MFT attributes */
    attribute_type: AttributeType,
    collation_type: CollationType,
    entry_size: u32,
    cluster_block_count: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum CollationType {
    Binary,
    Filename,
    Unicode,
    Uint32,
    Sid,
    /**Security hash AND then SID */
    SecurityHashSid,
    Uint32Array,
    Unknown,
}

#[derive(Debug)]
pub(crate) struct NodeHeader {
    values_offset: u32,
    node_size: u32,
    allocated_size: u32,
    is_branch: bool,
}

impl IndexRoot {
    /// Parse the Index Root attribute
    pub(crate) fn parse_root(data: &[u8]) -> nom::IResult<&[u8], Value> {
        let (input, root_header) = IndexRoot::parse_root_header(data)?;
        let (input, node_header) = IndexRoot::parse_node_header(input)?;
        let (input, index_entry) =
            IndexRoot::parse_index_entry(input, &root_header.attribute_type)?;
        Ok((input, index_entry))
    }

    /// Extract root header from Index
    fn parse_root_header(data: &[u8]) -> nom::IResult<&[u8], RootHeader> {
        let (input, attribute_type) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, collation_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, entry_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, cluster_block_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let header = RootHeader {
            attribute_type: AttributeHeader::get_type(&attribute_type),
            collation_type: IndexRoot::get_collation_type(&collation_type),
            entry_size,
            cluster_block_count,
        };

        Ok((input, header))
    }

    /// Extract node header from Index
    fn parse_node_header(data: &[u8]) -> nom::IResult<&[u8], NodeHeader> {
        let (input, values_offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, node_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, allocated_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let header = NodeHeader {
            values_offset,
            node_size,
            allocated_size,
            is_branch: if flags == 1 { true } else { false },
        };

        Ok((input, header))
    }

    /// Grab attribute Index entry
    fn parse_index_entry<'a>(
        data: &'a [u8],
        attribute_type: &AttributeType,
    ) -> nom::IResult<&'a [u8], Value> {
        let (input, parent_mft) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, parent_sequence) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, key_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, data_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, flag) = nom_unsigned_four_bytes(input, Endian::Le)?;

        // Sometimes entry does not have attribute data?
        if input.is_empty() {
            return Ok((input, Value::Null));
        }

        if *attribute_type == AttributeType::FileName {
            let min_size = 60;
            if input.len() < min_size {
                return Ok((input, Value::Null));
            }
            let (input, filename) = Filename::parse_filename(input)?;
            return Ok((input, serde_json::to_value(filename).unwrap()));
        } else if *attribute_type == AttributeType::Unused {
            return Ok((&[], Value::Null));
        }

        println!("{attribute_type:?}");
        panic!("{input:?}");
    }

    /// Determine collection type for Index
    fn get_collation_type(data: &u32) -> CollationType {
        match data {
            0x0 => CollationType::Binary,
            0x1 => CollationType::Filename,
            0x2 => CollationType::Unicode,
            0x10 => CollationType::Uint32,
            0x11 => CollationType::Sid,
            0x12 => CollationType::SecurityHashSid,
            0x13 => CollationType::Uint32Array,
            _ => CollationType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::IndexRoot;
    use crate::artifacts::os::windows::mft::attributes::index::{AttributeType, CollationType};

    #[test]
    fn test_parse_root() {
        let test = [
            48, 0, 0, 0, 1, 0, 0, 0, 0, 16, 0, 0, 1, 0, 0, 0, 16, 0, 0, 0, 152, 0, 0, 0, 152, 0, 0,
            0, 1, 0, 0, 0, 49, 124, 1, 0, 0, 0, 5, 0, 112, 0, 82, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0,
            5, 0, 200, 193, 152, 167, 186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 200,
            193, 152, 167, 186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 36, 0, 16, 3, 0, 0, 160, 8, 2, 68, 0, 79, 0, 67, 0,
            85, 0, 77, 0, 69, 0, 126, 0, 49, 0, 107, 0, 46, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = IndexRoot::parse_root(&test).unwrap();
        assert!(result.to_string().contains("DOCUME~1"));
    }

    #[test]
    fn test_parse_root_header() {
        let test = [
            48, 0, 0, 0, 1, 0, 0, 0, 0, 16, 0, 0, 1, 0, 0, 0, 16, 0, 0, 0, 152, 0, 0, 0, 152, 0, 0,
            0, 1, 0, 0, 0, 49, 124, 1, 0, 0, 0, 5, 0, 112, 0, 82, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0,
            5, 0, 200, 193, 152, 167, 186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 200,
            193, 152, 167, 186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 36, 0, 16, 3, 0, 0, 160, 8, 2, 68, 0, 79, 0, 67, 0,
            85, 0, 77, 0, 69, 0, 126, 0, 49, 0, 107, 0, 46, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = IndexRoot::parse_root_header(&test).unwrap();
        assert_eq!(result.cluster_block_count, 1);
        assert_eq!(result.entry_size, 4096);
        assert_eq!(result.attribute_type, AttributeType::FileName);
        assert_eq!(result.collation_type, CollationType::Filename);
    }

    #[test]
    fn test_parse_node_header() {
        let test = [
            16, 0, 0, 0, 152, 0, 0, 0, 152, 0, 0, 0, 1, 0, 0, 0, 49, 124, 1, 0, 0, 0, 5, 0, 112, 0,
            82, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 5, 0, 200, 193, 152, 167, 186, 223, 218, 1, 200,
            193, 152, 167, 186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 200, 193, 152,
            167, 186, 223, 218, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 36, 0, 16, 3,
            0, 0, 160, 8, 2, 68, 0, 79, 0, 67, 0, 85, 0, 77, 0, 69, 0, 126, 0, 49, 0, 107, 0, 46,
            0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0, 0, 0, 3, 0, 0, 0, 1,
            0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = IndexRoot::parse_node_header(&test).unwrap();
        assert_eq!(result.allocated_size, 152);
        assert_eq!(result.node_size, 152);
        assert_eq!(result.values_offset, 16);
        assert_eq!(result.is_branch, true);
    }

    #[test]
    fn test_parse_index_entry() {
        let test = [
            49, 124, 1, 0, 0, 0, 5, 0, 112, 0, 82, 0, 1, 0, 0, 0, 5, 0, 0, 0, 0, 0, 5, 0, 200, 193,
            152, 167, 186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 200, 193, 152, 167,
            186, 223, 218, 1, 200, 193, 152, 167, 186, 223, 218, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 6, 36, 0, 16, 3, 0, 0, 160, 8, 2, 68, 0, 79, 0, 67, 0, 85, 0, 77, 0,
            69, 0, 126, 0, 49, 0, 107, 0, 46, 0, 108, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 24, 0, 0, 0, 3, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = IndexRoot::parse_index_entry(&test, &AttributeType::FileName).unwrap();
        assert!(result.to_string().contains("133665131729568200"));
    }

    #[test]
    fn test_get_collation_type() {
        let test = [0x0, 0x1, 0x2, 0x10, 0x11, 0x12, 0x13];

        for entry in test {
            let result = IndexRoot::get_collation_type(&entry);
            assert_ne!(result, CollationType::Unknown);
        }
    }
}
