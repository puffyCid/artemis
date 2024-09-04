use crate::artifacts::os::windows::outlook::tables::{
    context::{TableContext, TableRows},
    properties::PropertyName,
    property::PropertyContext,
};

pub(crate) struct FolderInfo {
    /**Name of the folder */
    pub(crate) name: String,
    /**Timestamp when folder was created */
    pub(crate) created: String,
    /**Timestamp when folder was modified */
    pub(crate) modified: String,
    /**TableRows associated with the Hierarchy (subfolders) */
    hierarchy: Vec<Vec<TableRows>>,
    /**Folder Properties */
    pub(crate) properties: Vec<PropertyContext>,
    /**Number of children under the folder. May contain subfolders and/or messages */
    pub(crate) subfolders: Vec<SubFolder>,
    /**TableRows associated with FAI (FolderAssociatedInfo) */
    associated_content: Vec<Vec<TableRows>>,
    /**Number of subfolders */
    pub(crate) subfolder_count: usize,
    /**Number of messages or non-subfolder children */
    pub(crate) message_count: u64,
    pub(crate) messages: Vec<String>,
    //**Array of parent folders tracked*/
    //folders: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct SubFolder {
    pub(crate) name: String,
    pub(crate) node: u64,
}

pub(crate) fn folder_details(
    normal: &[PropertyContext],
    hierarchy: &TableContext,
    contents: &TableContext,
    fai: &TableContext,
) -> FolderInfo {
    //println!("normal: {normal:?}");

    let mut info = FolderInfo {
        name: String::new(),
        created: String::new(),
        modified: String::new(),
        hierarchy: Vec::new(),
        associated_content: Vec::new(),
        properties: Vec::new(),
        subfolders: Vec::new(),
        subfolder_count: 0,
        message_count: 0,
        messages: Vec::new(),
        // folders: Vec::new(),
    };

    for props in normal {
        if props.name.contains(&PropertyName::PidTagDisplayNameW) {
            info.name = props.value.as_str().unwrap_or_default().to_string();
            // info.folders.push(info.name);
        } else if props.name.contains(&PropertyName::PidTagCreationTime) {
            info.created = props.value.as_str().unwrap_or_default().to_string();
        } else if props
            .name
            .contains(&PropertyName::PidTagLastModificationTime)
        {
            info.modified = props.value.as_str().unwrap_or_default().to_string();
        }
    }

    info.properties = normal.to_vec();

    // println!("hiearchy: {:?}", hierarchy.rows[0]);
    // Now get any subfolders!
    for rows in &hierarchy.rows {
        let mut sub = SubFolder {
            name: String::new(),
            node: 0,
        };
        for columns in rows {
            if columns
                .column
                .property_name
                .contains(&PropertyName::PidTagDisplayNameW)
            {
                sub.name = columns.value.as_str().unwrap_or_default().to_string();
            } else if columns
                .column
                .property_name
                .contains(&PropertyName::PidTagLtpRowId)
            {
                sub.node = columns.value.as_u64().unwrap_or_default();
            }

            if !sub.name.is_empty() && sub.node != 0 {
                println!("sub name: {}", sub.name);
                info.subfolders.push(sub);
                break;
            }
        }
    }

    info.subfolder_count = info.subfolders.len();
    info.hierarchy = hierarchy.rows.clone();

    //println!("FAI: {fai:?}");
    // FAI contains associated folder metadata
    for rows in &fai.rows {
        /*
         * TODO:
         * 1. Get PidTagLtpRowId. Need to node id and blocks again :/
         * 2. Probably do that in another function/file
         */
        break;
        //panic!("FAI info!");
    }

    println!("Contents len: {}", contents.rows.len());
    for rows in &contents.rows {
        /*
         * TODO:
         * 1. Get PidTagLtpRowId. Need to get node id and blocks again :/
         * 2. Probably do that in another function/file
         */
        break;
    }

    info
}

pub(crate) fn search_folder_details(
    search: &[PropertyContext],
    criteria: &[PropertyContext],
) -> FolderInfo {
    let mut info = FolderInfo {
        name: String::new(),
        created: String::new(),
        modified: String::new(),
        hierarchy: Vec::new(),
        associated_content: Vec::new(),
        properties: Vec::new(),
        subfolders: Vec::new(),
        subfolder_count: 0,
        message_count: 0,
        messages: Vec::new(),
        // folders: Vec::new(),
    };

    for props in search {
        if props.name.contains(&PropertyName::PidTagDisplayNameW) {
            info.name = props.value.as_str().unwrap_or_default().to_string();
            // info.folders.push(info.name);
        } else if props.name.contains(&PropertyName::PidTagCreationTime) {
            info.created = props.value.as_str().unwrap_or_default().to_string();
        } else if props
            .name
            .contains(&PropertyName::PidTagLastModificationTime)
        {
            info.modified = props.value.as_str().unwrap_or_default().to_string();
        }
    }

    info.properties = search.to_vec();
    info.properties.append(&mut criteria.to_vec());
    //info.hierarchy = contents.rows.clone();
    info
}
#[cfg(test)]
mod tests {
    use super::folder_details;
    use crate::{
        artifacts::os::windows::outlook::{
            header::{FormatType, Node, NodeID},
            helper::{OutlookReader, OutlookReaderAction},
            pages::btree::{BlockType, LeafBlockData, LeafNodeData},
            tables::{context::parse_table_context, property::OutlookPropertyContext},
        },
        filesystem::files::file_reader,
    };
    use std::io::BufReader;

    #[test]
    fn test_folder_details_root() {
        // Read the 4 parts of the root folder and see what happens! :)

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
        let mut leaf_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };

        let mut leaf_descriptor = None;

        let mut root_num = 0;
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
        let folder_number: u64 = 290;
        for nodes in outlook_reader.node_btree.iter() {
            if let Some(id) = nodes.btree.get(&(folder_number as u32)) {
                root_num = id.node.node_id_num;

                for node in nodes.btree.values() {
                    if node.node.node_id_num == root_num
                        && node.node.node_id == NodeID::NormalFolder
                    {
                        normal = node.clone();
                    } else if node.node.node_id_num == root_num
                        && node.node.node_id == NodeID::HierarchyTable
                    {
                        hierahcy = node.clone();
                    } else if node.node.node_id_num == root_num
                        && node.node.node_id == NodeID::ContentsTable
                    {
                        contents = node.clone();
                    } else if node.node.node_id_num == root_num
                        && node.node.node_id == NodeID::FaiContentsTable
                    {
                        fai = node.clone();
                    }
                }
                break;
            }
        }

        let mut hierarchy_block = leaf_block.clone();
        let mut hiearchy_descriptor = None;
        let mut contents_block = leaf_block.clone();
        let mut contents_descriptor = None;
        let mut fai_block = leaf_block.clone();
        let mut fai_descriptor = None;

        for blocks in outlook_reader.block_btree.iter() {
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

        let block_value = outlook_reader
            .get_block_data(None, &leaf_block, leaf_descriptor.as_ref())
            .unwrap();
        let (_, normal_result) = outlook_reader
            .parse_property_context(&block_value.data, &block_value.descriptors)
            .unwrap();

        let hiearchy_value = outlook_reader
            .get_block_data(None, &hierarchy_block, hiearchy_descriptor.as_ref())
            .unwrap();
        let (_, hiearhy_result) =
            parse_table_context(&hiearchy_value.data, &hiearchy_value.descriptors).unwrap();

        let content_value = outlook_reader
            .get_block_data(None, &contents_block, contents_descriptor.as_ref())
            .unwrap();
        let (_, contents_result) =
            parse_table_context(&content_value.data, &content_value.descriptors).unwrap();

        let fai_value = outlook_reader
            .get_block_data(None, &fai_block, fai_descriptor.as_ref())
            .unwrap();
        let (_, fai_result) = parse_table_context(&fai_value.data, &fai_value.descriptors).unwrap();

        let result = folder_details(
            &normal_result,
            &hiearhy_result,
            &contents_result,
            &fai_result,
        );

        assert_eq!(result.created, "2024-07-29T04:29:52.000Z");
        assert_eq!(result.subfolder_count, 2);
        assert_eq!(result.name, "");
    }
}
