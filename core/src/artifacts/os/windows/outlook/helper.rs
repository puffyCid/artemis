/*
 * Steps to parse outlook
 *
 * 1. Parse header -- DONE!
 * 2. Parse pages -- in progress!
 *    2.1 Parse block_offset_descriptor_id next?
 * 4. Parse tables -- ??
 *    - Parse Table Context
 *      - Need to determine the number of rows in the Table Context structure :)
 *        - Folders have 4 components! All have the same node_id_num value!
 *          - NormalFolder - i can parse!
 *          - HierarchyTable - i can parse!
 *          - ContentsTable - i can parse (its a descriptor table)
 *          - FaiContentsTable - i can parse!
 * 7. Support parsing remainign property_types (see: https://github.com/libyal/libfmapi/blob/main/documentation/MAPI%20definitions.asciidoc)
 *
 * (file)/offset = block btree
 * (item)/descriptor = node btree
 *
 * Working implmetation at https://github.com/Jmcleodfoss/pstreader (MIT LICENSE!)
 *  - run with: java -jar explorer-1.1.2.jar (download from: https://github.com/Jmcleodfoss/pstreader/tree/master/explorer)
 */

use super::{
    blocks::block::{parse_blocks, BlockValue},
    error::OutlookError,
    header::{parse_header, FormatType},
    pages::btree::{get_block_btree, get_node_btree, LeafBlockData, LeafNodeData},
    tables::property::{OutlookPropertyContext, PropertyContext},
};
use crate::filesystem::ntfs::reader::read_bytes;
use ntfs::NtfsFile;
use std::{collections::BTreeMap, io::BufReader};

pub(crate) struct OutlookReader<'a, T: std::io::Seek + std::io::Read> {
    pub(crate) ntfs_file: Option<&'a NtfsFile<'a>>,
    pub(crate) fs: BufReader<T>,
    pub(crate) block_btree: Vec<BTreeMap<u64, LeafBlockData>>,
    pub(crate) node_btree: Vec<BTreeMap<u32, LeafNodeData>>,
    pub(crate) format: FormatType,
    pub(crate) size: u64,
}

pub(crate) trait OutlookReaderAction<'a, T: std::io::Seek + std::io::Read> {
    fn setup(&mut self) -> Result<(), OutlookError>;
    fn get_block_data(
        &mut self,
        block: &LeafBlockData,
        descriptor: Option<&LeafBlockData>,
    ) -> Result<BlockValue, OutlookError>;
    fn message_store(&self) -> Result<Vec<PropertyContext>, OutlookError>;
    fn name_id_map(&self) -> Result<Vec<PropertyContext>, OutlookError>;
    fn root_folder(&self) -> Result<(), OutlookError>;
}

impl<'a, T: std::io::Seek + std::io::Read> OutlookReaderAction<'a, T> for OutlookReader<'a, T> {
    /// Get Block and Node BTrees and determine Outlook format type
    fn setup(&mut self) -> Result<(), OutlookError> {
        let ost_size = 564;
        let header_bytes = read_bytes(&0, ost_size, self.ntfs_file, &mut self.fs).unwrap();
        let (_, header) = parse_header(&header_bytes).unwrap();

        self.format = header.format_type;
        self.size = match self.format {
            FormatType::ANSI32 | FormatType::Unicode64 => 512,
            FormatType::Unicode64_4k => 4096,
            FormatType::Unknown => panic!("should not be possible"),
        };
        get_block_btree(
            self.ntfs_file,
            &mut self.fs,
            &header.block_btree_root,
            &self.size,
            &self.format,
            &mut self.block_btree,
        )?;
        get_node_btree(
            self.ntfs_file,
            &mut self.fs,
            &header.node_btree_root,
            &self.size,
            &self.format,
            &mut self.node_btree,
        )?;
        Ok(())
    }

    /// Get block data for a specific Block
    fn get_block_data(
        &mut self,
        block: &LeafBlockData,
        descriptor: Option<&LeafBlockData>,
    ) -> Result<BlockValue, OutlookError> {
        parse_blocks(
            self.ntfs_file,
            &mut self.fs,
            &block,
            descriptor,
            &self.block_btree,
            &self.format,
        )
    }

    /// Extract the Outlook MessageStore
    fn message_store(&self) -> Result<Vec<PropertyContext>, OutlookError> {
        /*
         * Steps:
         * 1. Get static node ID value (33) from node_btree
         * 2. Parse block data
         * 3. Parse PropertyContext
         */
        Ok(Vec::new())
    }

    /// Extract the Outlook NameToIdMap
    fn name_id_map(&self) -> Result<Vec<PropertyContext>, OutlookError> {
        /*
         * Steps:
         * 1. Get static node ID value (97) from node_btree
         * 2. Parse block data
         * 3. Parse PropertyContext
         * 4. Return HashMap of entries
         */
        Ok(Vec::new())
    }

    /// Get the Outlook Root folder. Starting point to get the contents of Outlook
    fn root_folder(&self) -> Result<(), OutlookError> {
        /*
         * Steps:
         * 1. Get static node ID value (290) from node_btree
         * 2. Parse block data
         * 3. Parse PropertyContext and TableContext
         * 4. Parse the other nodes with the ID 290
         */
        Ok(())
    }
}
