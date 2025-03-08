use crate::utils::nom_helper::{
    Endian, nom_data, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
    nom_unsigned_sixteen_bytes, nom_unsigned_two_bytes,
};
use log::error;
use nom::error::ErrorKind;
use serde::{Deserialize, Serialize};

use super::tables::header::get_heap_node_id;

#[derive(PartialEq, Debug)]
pub(crate) struct OutlookHeader {
    sig: u32,
    crc_hash: u32,
    pub(crate) content_type: ContentType,
    pub(crate) format_type: FormatType,
    client_version: u16,
    creation_platform: u8,
    access_platform: u8,
    unknown: u32,
    unknown2: u32,
    next_available_index_pointer: u64,
    next_available_index_back_pointer: u64,
    node_ids: Vec<Node>,
    last_data_allocation_table_offset: u64,
    total_available_data_size: u64,
    total_available_page_size: u64,
    node_btree_backpointer: u64,
    pub(crate) node_btree_root: u64,
    block_btree_backpointer: u64,
    pub(crate) block_btree_root: u64,
    allocation_type: AllocationType,
    pub(crate) encryption_type: EncryptionType,
    /**Only used on ANSI32 header */
    initial_data_free_map: u128,
    /**Only used on ANSI32 header */
    initial_page_free_map: u128,
}

#[derive(PartialEq, Debug)]
pub(crate) enum ContentType {
    PersonalAddressBook,
    PersonalStorageTable,
    OfflineStorageTable,
    Unknown,
}

#[derive(PartialEq, Debug)]
pub(crate) enum FormatType {
    ANSI32,
    Unicode64,
    Unicode64_4k,
    Unknown,
}

#[derive(PartialEq, Debug)]
/// `<https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/d9bcc1fd-c66a-41b3-b6d7-ed09d2a25ced>`
enum AllocationType {
    /**Possible corruption */
    InvalidMaps,
    ValidMaps,
    Unknown,
}

#[derive(PartialEq, Debug)]
pub(crate) enum EncryptionType {
    None,
    CompressEncryption,
    HighEncryption,
    Unknown,
}

/// Parse Outlook file header
pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], OutlookHeader> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, crc_hash) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, content_data) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, format_data) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, client_version) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (input, creation_platform) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, access_platform) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let format_type = get_format(&format_data);

    if format_type == FormatType::ANSI32 {
        error!("[outlook] Got ANSI32 FormatType. This type is currently unsupported");
        return Err(nom::Err::Failure(nom::error::Error::new(
            data,
            ErrorKind::Fail,
        )));
    }

    let (input, _unused) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, next_available_index_back_pointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (mut input, _seed_value) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let limit = 32;
    let mut count = 0;
    let mut node_ids = Vec::new();

    // Array of known node IDs `<https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/18d7644e-cb33-4e11-95c0-34d8a84fbff6>`
    while count < limit {
        let (remaining, id_data) = nom_data(input, 4)?;
        let result = get_node_ids(id_data);
        let values = match result {
            Ok((_, value)) => value,
            Err(err) => {
                error!("[outlook] Failed to parse node id data: {err:?}");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    data,
                    ErrorKind::Fail,
                )));
            }
        };

        node_ids.push(values);
        input = remaining;

        count += 1;
    }
    let (input, _unknown) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    // Start of header root
    let (input, _unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _total_file_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, last_data_allocation_table_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, total_available_data_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, total_available_page_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, node_btree_backpointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, node_btree_root) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, block_btree_backpointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, block_btree_root) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let (input, allocation_data) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, _unknown2) = nom_unsigned_two_bytes(input, Endian::Le)?;

    // Done parsing Header root. Now need to parse rest of header
    let (input, _unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, initial_data_free_map) = nom_unsigned_sixteen_bytes(input, Endian::Le)?;
    let (input, initial_page_free_map) = nom_unsigned_sixteen_bytes(input, Endian::Le)?;
    let (input, _sentinel) = nom_unsigned_one_byte(input, Endian::Le)?;

    let (input, encryption_data) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, _unknown4) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, next_available_index_pointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, _crc_hash) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let header = OutlookHeader {
        sig,
        crc_hash,
        content_type: get_content(&content_data),
        format_type,
        client_version,
        creation_platform,
        access_platform,
        unknown,
        unknown2,
        next_available_index_pointer,
        next_available_index_back_pointer,
        node_ids,
        last_data_allocation_table_offset,
        total_available_data_size,
        total_available_page_size,
        node_btree_backpointer,
        node_btree_root,
        block_btree_backpointer,
        block_btree_root,
        allocation_type: get_allocation(&allocation_data),
        encryption_type: get_encryption(&encryption_data),
        initial_data_free_map,
        initial_page_free_map,
    };

    if header.encryption_type != EncryptionType::None {
        error!(
            "[outlook] Outlook file is encrypted: {:?}. Currently decryption is not supported",
            header.encryption_type
        );
        return Err(nom::Err::Failure(nom::error::Error::new(
            data,
            ErrorKind::Fail,
        )));
    }

    Ok((input, header))
}

/// Determine Outlook content format
fn get_content(content: &u16) -> ContentType {
    match content {
        0x4f53 => ContentType::OfflineStorageTable,
        0x4d53 => ContentType::PersonalStorageTable,
        0x4241 => ContentType::PersonalAddressBook,
        _ => ContentType::Unknown,
    }
}

/// Determine Outlook structure format
fn get_format(format: &u16) -> FormatType {
    match format {
        14 | 15 => FormatType::ANSI32,
        21 | 23 => FormatType::Unicode64,
        36 => FormatType::Unicode64_4k,
        _ => FormatType::Unknown,
    }
}

/// Get Outlook allocation type
fn get_allocation(data: &u8) -> AllocationType {
    match data {
        0 => AllocationType::InvalidMaps,
        1 | 2 => AllocationType::ValidMaps,
        _ => AllocationType::Unknown,
    }
}

/// Check if Outlook data is encrypted
fn get_encryption(data: &u8) -> EncryptionType {
    match data {
        0 => EncryptionType::None,
        1 => EncryptionType::CompressEncryption,
        2 => EncryptionType::HighEncryption,
        _ => EncryptionType::Unknown,
    }
}

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Node {
    pub(crate) node_id: NodeID,
    pub(crate) node_id_num: u64,
    pub(crate) node: u32,
}

#[derive(Eq, Hash, PartialEq, Debug, Clone, Serialize, Deserialize)]
/**See: `<https://learn.microsoft.com/en-us/openspecs/office_file_formats/ms-pst/18d7644e-cb33-4e11-95c0-34d8a84fbff6>` */
pub(crate) enum NodeID {
    HeapNode,
    InternalNode,
    NormalFolder,
    SearchFolder,
    Message,
    Attachment,
    SearchUpdateQueue,
    SearchCriteria,
    FolderAssociatedInfo,
    ContentsTableIndex,
    Inbox,
    Outbox,
    HierarchyTable,
    ContentsTable,
    /**`FolderAssociatedInfo` (FAI) */
    FaiContentsTable,
    SearchContentsTable,
    AttachmentTable,
    RecipientTable,
    SearchTableIndex,
    /**Referred to as: LTP */
    LocalDescriptors,
    Unknown,
}

/// Determine `NodeIDs` in header
pub(crate) fn get_node_ids(data: &[u8]) -> nom::IResult<&[u8], Node> {
    let (input, value) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let id = get_heap_node_id(value);

    let node = Node {
        node_id: id.node,
        node_id_num: ((value >> 5) & 0x07ffffff) as u64,
        node: value,
    };

    Ok((input, node))
}

#[cfg(test)]
mod tests {
    use super::parse_header;
    use crate::artifacts::os::windows::outlook::header::{
        AllocationType, ContentType, EncryptionType, FormatType, NodeID, get_allocation,
        get_content, get_encryption, get_format, get_node_ids,
    };

    #[test]
    fn test_parse_header() {
        let test = [
            33, 66, 68, 78, 25, 171, 176, 200, 83, 79, 36, 0, 12, 0, 1, 1, 120, 0, 0, 0, 143, 148,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 73, 86, 0, 0, 0, 0, 0, 0, 231, 9, 0, 0, 0, 4, 0, 0, 0, 4,
            0, 0, 33, 4, 0, 0, 7, 64, 0, 0, 58, 1, 1, 0, 17, 4, 0, 0, 7, 64, 0, 0, 7, 64, 0, 0, 39,
            128, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 33, 4, 0, 0, 33, 4, 0, 0,
            33, 4, 0, 0, 7, 64, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 33, 4, 0, 0, 33, 4, 0, 0,
            0, 4, 0, 0, 0, 4, 0, 0, 0, 4, 0, 0, 17, 4, 0, 0, 0, 4, 0, 0, 35, 128, 0, 0, 0, 4, 0, 0,
            0, 4, 0, 0, 0, 4, 0, 0, 244, 13, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 32, 255,
            1, 0, 0, 0, 0, 0, 160, 0, 1, 0, 0, 0, 0, 0, 148, 35, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 38, 86, 0, 0, 0, 0, 0, 0, 0, 64, 25, 1, 0, 0, 0, 0, 72, 86, 0, 0, 0, 0, 0, 0, 0,
            224, 30, 1, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 132, 37, 1, 0, 0, 0, 0, 0, 197, 129,
            205, 177, 147, 6, 129, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, header) = parse_header(&test).unwrap();
        assert_eq!(header.content_type, ContentType::OfflineStorageTable);
        assert_eq!(header.format_type, FormatType::Unicode64_4k);
        assert_eq!(header.block_btree_root, 18800640);
        assert_eq!(header.node_btree_root, 18432000);
    }

    #[test]
    fn test_get_content() {
        let test = 0x4241;
        assert_eq!(get_content(&test), ContentType::PersonalAddressBook);
    }

    #[test]
    fn test_get_format() {
        let test = 14;
        assert_eq!(get_format(&test), FormatType::ANSI32);
    }

    #[test]
    fn test_get_allocation() {
        let test = 0;
        assert_eq!(get_allocation(&test), AllocationType::InvalidMaps);
    }

    #[test]
    fn test_get_encryption() {
        let test = 1;
        assert_eq!(get_encryption(&test), EncryptionType::CompressEncryption);
    }

    #[test]
    fn test_get_node_ids() {
        let test = [0, 4, 0, 0];

        let mut data = Vec::new();
        let (_, results) = get_node_ids(&test).unwrap();
        data.push(results);
        assert_eq!(data[0].node_id, NodeID::HeapNode);
        assert_eq!(data[0].node_id_num, 32);
        assert_eq!(data[0].node, 1024);
    }
}
