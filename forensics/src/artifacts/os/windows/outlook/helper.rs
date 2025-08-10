use super::{
    blocks::block::{BlockValue, OutlookBlock},
    error::OutlookError,
    header::{FormatType, Node, NodeID, parse_header},
    items::{
        attachment::{Attachment, extract_attachment},
        fai::{FolderMeta, extract_fai},
        folder::{FolderInfo, folder_details, search_folder_details},
        message::MessageDetails,
        name_map::{NameEntry, extract_name_id_map},
    },
    pages::btree::{
        BlockType, LeafBlockData, LeafNodeData, NodeBtree, get_block_btree, get_node_btree,
    },
    tables::{
        context::{TableBranchInfo, TableInfo, TableRows},
        property::OutlookPropertyContext,
    },
};
use crate::{
    artifacts::os::windows::outlook::{
        items::message::{message_details, recipients, table_message_preview},
        tables::context::OutlookTableContext,
    },
    filesystem::ntfs::reader::read_bytes,
};
use common::windows::PropertyContext;
use log::{error, warn};
use ntfs::NtfsFile;
use std::{
    collections::{BTreeMap, HashMap},
    io::BufReader,
};

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
    fn message_store(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<Vec<PropertyContext>, OutlookError>;
    fn name_id_map(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<HashMap<u16, NameEntry>, OutlookError>;
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
    fn read_message(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        info: &TableInfo,
        branch: Option<&TableBranchInfo>,
    ) -> Result<Vec<MessageDetails>, OutlookError>;
    fn recipient_table(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_data_id: u64,
        block_descriptor_id: u64,
    ) -> Result<Vec<Vec<TableRows>>, OutlookError>;
    fn read_attachment(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_data_id: u64,
        block_descriptor_id: u64,
    ) -> Result<Attachment, OutlookError>;
}

impl<T: std::io::Seek + std::io::Read> OutlookReaderAction<T> for OutlookReader<T> {
    /// Get Block and Node `BTrees` and determine Outlook format type
    fn setup(&mut self, ntfs_file: Option<&NtfsFile<'_>>) -> Result<(), OutlookError> {
        let ost_size = 564;
        let header_results = read_bytes(0, ost_size, ntfs_file, &mut self.fs);
        let header_bytes = match header_results {
            Ok(result) => result,
            Err(err) => {
                error!("[outlook] Could not read header bytes: {err:?}");
                return Err(OutlookError::ReadFile);
            }
        };
        let header_result = parse_header(&header_bytes);
        let (_, header) = match header_result {
            Ok(result) => result,
            Err(err) => {
                error!("[outlook] Could not parse header: {err:?}");
                return Err(OutlookError::Header);
            }
        };

        self.format = header.format_type;
        self.size = match self.format {
            FormatType::ANSI32 | FormatType::Unicode64 => 512,
            FormatType::Unicode64_4k => 4096,
            FormatType::Unknown => return Err(OutlookError::UnknownPageFormat),
        };

        let mut block_tree = Vec::new();
        get_block_btree(
            ntfs_file,
            &mut self.fs,
            header.block_btree_root,
            self.size,
            &self.format,
            &mut block_tree,
        )?;

        self.block_btree = block_tree;

        let mut node_tree = Vec::new();
        get_node_btree(
            ntfs_file,
            &mut self.fs,
            header.node_btree_root,
            self.size,
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
        if block.block_offset == 0 && block.size == 0 {
            error!(
                "[outlook] Got offset and size value of 0. Cannot parse blocks with these values."
            );
            return Err(OutlookError::NoBlocks);
        }
        self.parse_blocks(ntfs_file, block, descriptor)
    }

    /// Extract the Outlook `MessageStore`
    fn message_store(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<Vec<PropertyContext>, OutlookError> {
        let store = 33;
        let mut node: Option<&LeafNodeData> = None;
        for entry in &self.node_btree {
            if let Some(entry) = entry.btree.get(&store) {
                node = Some(entry);
                break;
            }
        }

        if node.is_none() {
            error!("[outlook] Could not find Message Store node");
            return Err(OutlookError::PropertyContext);
        }

        let mut block: Option<LeafBlockData> = None;
        for blocks in &self.block_btree {
            if let Some(entry) = blocks.get(&node.unwrap().block_offset_data_id) {
                block = Some(*entry);
                break;
            }
        }

        if block.is_none() {
            error!("[outlook] Could not find Message Store block");
            return Err(OutlookError::PropertyContext);
        }

        let store_value = self.get_block_data(ntfs_file, &block.unwrap(), None)?;
        let message_store =
            self.parse_property_context(ntfs_file, &store_value.data, &store_value.descriptors)?;

        Ok(message_store)
    }

    /// Extract the Outlook `NameToIdMap`
    fn name_id_map(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<HashMap<u16, NameEntry>, OutlookError> {
        let map = 97;
        let mut node: Option<&LeafNodeData> = None;
        for entry in &self.node_btree {
            if let Some(entry) = entry.btree.get(&map) {
                node = Some(entry);
                break;
            }
        }

        if node.is_none() {
            error!("[outlook] Could not find Name Map node");
            return Err(OutlookError::PropertyContext);
        }

        let mut block: Option<LeafBlockData> = None;
        let mut leaf_descriptor: Option<LeafBlockData> = None;

        for blocks in &self.block_btree {
            if let Some(entry) = blocks.get(&node.unwrap().block_offset_data_id) {
                block = Some(*entry);
            }
            if let Some(block_data) = blocks.get(&node.unwrap().block_offset_descriptor_id) {
                leaf_descriptor = Some(*block_data);
            }

            if leaf_descriptor.is_some() && block.is_some() {
                break;
            }
        }

        if block.is_none() || leaf_descriptor.is_none() {
            error!("[outlook] Could not find Name Map block");
            return Err(OutlookError::PropertyContext);
        }

        let map_value =
            self.get_block_data(ntfs_file, &block.unwrap(), leaf_descriptor.as_ref())?;
        let name_map =
            self.parse_property_context(ntfs_file, &map_value.data, &map_value.descriptors)?;

        extract_name_id_map(&name_map)
    }

    /// Get the Outlook Root folder. Starting point to get the contents of Outlook
    fn root_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
    ) -> Result<FolderInfo, OutlookError> {
        let root = 290;
        self.read_folder(ntfs_file, root)
    }

    /// Read a folder and get its details. Use `root_folder` if you do not know any folder number
    fn read_folder(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        folder: u64,
    ) -> Result<FolderInfo, OutlookError> {
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

        let mut hierarchy = normal.clone();
        let mut contents = normal.clone();
        let mut fai = normal.clone();

        let search = [
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
                        hierarchy = node.clone();
                    } else if node.node.node_id == NodeID::ContentsTable {
                        contents = node.clone();
                    } else if node.node.node_id == NodeID::FaiContentsTable {
                        fai = node.clone();
                    } else if search.contains(&node.node.node_id) {
                        return self.search_folder(ntfs_file, folder);
                    } else if node.node.node_id != NodeID::ContentsTableIndex
                        && node.node.node_id != NodeID::Unknown
                    {
                        // ContentsTableIndex is undocumented may be internal to Outlook
                        warn!("[outlook] Unexpected NodeID for folder: {node:?}");
                    }
                }
                if normal.block_offset_data_id != 0
                    && fai.block_offset_data_id != 0
                    && hierarchy.block_offset_data_id != 0
                    && contents.block_offset_data_id != 0
                {
                    break;
                }

                // Ugh not all the folders were in the same Branch!
                // If this happens the start of the next branch should contain the remaining folders
                // We peek to get the folder number for the branch which should be associated with the folder we want

                /* Ex:
                 * Here is a folder at the end of a Branch
                 * node: LeafNodeData { node: Node { node_id: NormalFolder, node_id_num: 270, node: 8642 }, block_offset_data_id: 66548, block_offset_descriptor_id: 66558, parent_node_index: 8514 }
                 *
                 * Here is the next branch. The three (3) LeafNodeData values belong with the LeafNodeData above. Note the `node_id_num` values are all the same
                 * The branch `node` value matches the `node` value of the first LeafNodeData in the branch
                 * branch: BranchData { node: Node { node_id: HierarchyTable, node_id_num: 270, node: 8653 }, back_pointer: 21896, offset: 20832256 }
                 * node: LeafNodeData { node: Node { node_id: HierarchyTable, node_id_num: 270, node: 8653 }, block_offset_data_id: 4, block_offset_descriptor_id: 0, parent_node_index: 0 }
                 * node: LeafNodeData { node: Node { node_id: ContentsTable, node_id_num: 270, node: 8654 }, block_offset_data_id: 6132, block_offset_descriptor_id: 22, parent_node_index: 0 }
                 * node: LeafNodeData { node: Node { node_id: FaiContentsTable, node_id_num: 270, node: 8655 }, block_offset_data_id: 6120, block_offset_descriptor_id: 0, parent_node_index: 0 }
                 */
                if let Some(next_branch) = peek_nodes.peek() {
                    // Next folder number should contain the NodeID number associated with the remaining folders we need
                    folder_number = next_branch.branch_node as u64;
                }
            }
        }
        check_node(&normal)?;

        let mut leaf_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };

        let mut hierarchy_block = leaf_block;
        let mut hierarchy_descriptor = None;
        let mut contents_block = leaf_block;
        let mut contents_descriptor = None;
        let mut fai_block = leaf_block;
        let mut fai_descriptor = None;

        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&normal.block_offset_data_id) {
                leaf_block = *block_data;
            }
            if normal.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&normal.block_offset_descriptor_id)
            {
                leaf_descriptor = Some(*block_data);
            }

            if let Some(block_data) = blocks.get(&hierarchy.block_offset_data_id) {
                hierarchy_block = *block_data;
            }
            if hierarchy.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&hierarchy.block_offset_descriptor_id)
            {
                hierarchy_descriptor = Some(*block_data);
            }

            if let Some(block_data) = blocks.get(&contents.block_offset_data_id) {
                contents_block = *block_data;
            }
            if contents.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&contents.block_offset_descriptor_id)
            {
                contents_descriptor = Some(*block_data);
            }

            if let Some(block_data) = blocks.get(&fai.block_offset_data_id) {
                fai_block = *block_data;
            }
            if fai.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&fai.block_offset_descriptor_id)
            {
                fai_descriptor = Some(*block_data);
            }

            if leaf_block.index != 0
                && fai_block.index != 0
                && contents_block.index != 0
                && hierarchy_block.index != 0
                && fai.block_offset_descriptor_id == 0
                && contents.block_offset_descriptor_id == 0
                && hierarchy.block_offset_descriptor_id == 0
                && normal.block_offset_descriptor_id == 0
            {
                // We can stop early if none of the items associated with a folder have descriptors
                break;
            }
        }

        let normal_value = self.get_block_data(ntfs_file, &leaf_block, leaf_descriptor.as_ref())?;
        let normal =
            self.parse_property_context(ntfs_file, &normal_value.data, &normal_value.descriptors)?;

        let hierarchy_value =
            self.get_block_data(ntfs_file, &hierarchy_block, hierarchy_descriptor.as_ref())?;

        // Hierarchy table contains info on nested sub-folders
        let mut hierarchy_info =
            self.table_info(&hierarchy_value.data, &hierarchy_value.descriptors)?;

        // We get all sub-folders. The data is not that large
        let rows_to_get = (0..hierarchy_info.total_rows).collect();
        hierarchy_info.rows = rows_to_get;
        let hierarchy_rows = self.get_rows(&hierarchy_info, ntfs_file)?;
        let content_value =
            self.get_block_data(ntfs_file, &contents_block, contents_descriptor.as_ref())?;

        // Contents contains **a lot** of metadata about the email content. Since we do not know how emails are in the OST. We just return info required to start parsing emails
        // And let the caller determine how many to parse at once
        let contents_info = self.table_info(&content_value.data, &content_value.descriptors)?;

        let fai_value = self.get_block_data(ntfs_file, &fai_block, fai_descriptor.as_ref())?;
        // FAI table contains preview info on extra folder metadata
        let mut fai_info = self.table_info(&fai_value.data, &fai_value.descriptors)?;

        // We get all FAI metadata. The data is not that large
        let rows_to_get = (0..fai_info.total_rows).collect();
        fai_info.rows = rows_to_get;
        let fai_rows = self.get_rows(&fai_info, ntfs_file)?;

        let result = folder_details(&normal, &hierarchy_rows, &contents_info, &fai_rows);

        Ok(result)
    }

    /// Read a special "Search Folder" folder type. This function does **NO** searching. You should use `read_folder` if you are iterating through the OST file.
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
                    } else if node.node.node_id != NodeID::SearchContentsTable
                        && node.node.node_id != NodeID::SearchUpdateQueue
                        && node.node.node_id != NodeID::Unknown
                    {
                        warn!("[outlook] Unexpected NodeID for search folder: {node:?}");
                    }
                }
                if search.block_offset_data_id != 0 && criteria.block_offset_data_id != 0 {
                    break;
                }

                // Ugh not all the folders were in the same Branch!
                // If this happens the start of the next branch should contain the remaining folders
                // We peek to get the folder number for the branch which should be associated with the folder we want
                if let Some(next_branch) = peek_nodes.peek() {
                    // Next folder number should contain the NodeID number associated with the remaining folders we need
                    folder_number = next_branch.branch_node as u64;
                }
            }
        }

        check_node(&search)?;

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

        let mut criteria_block = search_block;
        let mut criteria_descriptor = None;

        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&search.block_offset_data_id) {
                search_block = *block_data;
            }
            if search.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&search.block_offset_descriptor_id)
            {
                search_descriptor = Some(*block_data);
            }

            if let Some(block_data) = blocks.get(&criteria.block_offset_data_id) {
                criteria_block = *block_data;
            }
            if criteria.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&criteria.block_offset_descriptor_id)
            {
                criteria_descriptor = Some(*block_data);
            }

            if search_block.index != 0 && criteria_block.index != 0 {
                break;
            }
        }

        let search_value =
            self.get_block_data(ntfs_file, &search_block, search_descriptor.as_ref())?;
        let search_result =
            self.parse_property_context(ntfs_file, &search_value.data, &search_value.descriptors)?;

        let criteria_value =
            self.get_block_data(ntfs_file, &criteria_block, criteria_descriptor.as_ref())?;
        let criteria_result = self.parse_property_context(
            ntfs_file,
            &criteria_value.data,
            &criteria_value.descriptors,
        )?;

        let result = search_folder_details(&search_result, &criteria_result);
        Ok(result)
    }

    /// Get additional folder metadata by parsing the FAI data
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

        for nodes in &self.node_btree {
            if let Some(id) = nodes.btree.get(&(folder as u32)) {
                let node_number = id.node.node_id_num;

                for node in nodes.btree.values() {
                    if node.node.node_id_num != node_number {
                        continue;
                    }
                    if node.node.node_id == NodeID::FolderAssociatedInfo {
                        info = node.clone();
                        break;
                    }
                }
                if info.block_offset_data_id != 0 {
                    break;
                }
            }
        }

        check_node(&info)?;

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
                info_block = *block_data;
            }
            if info.block_offset_descriptor_id != 0
                && let Some(block_data) = blocks.get(&info.block_offset_descriptor_id)
            {
                info_descriptor = Some(*block_data);
            }
        }

        let info_value = self.get_block_data(ntfs_file, &info_block, info_descriptor.as_ref())?;
        let info =
            self.parse_property_context(ntfs_file, &info_value.data, &info_value.descriptors)?;
        let meta = extract_fai(&info);

        Ok(meta)
    }

    /// Read and get info on recipient table
    fn recipient_table(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_data_id: u64,
        block_descriptor_id: u64,
    ) -> Result<Vec<Vec<TableRows>>, OutlookError> {
        let mut table_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };
        let mut table_descriptor = None;
        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&block_data_id) {
                table_block = *block_data;
            }
            if block_descriptor_id != 0
                && let Some(block_data) = blocks.get(&block_descriptor_id)
            {
                table_descriptor = Some(*block_data);
            }
        }

        let table_value =
            self.get_block_data(ntfs_file, &table_block, table_descriptor.as_ref())?;
        let mut table_info = self.table_info(&table_value.data, &table_value.descriptors)?;

        let rows_to_get = (0..table_info.total_rows).collect();
        table_info.rows = rows_to_get;

        self.get_rows(&table_info, ntfs_file)
    }

    /// Read and extract email
    fn read_message(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        info: &TableInfo,
        branch: Option<&TableBranchInfo>,
    ) -> Result<Vec<MessageDetails>, OutlookError> {
        if info.rows.len() > info.total_rows as usize {
            warn!(
                "[outlook] Caller asked for too many messages. Caller asked for {} messages. But there are only {} available. We will return {}",
                info.rows.len(),
                info.total_rows,
                info.total_rows
            );
            // return Err(OutlookError::MessageCount);
        }
        // First we parse the table that points to our messages
        // The number of messages is dependent on many the caller wants to get
        let table_meta = if branch.is_none() {
            self.get_rows(info, ntfs_file)?
        } else {
            // Unwrap is ok here since we check above if it is none
            self.get_branch_rows(ntfs_file, info, branch.unwrap())?
        };

        let table_info = table_message_preview(&table_meta);
        let mut mess = LeafNodeData {
            node: Node {
                node_id: NodeID::InternalNode,
                node_id_num: 0,
                node: 0,
            },
            block_offset_data_id: 0,
            block_offset_descriptor_id: 0,
            parent_node_index: 0,
        };

        let mut messages = Vec::new();
        // Loop through each message we want
        for info in &table_info {
            // Search until we find the Message node in the BTree
            for nodes in &self.node_btree {
                if let Some(id) = nodes.btree.get(&(info.node as u32)) {
                    let node_number = id.node.node_id_num;

                    for node in nodes.btree.values() {
                        if node.node.node_id_num != node_number {
                            continue;
                        }
                        if node.node.node_id == NodeID::Message {
                            mess = node.clone();
                            break;
                        }
                    }
                    if mess.block_offset_data_id != 0 {
                        break;
                    }
                }
            }

            let status = check_node(&mess);
            if status.is_err() {
                continue;
            }

            let mut mess_block = LeafBlockData {
                block_type: BlockType::Internal,
                index_id: 0,
                index: 0,
                block_offset: 0,
                size: 0,
                total_size: 0,
                reference_count: 0,
            };
            let mut mess_descriptor = None;
            // Search until we find the Block data in the BTree that contains the message data
            for blocks in self.block_btree.iter() {
                if let Some(block_data) = blocks.get(&mess.block_offset_data_id) {
                    mess_block = *block_data;
                }
                if mess.block_offset_descriptor_id != 0
                    && let Some(block_data) = blocks.get(&mess.block_offset_descriptor_id)
                {
                    mess_descriptor = Some(*block_data);
                }
            }

            let mess_value =
                self.get_block_data(ntfs_file, &mess_block, mess_descriptor.as_ref())?;
            let mut message =
                self.parse_property_context(ntfs_file, &mess_value.data, &mess_value.descriptors)?;

            let mut recipient_block_id = 0;
            let mut recipient_block_descriptors = 0;

            let mut attach = Vec::new();
            for value in mess_value.descriptors.values() {
                if value.node.node_id == NodeID::RecipientTable {
                    recipient_block_id = value.block_data_id;
                    recipient_block_descriptors = value.block_descriptor_id;
                } else if value.node.node_id == NodeID::AttachmentTable {
                    attach.push((value.block_data_id, value.block_descriptor_id));
                }
            }

            let mut attach_rows = Vec::new();

            // Get attachment previews if we have any
            for (block_id, descriptor_id) in attach {
                let mut table_block = LeafBlockData {
                    block_type: BlockType::Internal,
                    index_id: 0,
                    index: 0,
                    block_offset: 0,
                    size: 0,
                    total_size: 0,
                    reference_count: 0,
                };
                let mut table_descriptor = None;
                for blocks in self.block_btree.iter() {
                    if let Some(block_data) = blocks.get(&block_id) {
                        table_block = *block_data;
                    }
                    if descriptor_id != 0
                        && let Some(block_data) = blocks.get(&descriptor_id)
                    {
                        table_descriptor = Some(*block_data);
                    }
                }

                // If still zero the block_data_id may be off by 1.
                // This may be a 0 vs 1 issue when determining the "first" number?
                if table_block.block_offset == 0 && table_block.size == 0 {
                    let adjust = 1;
                    for blocks in &self.block_btree {
                        if let Some(block_data) = blocks.get(&(block_id - adjust)) {
                            table_block = *block_data;

                            if descriptor_id == 0 {
                                break;
                            }
                        }
                        if descriptor_id != 0
                            && let Some(block_data) = blocks.get(&(descriptor_id - adjust))
                        {
                            table_descriptor = Some(*block_data);
                        }
                    }
                }

                let table_value =
                    self.get_block_data(ntfs_file, &table_block, table_descriptor.as_ref())?;

                let mut attach_info =
                    self.table_info(&table_value.data, &table_value.descriptors)?;
                // We get all attachment preview metadata. The data is not that large
                let rows_to_get = (0..attach_info.total_rows).collect();
                attach_info.rows = rows_to_get;

                let mut rows = self.get_rows(&attach_info, ntfs_file).unwrap();

                attach_rows.append(&mut rows);
            }

            let mut details = message_details(&mut message, &attach_rows, &mess_value.descriptors);

            let mut recipient_rows = Vec::new();
            // Get Recipient data if we have any
            if recipient_block_id != 0 {
                let table = self.recipient_table(
                    ntfs_file,
                    recipient_block_id,
                    recipient_block_descriptors,
                )?;
                recipient_rows = table;
            }
            details.recipients = recipients(&recipient_rows);
            messages.push(details);
        }

        Ok(messages)
    }

    /// Read and extract email attachment
    fn read_attachment(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_data_id: u64,
        block_descriptor_id: u64,
    ) -> Result<Attachment, OutlookError> {
        let mut table_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };
        let mut table_descriptor = None;
        for blocks in self.block_btree.iter() {
            if let Some(block_data) = blocks.get(&block_data_id) {
                table_block = *block_data;
            }
            if block_descriptor_id != 0
                && let Some(block_data) = blocks.get(&block_descriptor_id)
            {
                table_descriptor = Some(*block_data);
            }
        }

        let table_value =
            self.get_block_data(ntfs_file, &table_block, table_descriptor.as_ref())?;
        let mut attachment =
            self.parse_property_context(ntfs_file, &table_value.data, &table_value.descriptors)?;

        Ok(extract_attachment(&mut attachment))
    }
}

/// Check to make the node info has been update. If we found data in the Node Btree the value should change from zero (0)
fn check_node(leaf: &LeafNodeData) -> Result<(), OutlookError> {
    let not_set = 0;
    if leaf.block_offset_data_id == not_set
        && leaf.block_offset_descriptor_id == not_set
        && leaf.parent_node_index as u64 == not_set
        && leaf.node.node as u64 == not_set
        && leaf.node.node_id_num == not_set
    {
        error!(
            "[outlook] Leaf node data has default values. Its likely the data was not found in the Node Btree"
        );
        return Err(OutlookError::LeafNode);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{OutlookReader, OutlookReaderAction};
    use crate::{
        artifacts::os::windows::outlook::{header::FormatType, items::message::AttachMethod},
        filesystem::files::file_reader,
    };
    use std::{io::BufReader, path::PathBuf};

    fn stream_ost<T: std::io::Seek + std::io::Read>(reader: &mut OutlookReader<T>, folder: u64) {
        let mut results = reader.read_folder(None, folder).unwrap();

        for meta in results.associated_content {
            let _meta_value = reader.folder_metadata(None, meta.node).unwrap();
        }

        if results.message_count > 5 && results.name == "Inbox" {
            // Get first 5 messages!
            let messages_to_get = (0..5).collect();
            results.messages_table.rows = messages_to_get;
            let messages = reader
                .read_message(None, &results.messages_table, None)
                .unwrap();

            assert_eq!(messages.len(), 5);
            assert_eq!(messages[0].delivered, "2024-09-10T04:14:19.000Z");
            assert_eq!(
                messages[0].subject,
                "     Get to know your OneDrive – How to back up your PC and mobile"
            );
            assert_eq!(messages[0].from, "Microsoft@notificationmail.microsoft.com");
            assert_eq!(messages[0].body.len(), 132324);
            assert_eq!(messages[0].props.len(), 120);
            assert_eq!(messages[0].recipient.len(), 32);

            // Check other messages
            for message in messages {
                assert!(!message.delivered.is_empty());
                assert!(!message.from.is_empty());
                assert!(!message.subject.is_empty());
                assert!(!message.body.is_empty());
                assert!(!message.props.is_empty());
                assert!(!message.recipient.is_empty());
            }
        }

        for sub in results.subfolders {
            stream_ost(reader, sub.node);
        }
    }

    fn setup_reader<T: std::io::Seek + std::io::Read>() -> OutlookReader<std::fs::File> {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/test@outlook.com.ost");

        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        outlook_reader.setup(None).unwrap();
        outlook_reader
    }

    #[test]
    fn test_outlook_reader_only() {
        let mut outlook_reader = setup_reader::<std::fs::File>();
        stream_ost(&mut outlook_reader, 290)
    }

    #[test]
    fn test_outlook_reader_root_folder() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let folder = outlook_reader.root_folder(None).unwrap();
        assert_eq!(folder.created, "2024-09-10T07:14:31.000Z");
        assert_eq!(folder.modified, "2024-09-10T07:14:31.000Z");
        assert_eq!(folder.subfolder_count, 2);
        assert_eq!(folder.subfolders[0].name, "Root - Public");
        assert_eq!(folder.subfolders[1].name, "Root - Mailbox");
        assert_eq!(folder.name, "");
        assert_eq!(folder.properties.len(), 12);
    }

    #[test]
    fn test_outlook_reader_read_folder() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let folder = outlook_reader.read_folder(None, 8610).unwrap();

        assert_eq!(folder.name, "Outbox");
        assert_eq!(folder.created, "2024-09-10T04:03:24.000Z");
        assert_eq!(folder.modified, "2024-09-10T07:14:50.000Z");
        assert_eq!(folder.properties.len(), 27);
        assert_eq!(folder.subfolder_count, 0);
        assert_eq!(folder.messages_table.columns.len(), 78);
    }

    #[test]
    fn test_outlook_reader_message_store() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let store = outlook_reader.message_store(None).unwrap();
        assert_eq!(store.len(), 23);
    }

    #[test]
    fn test_outlook_reader_name_map() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let store = outlook_reader.name_id_map(None).unwrap();
        assert_eq!(store.len(), 1276);
    }

    #[test]
    fn test_outlook_reader_read_attachment() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let attach = outlook_reader.read_attachment(None, 7592, 7586).unwrap();

        assert_eq!(attach.data.len(), 15320);
        assert_eq!(attach.extension, ".png");
        assert_eq!(attach.method, AttachMethod::ByValue);
        assert_eq!(attach.mime, "image/png");
        assert_eq!(attach.name, "wm-google-store.png");
        assert_eq!(attach.size, 11703);
        assert_eq!(attach.props.len(), 11);
    }

    #[test]
    fn test_outlook_reader_read_message() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let mut info = outlook_reader.read_folder(None, 8546).unwrap();
        info.messages_table.rows = vec![0];
        let mess = outlook_reader
            .read_message(None, &info.messages_table, None)
            .unwrap();

        assert_eq!(mess.len(), 1);
    }

    #[test]
    fn test_outlook_reader_recipient_table() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let table = outlook_reader.recipient_table(None, 36, 0).unwrap();
        assert!(table.is_empty());
    }

    #[test]
    fn test_outlook_reader_folder_metadata() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        let meta = outlook_reader.folder_metadata(None, 1048648).unwrap();

        assert_eq!(meta.message_class, "IPM.Microsoft.WunderBar.Link");
        assert_eq!(meta.properties.len(), 54);
    }

    #[test]
    #[should_panic(expected = "LeafNode")]
    fn test_outlook_reader_search_folder_details_bad() {
        let mut outlook_reader = setup_reader::<std::fs::File>();

        outlook_reader.folder_metadata(None, 99999).unwrap();
    }
}
