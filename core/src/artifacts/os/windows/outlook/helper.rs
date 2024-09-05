/*
 * TODO:
 * 1. Support parsing remainign property_types (see: https://github.com/libyal/libfmapi/blob/main/documentation/MAPI%20definitions.asciidoc)
 * 3. Some clean up
 * 4. Make TableContext rows a iterator somehow...?
 * 5. Get FAI contenst!
 * 6. Get email contents!
 *
 * (file)/offset = block btree
 * (item)/descriptor = node btree
 *
 * Working implmetation at https://github.com/Jmcleodfoss/pstreader (MIT LICENSE!)
 *  - run with: java -jar explorer-1.1.2.jar (download from: https://github.com/Jmcleodfoss/pstreader/tree/master/explorer)
 */

use super::{
    blocks::block::{BlockValue, OutlookBlock},
    error::OutlookError,
    header::{parse_header, FormatType, Node, NodeID},
    items::{
        fai::{extract_fai, FolderMeta},
        folder::{folder_details, search_folder_details, FolderInfo},
    },
    pages::btree::{
        get_block_btree, get_node_btree, BlockType, LeafBlockData, LeafNodeData, NodeBtree,
    },
    tables::property::{OutlookPropertyContext, PropertyContext},
};
use crate::{
    artifacts::os::windows::outlook::tables::context::OutlookTableContext,
    filesystem::ntfs::reader::read_bytes,
};
use log::warn;
use ntfs::NtfsFile;
use std::{collections::BTreeMap, io::BufReader};

pub(crate) struct OutlookReader<T: std::io::Seek + std::io::Read> {
    pub(crate) fs: BufReader<T>,
    pub(crate) block_btree: Vec<BTreeMap<u64, LeafBlockData>>,
    pub(crate) node_btree: Vec<NodeBtree>,
    pub(crate) format: FormatType,
    pub(crate) size: u64,
}

pub(crate) trait OutlookReaderAction<T: std::io::Seek + std::io::Read> {
    fn setup(&mut self, ntfs_file: Option<&NtfsFile<'_>>) -> Result<(), OutlookError>;
    fn get_block_data(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block: &LeafBlockData,
        descriptor: Option<&LeafBlockData>,
    ) -> Result<BlockValue, OutlookError>;
    fn message_store(&self) -> Result<Vec<PropertyContext>, OutlookError>;
    fn name_id_map(&self) -> Result<Vec<PropertyContext>, OutlookError>;
    fn root_folder(&mut self, ntfs_file: Option<&NtfsFile<'_>>)
        -> Result<FolderInfo, OutlookError>;
    fn read_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderInfo, OutlookError>;
    fn search_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderInfo, OutlookError>;
    fn folder_metadata(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderMeta, OutlookError>;
}

impl<T: std::io::Seek + std::io::Read> OutlookReaderAction<T> for OutlookReader<T> {
    /// Get Block and Node BTrees and determine Outlook format type
    fn setup(&mut self, ntfs_file: Option<&NtfsFile<'_>>) -> Result<(), OutlookError> {
        let ost_size = 564;
        let header_bytes = read_bytes(&0, ost_size, ntfs_file, &mut self.fs).unwrap();
        let (_, header) = parse_header(&header_bytes).unwrap();

        self.format = header.format_type;
        self.size = match self.format {
            FormatType::ANSI32 | FormatType::Unicode64 => 512,
            FormatType::Unicode64_4k => 4096,
            FormatType::Unknown => panic!("should not be possible"),
        };

        let mut block_tree = Vec::new();
        get_block_btree(
            ntfs_file,
            &mut self.fs,
            &header.block_btree_root,
            &self.size,
            &self.format,
            &mut block_tree,
        )?;

        self.block_btree = block_tree;

        let mut node_tree = Vec::new();
        get_node_btree(
            ntfs_file,
            &mut self.fs,
            &header.node_btree_root,
            &self.size,
            &self.format,
            &mut node_tree,
            None,
        )?;

        self.node_btree = node_tree;
        Ok(())
    }

    /// Get block data for a specific Block
    fn get_block_data(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block: &LeafBlockData,
        descriptor: Option<&LeafBlockData>,
    ) -> Result<BlockValue, OutlookError> {
        self.parse_blocks(ntfs_file, &block, descriptor)
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
    fn root_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<FolderInfo, OutlookError> {
        /*
         * Steps:
         * 1. Get static node ID value (290) from node_btree
         * 2. Parse block data
         * 3. Parse PropertyContext and TableContext
         * 4. Parse the other nodes with the ID 290
         */
        let root = 290;
        self.read_folder(ntfs_file, root)
    }

    /// Read a folder and get its details. Use `root_folder` if you do not know a folder number
    fn read_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderInfo, OutlookError> {
        /*
         * Steps:
         * 1. Get folder node_id_num. Requires 4 node_ids
         * 2. Parse block data
         * 3. Parse PropertyContext and TableContext
         * 4. Parse the other nodes with the folder number (4 total)
         * 5. Call folder_details()
         */

        let mut leaf_descriptor = None;

        let mut normal = LeafNodeData {
            node: Node {
                node_id: NodeID::InternalNode,
                node_id_num: 0,
                node: 0,
            },
            block_offset_data_id: 0,
            block_offset_descriptor_id: 0,
            parent_node_index: 0,
        };

        let mut hierahcy = normal.clone();
        let mut contents = normal.clone();
        let mut fai = normal.clone();

        let search = vec![
            NodeID::SearchFolder,
            NodeID::SearchContentsTable,
            NodeID::SearchUpdateQueue,
            NodeID::SearchCriteria,
        ];

        let mut folder_number = folder;
        let mut peek_nodes = self.node_btree.iter().peekable();

        while let Some(nodes) = peek_nodes.next() {
            if let Some(id) = nodes.btree.get(&(folder_number as u32)) {
                let node_number = id.node.node_id_num;

                for node in nodes.btree.values() {
                    if node.node.node_id_num != node_number {
                        continue;
                    }
                    if node.node.node_id == NodeID::NormalFolder {
                        normal = node.clone();
                    } else if node.node.node_id == NodeID::HierarchyTable {
                        hierahcy = node.clone();
                    } else if node.node.node_id == NodeID::ContentsTable {
                        contents = node.clone();
                    } else if node.node.node_id == NodeID::FaiContentsTable {
                        fai = node.clone();
                    } else if node.node.node_id == NodeID::Unknown {
                        warn!("[outlook] Unknown NodeID: {node:?}");
                        continue;
                    } else if search.contains(&node.node.node_id) {
                        return self.search_folder(ntfs_file, folder);
                    } else {
                        panic!("other optoin!?: {node:?}");
                    }
                }
                if normal.block_offset_data_id != 0
                    && fai.block_offset_data_id != 0
                    && hierahcy.block_offset_data_id != 0
                    && contents.block_offset_data_id != 0
                {
                    break;
                }

                // Ugh not all the folders were in the same Branch!
                // If this happens the start of the next branch should contain the remaining folders
                // We peek to get the folder number for the branch which should be associated with the folder we want

                /* Ex:
                 * Here is a folder at the end of a Branch
                 * my node: LeafNodeData { node: Node { node_id: NormalFolder, node_id_num: 270, node: 8642 }, block_offset_data_id: 66548, block_offset_descriptor_id: 66558, parent_node_index: 8514 }
                 *
                 * Here is the next branch. The three (3) LeafNodeData values belong with the LeafNodeData above. Note the `node_id_num` values are all the same
                 * The branch `node` value matches the `node` value of the first LeafNodeData in the branch
                 * branch: BranchData { node: Node { node_id: HierarchyTable, node_id_num: 270, node: 8653 }, back_pointer: 21896, offset: 20832256 }
                 * my node: LeafNodeData { node: Node { node_id: HierarchyTable, node_id_num: 270, node: 8653 }, block_offset_data_id: 4, block_offset_descriptor_id: 0, parent_node_index: 0 }
                 * my node: LeafNodeData { node: Node { node_id: ContentsTable, node_id_num: 270, node: 8654 }, block_offset_data_id: 6132, block_offset_descriptor_id: 22, parent_node_index: 0 }
                 * my node: LeafNodeData { node: Node { node_id: FaiContentsTable, node_id_num: 270, node: 8655 }, block_offset_data_id: 6120, block_offset_descriptor_id: 0, parent_node_index: 0 }
                 */
                if let Some(next_branch) = peek_nodes.peek() {
                    // Next folder number should contain the NodeID number associated with the remaing folders we need
                    folder_number = next_branch.branch_node as u64;
                }
            }
        }

        let mut leaf_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };

        let mut hierarchy_block = leaf_block.clone();
        let mut hiearchy_descriptor = None;
        let mut contents_block = leaf_block.clone();
        let mut contents_descriptor = None;
        let mut fai_block = leaf_block.clone();
        let mut fai_descriptor = None;

        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&normal.block_offset_data_id) {
                leaf_block = block_data.clone();
            }
            if normal.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&normal.block_offset_descriptor_id) {
                    leaf_descriptor = Some(block_data.clone());
                }
            }

            if let Some(block_data) = blocks.get(&hierahcy.block_offset_data_id) {
                hierarchy_block = block_data.clone();
            }
            if hierahcy.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&hierahcy.block_offset_descriptor_id) {
                    hiearchy_descriptor = Some(block_data.clone());
                }
            }

            if let Some(block_data) = blocks.get(&contents.block_offset_data_id) {
                contents_block = block_data.clone();
            }
            if contents.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&contents.block_offset_descriptor_id) {
                    contents_descriptor = Some(block_data.clone());
                }
            }

            if let Some(block_data) = blocks.get(&fai.block_offset_data_id) {
                fai_block = block_data.clone();
            }
            if fai.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&fai.block_offset_descriptor_id) {
                    fai_descriptor = Some(block_data.clone());
                }
            }

            if leaf_block.index != 0
                && fai_block.index != 0
                && contents_block.index != 0
                && hierarchy_block.index != 0
            {
                break;
            }
        }

        let normal_value = self.get_block_data(ntfs_file, &leaf_block, leaf_descriptor.as_ref())?;
        let (_, normal_result) = self
            .parse_property_context(&normal_value.data, &normal_value.descriptors)
            .unwrap();

        let hiearchy_value =
            self.get_block_data(None, &hierarchy_block, hiearchy_descriptor.as_ref())?;
        let (_, hiearhy_result) = self
            .parse_table_context(&hiearchy_value.data, &hiearchy_value.descriptors)
            .unwrap();

        let content_value =
            self.get_block_data(None, &contents_block, contents_descriptor.as_ref())?;
        let (_, contents_result) = self
            .parse_table_context(&content_value.data, &content_value.descriptors)
            .unwrap();

        let fai_value = self.get_block_data(None, &fai_block, fai_descriptor.as_ref())?;
        let (_, fai_result) = self
            .parse_table_context(&fai_value.data, &fai_value.descriptors)
            .unwrap();

        let result = folder_details(
            &normal_result,
            &hiearhy_result,
            &contents_result,
            &fai_result,
        );

        Ok(result)
    }

    /// Read a special "Serch Folder" folder type. This function does **NO** searching. You should use `read_folder` if you are iterating through the OST file.
    /// It will call this function automatically if it encounters a "Search Folder"
    fn search_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderInfo, OutlookError> {
        let mut search = LeafNodeData {
            node: Node {
                node_id: NodeID::InternalNode,
                node_id_num: 0,
                node: 0,
            },
            block_offset_data_id: 0,
            block_offset_descriptor_id: 0,
            parent_node_index: 0,
        };

        let mut criteria = search.clone();
        let mut contents = search.clone();
        // let mut update = search.clone();

        let mut folder_number = folder;
        let mut peek_nodes = self.node_btree.iter().peekable();

        while let Some(nodes) = peek_nodes.next() {
            if let Some(id) = nodes.btree.get(&(folder_number as u32)) {
                let node_number = id.node.node_id_num;

                for node in nodes.btree.values() {
                    if node.node.node_id_num != node_number {
                        continue;
                    }
                    if node.node.node_id == NodeID::SearchFolder {
                        search = node.clone();
                    } else if node.node.node_id == NodeID::SearchCriteria {
                        criteria = node.clone();
                    } else if node.node.node_id == NodeID::SearchContentsTable {
                        contents = node.clone();
                    } else if node.node.node_id == NodeID::SearchUpdateQueue {
                        // update = node.clone();
                        println!(
                            "Got update queue. Unsure whether property or table context: {node:?}"
                        );
                        continue;
                    } else if node.node.node_id == NodeID::Unknown {
                        warn!("[outlook] Unknown NodeID: {node:?}");
                        continue;
                    } else {
                        panic!("other optoin!?: {node:?}");
                    }
                }
                if search.block_offset_data_id != 0
                    && criteria.block_offset_data_id != 0
                    && contents.block_offset_data_id != 0
                {
                    break;
                }

                // Ugh not all the folders were in the same Branch!
                // If this happens the start of the next branch should contain the remaining folders
                // We peek to get the folder number for the branch which should be associated with the folder we want

                /* Ex:
                 * Here is a folder at the end of a Branch
                 * my node: LeafNodeData { node: Node { node_id: NormalFolder, node_id_num: 270, node: 8642 }, block_offset_data_id: 66548, block_offset_descriptor_id: 66558, parent_node_index: 8514 }
                 *
                 * Here is the next branch. The three (3) LeafNodeData values belong with the LeafNodeData above. Note the `node_id_num` values are all the same
                 * The branch `node` value matches the `node` value of the first LeafNodeData in the branch
                 * branch: BranchData { node: Node { node_id: HierarchyTable, node_id_num: 270, node: 8653 }, back_pointer: 21896, offset: 20832256 }
                 * my node: LeafNodeData { node: Node { node_id: HierarchyTable, node_id_num: 270, node: 8653 }, block_offset_data_id: 4, block_offset_descriptor_id: 0, parent_node_index: 0 }
                 * my node: LeafNodeData { node: Node { node_id: ContentsTable, node_id_num: 270, node: 8654 }, block_offset_data_id: 6132, block_offset_descriptor_id: 22, parent_node_index: 0 }
                 * my node: LeafNodeData { node: Node { node_id: FaiContentsTable, node_id_num: 270, node: 8655 }, block_offset_data_id: 6120, block_offset_descriptor_id: 0, parent_node_index: 0 }
                 */
                if let Some(next_branch) = peek_nodes.peek() {
                    // Next folder number should contain the NodeID number associated with the remaing folders we need
                    folder_number = next_branch.branch_node as u64;
                }
            }
        }

        let mut search_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };
        let mut search_descriptor = None;

        let mut criteria_block = search_block.clone();
        let mut criteria_descriptor = None;
        let mut contents_block = search_block.clone();
        let mut contents_descriptor = None;
        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&search.block_offset_data_id) {
                search_block = block_data.clone();
            }
            if search.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&search.block_offset_descriptor_id) {
                    search_descriptor = Some(block_data.clone());
                }
            }

            if let Some(block_data) = blocks.get(&criteria.block_offset_data_id) {
                criteria_block = block_data.clone();
            }
            if criteria.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&criteria.block_offset_descriptor_id) {
                    criteria_descriptor = Some(block_data.clone());
                }
            }

            if let Some(block_data) = blocks.get(&contents.block_offset_data_id) {
                contents_block = block_data.clone();
            }
            if contents.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&contents.block_offset_descriptor_id) {
                    contents_descriptor = Some(block_data.clone());
                }
            }

            if search_block.index != 0 && contents_block.index != 0 && criteria_block.index != 0 {
                break;
            }
        }

        let search_value =
            self.get_block_data(ntfs_file, &search_block, search_descriptor.as_ref())?;
        let (_, search_result) = self
            .parse_property_context(&search_value.data, &search_value.descriptors)
            .unwrap();

        let criteria_value =
            self.get_block_data(None, &criteria_block, criteria_descriptor.as_ref())?;
        let (_, criteria_result) = self
            .parse_property_context(&criteria_value.data, &criteria_value.descriptors)
            .unwrap();

        let content_value =
            self.get_block_data(None, &contents_block, contents_descriptor.as_ref())?;
        let (_, contents_result) = self
            .parse_table_context(&content_value.data, &content_value.descriptors)
            .unwrap();

        let result = search_folder_details(&search_result, &criteria_result, &contents_result);
        Ok(result)
    }

    fn folder_metadata(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderMeta, OutlookError> {
        let mut info = LeafNodeData {
            node: Node {
                node_id: NodeID::InternalNode,
                node_id_num: 0,
                node: 0,
            },
            block_offset_data_id: 0,
            block_offset_descriptor_id: 0,
            parent_node_index: 0,
        };

        let mut folder_number = folder;
        let mut peek_nodes = self.node_btree.iter().peekable();

        while let Some(nodes) = peek_nodes.next() {
            if let Some(id) = nodes.btree.get(&(folder_number as u32)) {
                let node_number = id.node.node_id_num;

                for node in nodes.btree.values() {
                    if node.node.node_id_num != node_number {
                        continue;
                    }
                    if node.node.node_id == NodeID::FolderAssociatedInfo {
                        info = node.clone();
                    }
                }
                if info.block_offset_data_id != 0 {
                    break;
                }
            }
        }

        let mut info_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };
        let mut info_descriptor = None;
        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&info.block_offset_data_id) {
                info_block = block_data.clone();
            }
            if info.block_offset_descriptor_id != 0 {
                if let Some(block_data) = blocks.get(&info.block_offset_descriptor_id) {
                    info_descriptor = Some(block_data.clone());
                }
            }
        }

        let info_value = self.get_block_data(ntfs_file, &info_block, info_descriptor.as_ref())?;
        let (_, info_result) = self
            .parse_property_context(&info_value.data, &info_value.descriptors)
            .unwrap();

        let meta = extract_fai(&info_result);

        Ok(meta)
    }
}

#[cfg(test)]
mod tests {
    use super::{OutlookReader, OutlookReaderAction};
    use crate::{
        artifacts::os::windows::outlook::header::FormatType, filesystem::files::file_reader,
    };
    use std::io::BufReader;

    fn stream_ost<T: std::io::Seek + std::io::Read>(reader: &mut OutlookReader<T>, folder: &u64) {
        let results = reader.read_folder(None, *folder).unwrap();

        println!("My Folder name: {}", results.name);
        for meta in results.associated_content {
            println!(
                "Getting additional metadata for {} under: {:?}",
                results.name, meta
            );
            let meta_value = reader.folder_metadata(None, meta.node).unwrap();
            println!("Meta: {meta_value:?}");
        }

        for sub in results.subfolders {
            println!("getting info for sub folder: {:?}", sub);
            stream_ost(reader, &sub.node);
        }
    }

    #[test]
    fn test_outlook_reader() {
        // We need an OST file for this test
        let reader =
            file_reader("C:\\Users\\bob\\Desktop\\azur3m3m1crosoft@outlook.com.ost").unwrap();
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        outlook_reader.setup(None).unwrap();
        stream_ost(&mut outlook_reader, &290);
    }
}
