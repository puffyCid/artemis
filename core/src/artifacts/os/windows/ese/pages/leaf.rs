use crate::{
    artifacts::os::windows::ese::{page::PageFlags, tags::TagFlags},
    utils::nom_helper::{
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
};
use log::error;
use nom::{bytes::complete::take, error::ErrorKind, Needed};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug)]
pub(crate) struct PageLeaf {
    pub(crate) _common_page_key_size: u16,
    pub(crate) _local_page_key_size: u16,
    pub(crate) key_suffix: Vec<u8>,
    pub(crate) key_prefix: Vec<u8>,
    pub(crate) leaf_type: LeafType,
    pub(crate) leaf_data: Value,
}

#[derive(Debug, PartialEq)]
pub(crate) enum LeafType {
    SpaceTree,
    Index,
    LongValue,
    DataDefinition,
    Unknown,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SpaceTree {
    pub(crate) number_pages: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct DataDefinition {
    /**The last fixed data. Ex: A value of three (3) means there may be fixed data for 1,2,3 values. Dependent on table */
    pub(crate) last_fixed_data: u8,
    /**The last variable data. */
    pub(crate) last_variable_data: u8,
    pub(crate) variable_data_offset: u16,
    pub(crate) fixed_data: Vec<u8>,
    pub(crate) variable_data: Vec<u8>,
}

impl PageLeaf {
    /// Parsing leaf pages is the most critical part of parsing ESE. Leaf pages determine where all the data is
    pub(crate) fn parse_leaf_page<'a>(
        data: &'a [u8],
        flags: &[PageFlags],
        key_data: &'a [u8],
        tag_flag: &[TagFlags],
    ) -> nom::IResult<&'a [u8], PageLeaf> {
        let mut leaf_data = data;
        let common_page_key_size =
            if tag_flag.contains(&TagFlags::CommonKey) && !key_data.is_empty() {
                let (input, value_size) = nom_unsigned_two_bytes(data, Endian::Le)?;
                leaf_data = input;
                value_size
            } else {
                0
            };

        let (input, local_page_key_size) =
            if tag_flag.contains(&TagFlags::CommonKey) && !key_data.is_empty() {
                nom_unsigned_two_bytes(leaf_data, Endian::Le)?
            } else {
                let (input, local_page_key_size) = nom_unsigned_one_byte(leaf_data, Endian::Be)?;
                let (input, _unknown) = nom_unsigned_one_byte(input, Endian::Be)?;
                (input, local_page_key_size as u16)
            };

        let key_prefix = if !key_data.is_empty() {
            // if the common page size (key prefix) is larger than the key data, then page flags are included in the common page size
            // just nom the first byte to get the real size
            let common_size = if common_page_key_size as usize > key_data.len() {
                let (_, value_size) = nom_unsigned_one_byte(data, Endian::Le)?;
                value_size as u16
            } else {
                common_page_key_size
            };
            let (_, key_prefix) = take(common_size)(key_data)?;
            key_prefix.to_vec()
        } else {
            Vec::new()
        };
        let (input, key_suffix_data) = take(local_page_key_size)(input)?;

        let mut page_leaf = PageLeaf {
            _common_page_key_size: common_page_key_size,
            _local_page_key_size: local_page_key_size,
            key_prefix,
            key_suffix: key_suffix_data.to_vec(),
            leaf_type: LeafType::Unknown,
            leaf_data: Value::Null,
        };
        let leaf_value_result = if flags.contains(&PageFlags::SpaceTree) {
            let (input, space_data) = PageLeaf::parse_space_tree(input)?;
            leaf_data = input;
            page_leaf.leaf_type = LeafType::SpaceTree;
            serde_json::to_value(space_data)
        } else if flags.contains(&PageFlags::Index) {
            leaf_data = input;
            page_leaf.leaf_type = LeafType::Index;
            serde_json::to_value(input)
        } else if flags.contains(&PageFlags::LongValue) {
            page_leaf.leaf_type = LeafType::LongValue;
            serde_json::to_value(input)
        } else {
            let (input, table_data) = PageLeaf::parse_data_definition(input)?;
            leaf_data = input;
            page_leaf.leaf_type = LeafType::DataDefinition;
            serde_json::to_value(table_data)
        };
        page_leaf.leaf_data = match leaf_value_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[ese] Could not serialize {:?} leaf type: {err:?}",
                    page_leaf.leaf_type
                );
                return Err(nom::Err::Failure(nom::error::Error::new(
                    input,
                    ErrorKind::Fail,
                )));
            }
        };

        Ok((leaf_data, page_leaf))
    }

    /// Parse space tree page values
    fn parse_space_tree(data: &[u8]) -> nom::IResult<&[u8], SpaceTree> {
        let (input, number_pages) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let space_tree = SpaceTree { number_pages };

        Ok((input, space_tree))
    }

    /// Parse data definition Page values which contains table data
    fn parse_data_definition(data: &[u8]) -> nom::IResult<&[u8], DataDefinition> {
        let (input, last_fixed_data) = nom_unsigned_one_byte(data, Endian::Be)?;
        let (input, last_variable_data) = nom_unsigned_one_byte(input, Endian::Be)?;
        let (input, variable_data_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let adjust_offset = 4;
        if variable_data_offset < adjust_offset {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        // Offset is from the start of the table data, but we already nom'd four (4) bytes
        // Adjust offset accordingly, this gives use the start the variable data and the fixed data
        let (input, fixed) = take(variable_data_offset - adjust_offset)(input)?;

        let table = DataDefinition {
            last_fixed_data,
            last_variable_data,
            variable_data_offset,
            fixed_data: fixed.to_vec(),
            variable_data: input.to_vec(),
        };
        Ok((input, table))
    }
}

#[cfg(test)]
mod tests {
    use super::PageLeaf;
    use crate::artifacts::os::windows::ese::{page::PageFlags, pages::leaf::LeafType};
    use serde_json::json;

    #[test]
    fn test_parse_leaf_page() {
        let test = [
            13, 32, 127, 128, 0, 0, 2, 127, 128, 1, 127, 128, 0, 0, 2, 8, 128, 32, 0, 2, 0, 0, 0,
            1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 80, 0, 0, 0, 0, 0, 0, 192, 20, 0, 0, 0, 255, 0, 11, 0,
            77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115,
        ];

        let flags = vec![PageFlags::Root, PageFlags::Leaf];
        let (_, results) = PageLeaf::parse_leaf_page(&test, &flags, &[], &[]).unwrap();

        assert_eq!(results._common_page_key_size, 0);
        assert_eq!(results._local_page_key_size, 13);
        assert_eq!(results.key_prefix.is_empty(), true);
        assert_eq!(
            results.key_suffix,
            [127, 128, 0, 0, 2, 127, 128, 1, 127, 128, 0, 0, 2]
        );
        assert_eq!(results.leaf_type, LeafType::DataDefinition);
        assert_eq!(
            results.leaf_data,
            json![ {"last_fixed_data": 8, "last_variable_data": 128, "variable_data_offset": 32, "fixed_data": [2, 0, 0, 0, 1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 80, 0, 0, 0, 0, 0, 0, 192, 20, 0, 0, 0, 255, 0], "variable_data": [11, 0, 77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115]}]
        )
    }

    #[test]
    fn test_parse_data_definition() {
        let test = [
            8, 128, 32, 0, 2, 0, 0, 0, 1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 80, 0, 0, 0, 0, 0, 0, 192, 20,
            0, 0, 0, 255, 0, 11, 0, 77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115,
        ];

        let (_, results) = PageLeaf::parse_data_definition(&test).unwrap();

        assert_eq!(results.last_fixed_data, 8);
        assert_eq!(results.last_variable_data, 128);
        assert_eq!(results.variable_data_offset, 32);
        assert_eq!(
            results.fixed_data,
            [
                2, 0, 0, 0, 1, 0, 2, 0, 0, 0, 4, 0, 0, 0, 80, 0, 0, 0, 0, 0, 0, 192, 20, 0, 0, 0,
                255, 0
            ]
        );
        assert_eq!(
            results.variable_data,
            [11, 0, 77, 83, 121, 115, 79, 98, 106, 101, 99, 116, 115]
        );
    }

    #[test]
    fn test_parse_leaf_page_space() {
        let test = [4, 64, 0, 0, 0, 46, 5, 0, 0, 0];

        let flags = vec![PageFlags::Root, PageFlags::Leaf, PageFlags::SpaceTree];
        let (_, results) = PageLeaf::parse_leaf_page(&test, &flags, &[], &[]).unwrap();

        assert_eq!(results._common_page_key_size, 0);
        assert_eq!(results._local_page_key_size, 4);
        assert_eq!(results.key_suffix, [0, 0, 0, 46]);
        assert_eq!(results.key_prefix.is_empty(), true);
        assert_eq!(results.leaf_type, LeafType::SpaceTree);
        assert_eq!(results.leaf_data, json![ {"number_pages": 5}])
    }

    #[test]
    fn test_parse_space_tree() {
        let test = [5, 0, 0, 0];
        let (_, result) = PageLeaf::parse_space_tree(&test).unwrap();
        assert_eq!(result.number_pages, 5);
    }
}
