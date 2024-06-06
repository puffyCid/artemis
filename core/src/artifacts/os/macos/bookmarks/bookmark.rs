use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
    },
    time::{cocoatime_to_unixepoch, unixepoch_to_iso},
};
use common::macos::{BookmarkData, CreationFlags, TargetFlags, VolumeFlags};
use log::warn;
use nom::{
    bytes::complete::take,
    number::complete::{be_f64, le_i32, le_i64},
};
use std::{
    fmt::Debug,
    str::{from_utf8, Utf8Error},
};

#[derive(Debug)]
pub(crate) struct BookmarkHeader {
    /**Signature: Book */
    pub(crate) signature: u32,
    /**Total size of `Bookmark` */
    _bookmark_data_length: u32,
    /**`Bookmark` version */
    _version: u32,
    /**
     * Offset to start of `Bookmark` data, always 0x30 (48)
     * Followed by 32 bytes of reserved space
     */
    pub(crate) bookmark_data_offset: u32,
}

#[derive(Debug)]
struct TableOfContentsOffset {
    /**Offset to the start of Table of Contents (TOC) */
    table_of_contents_offset: u32,
}

#[derive(Debug)]
struct TableOfContentsHeader {
    /**Size of Table of Contents (TOC) */
    data_length: u32,
    /**Unused TOC record type */
    _record_type: u16,
    /**Unused TOC flags */
    _flags: u16,
}

#[derive(Debug)]
struct TableOfContentsData {
    /**TOC data level */
    _level: u32,
    /**Offset to next TOC record */
    _next_record_offset: u32,
    /**Number of records in TOC */
    number_of_records: u32,
}

#[derive(Debug)]
struct TableOfContentsDataRecord {
    /**TOC record type */
    record_type: u32,
    /**Offset to record data */
    data_offset: u32,
    /**Reserved (0) */
    _reserved: u32,
}

#[derive(Debug)]
struct StandardDataRecord {
    /**Length of data */
    _data_length: u32,
    /**Data type: STRING, four (4) bytes, true, false, URL, UUID, etc */
    data_type: u32,
    /**The actual `Bookmark` data. Based on `data_type` */
    record_data: Vec<u8>,
    /**Record type associated with TOC entry */
    record_type: u32,
}

/// Parse bookmark header
pub(crate) fn parse_bookmark_header(data: &[u8]) -> nom::IResult<&[u8], BookmarkHeader> {
    let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _bookmark_data_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _version) = nom_unsigned_four_bytes(input, Endian::Be)?;
    let (input, bookmark_data_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let filler_size: u32 = 32;
    let (input, _) = take(filler_size)(input)?;

    let bookmark_header = BookmarkHeader {
        signature,
        _bookmark_data_length,
        _version,
        bookmark_data_offset,
    };
    Ok((input, bookmark_header))
}

/// Parse the core bookmark data
pub(crate) fn parse_bookmark_data(data: &[u8]) -> nom::IResult<&[u8], BookmarkData> {
    let (input, table_of_contents_offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let book_data = TableOfContentsOffset {
        table_of_contents_offset,
    };

    let toc_offset_size: u32 = 4;
    let (input, core_data) = take(book_data.table_of_contents_offset - toc_offset_size)(input)?;
    let (input, toc_header) = table_of_contents_header(input)?;

    let (toc_record_data, toc_content_data) =
        table_of_contents_data(input, toc_header.data_length)?;

    let (_, toc_content_data_record) =
        table_of_contents_record(toc_record_data, &toc_content_data.number_of_records)?;

    let mut bookmark_data = BookmarkData {
        path: String::new(),
        cnid_path: String::new(),
        target_flags: Vec::new(),
        created: String::new(),
        volume_path: String::new(),
        volume_url: String::new(),
        volume_name: String::new(),
        volume_uuid: String::new(),
        volume_size: 0,
        volume_created: String::new(),
        volume_flags: Vec::new(),
        volume_root: false,
        localized_name: String::new(),
        security_extension_rw: String::new(),
        username: String::new(),
        uid: 0,
        creation_options: Vec::new(),
        folder_index: 0,
        is_executable: false,
        security_extension_ro: String::new(),
        file_ref_flag: false,
    };

    // Data types
    let string_type = 0x0101;
    let data_type = 0x0201;
    let _number_one_byte = 0x0301;
    let _number_two_byte = 0x0302;
    let number_four_byte = 0x0303;
    let number_eight_byte = 0x0304;
    let _number_float = 0x0305;
    let _number_float64 = 0x0306;
    let date = 0x0400;
    let bool_false = 0x0500;
    let bool_true = 0x0501;
    let array_type = 0x0601;
    let _dictionary = 0x0701;
    let _uuid = 0x0801;
    let url = 0x0901;
    let _url_relative = 0x0902;

    // Table of Contents Key types
    let _unknown = 0x1003;
    let target_path = 0x1004;
    let target_cnid_path = 0x1005;
    let target_flags = 0x1010;
    let _target_filename = 0x1020;
    let target_creation_date = 0x1040;
    let _unknown2 = 0x1054;
    let _unknown3 = 0x1055;
    let _unknown4 = 0x1056;
    let _unknown5 = 0x1057;
    let _unknown6 = 0x1101;
    let _unknown7 = 0x1102;
    let _toc_path = 0x2000;
    let volume_path = 0x2002;
    let volume_url = 0x2005;
    let volume_name = 0x2010;
    let volume_uuid = 0x2011;
    let volume_size = 0x2012;
    let volume_creation = 0x2013;
    let _volume_bookmark = 0x2040;
    let volume_flags = 0x2020;
    let volume_root = 0x2030;
    let _volume_mount_point = 0x2050;
    let _unknown8 = 0x2070;
    let contain_folder_index = 0xc001;
    let creator_username = 0xc011;
    let creator_uid = 0xc012;
    let file_ref_flag = 0xd001;
    let creation_options = 0xd010;
    let _url_length_array = 0xe003;
    let localized_name = 0xf017;
    let _unknown9 = 0xf022;
    let security_extension_rw = 0xf080;
    let security_extension_ro = 0xf081;
    let is_executable = 0xf000f;

    for record in toc_content_data_record {
        let (_, standard_data) = bookmark_standard_data(core_data, &record)?;
        let record_data = standard_data.record_data;
        let mut standard_data_vec: Vec<StandardDataRecord> = Vec::new();

        // If data type is ARRAY, standard_data data points to offsets that contain actual bookmark data
        if standard_data.data_type == array_type {
            let results_data = bookmark_array(&record_data);
            match results_data {
                Ok((_, results)) => {
                    if results.is_empty() {
                        continue;
                    }

                    let (_, std_data_vec) = bookmark_array_data(core_data, results, &record)?;

                    // Now we have data for actual bookmark data
                    standard_data_vec = std_data_vec;
                }
                Err(err) => warn!("[bookmarks] Failed to get bookmark standard data: {err:?}"),
            }
        }

        // If we did not have to parse array data, get bookmark data based on record and data types
        if standard_data_vec.is_empty() {
            if standard_data.record_type == target_flags && standard_data.data_type == data_type {
                let flag_data = bookmark_target_flags(&record_data);
                match flag_data {
                    Ok((_, flags)) => {
                        if flags.is_empty() {
                            continue;
                        }
                        bookmark_data.target_flags = get_target_flags(&flags);
                    }
                    Err(err) => warn!("[bookmarks] Failed to parse Target Flags: {err:?}"),
                }
            } else if standard_data.record_type == target_creation_date
                && standard_data.data_type == date
            {
                let creation_data = bookmark_data_type_date(&record_data);
                match creation_data {
                    Ok((_, creation)) => {
                        bookmark_data.created =
                            unixepoch_to_iso(&cocoatime_to_unixepoch(&creation));
                    }
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse Target File created timestamp: {err:?}");
                    }
                }
            } else if standard_data.record_type == volume_path
                && standard_data.data_type == string_type
            {
                let volume_root = bookmark_data_type_string(&record_data);
                match volume_root {
                    Ok(volume_root_data) => bookmark_data.volume_path = volume_root_data,
                    Err(err) => warn!("[bookmarks] Failed to parse Volume Path: {err:?}"),
                }
            } else if standard_data.record_type == volume_url && standard_data.data_type == url {
                let volume_url_data = bookmark_data_type_string(&record_data);
                match volume_url_data {
                    Ok(volume_url) => bookmark_data.volume_url = volume_url,
                    Err(err) => warn!("[bookmarks] Failed to parse Volume URL data: {err:?}"),
                }
            } else if standard_data.record_type == volume_name
                && standard_data.data_type == string_type
            {
                let volume_name_data = bookmark_data_type_string(&record_data);
                match volume_name_data {
                    Ok(volume_name) => bookmark_data.volume_name = volume_name,
                    Err(err) => warn!("[bookmarks] Failed to parse Volume Name data: {err:?}"),
                }
            } else if standard_data.record_type == volume_uuid
                && standard_data.data_type == string_type
            {
                let volume_uuid_data = bookmark_data_type_string(&record_data);
                match volume_uuid_data {
                    Ok(volume_uuid) => bookmark_data.volume_uuid = volume_uuid,
                    Err(err) => warn!("[bookmarks] Failed to parse Volume UUID: {err:?}"),
                }
            } else if standard_data.record_type == volume_size
                && standard_data.data_type == number_eight_byte
            {
                let test = bookmark_data_type_number_eight(&record_data);
                match test {
                    Ok((_, size)) => bookmark_data.volume_size = size,
                    Err(err) => warn!("[bookmarks] Failed to parse Volume size: {err:?}"),
                }
            } else if standard_data.record_type == volume_creation
                && standard_data.data_type == date
            {
                let creation_data = bookmark_data_type_date(&record_data);
                match creation_data {
                    Ok((_, creation)) => {
                        bookmark_data.volume_created =
                            unixepoch_to_iso(&cocoatime_to_unixepoch(&creation));
                    }
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse Volume Creation timestamp: {err:?}");
                    }
                }
            } else if standard_data.record_type == volume_flags
                && standard_data.data_type == data_type
            {
                let flags_data = bookmark_target_flags(&record_data);
                match flags_data {
                    Ok((_, flags)) => bookmark_data.volume_flags = get_volume_flags(&flags),
                    Err(err) => warn!("[bookmarks] Failed to parse Volume Flags: {err:?}"),
                }
            } else if standard_data.record_type == volume_root
                && standard_data.data_type == bool_true
            {
                bookmark_data.volume_root = true;
            } else if standard_data.record_type == volume_root
                && standard_data.data_type == bool_false
            {
                bookmark_data.volume_root = false;
            } else if standard_data.record_type == file_ref_flag
                && standard_data.data_type == bool_true
            {
                bookmark_data.file_ref_flag = true;
            } else if standard_data.record_type == is_executable
                && standard_data.data_type == bool_true
            {
                bookmark_data.is_executable = true;
            } else if standard_data.record_type == is_executable
                && standard_data.data_type == bool_false
            {
                bookmark_data.is_executable = false;
            } else if standard_data.record_type == localized_name
                && standard_data.data_type == string_type
            {
                let local_name_data = bookmark_data_type_string(&record_data);
                match local_name_data {
                    Ok(local_name) => bookmark_data.localized_name = local_name,
                    Err(err) => warn!("[bookmarks] Failed to parse Localized Name: {err:?}"),
                }
            } else if standard_data.record_type == security_extension_rw
                && standard_data.data_type == data_type
            {
                let extension_data = bookmark_data_type_string(&record_data);
                match extension_data {
                    Ok(extension) => bookmark_data.security_extension_rw = extension,
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse Security Extension RW: {err:?}");
                    }
                }
            } else if standard_data.record_type == security_extension_ro
                && standard_data.data_type == data_type
            {
                let extension_data = bookmark_data_type_string(&record_data);
                match extension_data {
                    Ok(extension) => bookmark_data.security_extension_ro = extension,
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse Security Extension RO: {err:?}");
                    }
                }
            } else if standard_data.record_type == creator_username
                && standard_data.data_type == string_type
            {
                let username_data = bookmark_data_type_string(&record_data);
                match username_data {
                    Ok(username) => bookmark_data.username = username,
                    Err(err) => warn!("[bookmarks] Failed to parse bookmark username: {err:?}"),
                }
            } else if standard_data.record_type == contain_folder_index
                && standard_data.data_type == number_four_byte
            {
                let index_data = bookmark_data_type_number_four(&record_data);
                match index_data {
                    Ok((_, index)) => bookmark_data.folder_index = index as i64,
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse bookmark folder index: {err:?}");
                    }
                }
            } else if standard_data.record_type == contain_folder_index
                && standard_data.data_type == number_eight_byte
            {
                let index_data = bookmark_data_type_number_eight(&record_data);
                match index_data {
                    Ok((_, index)) => bookmark_data.folder_index = index,
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse bookmark folder index: {err:?}");
                    }
                }
            } else if standard_data.record_type == creator_uid
                && standard_data.data_type == number_four_byte
            {
                let uid_data = bookmark_data_type_number_four(&record_data);
                match uid_data {
                    Ok((_, uid)) => bookmark_data.uid = uid,
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse bookmark Creator UID: {err:?}");
                    }
                }
            } else if standard_data.record_type == creation_options
                && standard_data.data_type == number_four_byte
            {
                let creation_options_data = bookmark_data_type_number_four(&record_data);
                match creation_options_data {
                    Ok((_, options)) => {
                        bookmark_data.creation_options = get_creation_flags(&options);
                    }
                    Err(err) => {
                        warn!("[bookmarks] Failed to parse bookmark Creation options: {err:?}");
                    }
                }
            } else {
                warn!(
                    "[bookmarks] Unknown Record Type: {} and Data type: {}",
                    standard_data.record_type, standard_data.data_type
                );
            }
            continue;
        }

        // Get bookmark array data based on data and record types
        for standard_data in standard_data_vec {
            if standard_data.data_type == string_type && standard_data.record_type == target_path {
                let path_data = bookmark_data_type_string(&standard_data.record_data);
                match path_data {
                    Ok(path) => bookmark_data.path = format!("{}/{path}", bookmark_data.path),
                    Err(_err) => continue,
                }
            } else if standard_data.data_type == number_eight_byte
                && standard_data.record_type == target_cnid_path
            {
                let cnid_data = bookmark_cnid(&standard_data.record_data);
                match cnid_data {
                    Ok((_, cnid)) => {
                        bookmark_data.cnid_path = format!("{}/{cnid}", bookmark_data.cnid_path);
                    }
                    Err(_err) => continue,
                }
            }
        }
    }
    Ok((input, bookmark_data))
}

/// Parse the bookmark array data
fn bookmark_array_data<'a>(
    data: &'a [u8],
    array_offsets: Vec<u32>,
    record: &TableOfContentsDataRecord,
) -> nom::IResult<&'a [u8], Vec<StandardDataRecord>> {
    let mut standard_data_vec: Vec<StandardDataRecord> = Vec::new();

    for offset in array_offsets {
        let data_record = TableOfContentsDataRecord {
            record_type: record.record_type,
            data_offset: offset,
            _reserved: 0,
        };
        let (_, results) = bookmark_standard_data(data, &data_record)?;
        standard_data_vec.push(results);
    }

    Ok((data, standard_data_vec))
}

/// Parse the Table of Contents (TOC) header
fn table_of_contents_header(data: &[u8]) -> nom::IResult<&[u8], TableOfContentsHeader> {
    let (input, data_length) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _record_type) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, _flags) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let toc_header = TableOfContentsHeader {
        data_length,
        _record_type,
        _flags,
    };

    Ok((input, toc_header))
}

/// Parse the TOC data
fn table_of_contents_data(
    data: &[u8],
    data_length: u32,
) -> nom::IResult<&[u8], TableOfContentsData> {
    let (input, _level) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _next_record_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, number_of_records) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut final_input = input;

    let toc_data = TableOfContentsData {
        _level,
        _next_record_offset,
        number_of_records,
    };

    let record_size = 12;
    let record_data = record_size * toc_data.number_of_records;

    // Verify TOC data length is equal to number of records (Number of Records * Record Size (12 bytes))
    if record_data > data_length {
        let (_, actual_record_data) = take(record_data)(input)?;
        final_input = actual_record_data;
    }
    Ok((final_input, toc_data))
}

/// Parse the TOC data record
fn table_of_contents_record<'a>(
    data: &'a [u8],
    records: &u32,
) -> nom::IResult<&'a [u8], Vec<TableOfContentsDataRecord>> {
    let mut input_data = data;
    let mut record: u32 = 0;
    let mut toc_records_vec: Vec<TableOfContentsDataRecord> = Vec::new();

    // Loop through until all records have been parsed
    loop {
        if &record == records {
            break;
        }
        record += 1;

        let (input, record_type) = nom_unsigned_four_bytes(input_data, Endian::Le)?;
        let (input, data_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _reserved) = nom_unsigned_four_bytes(input, Endian::Le)?;
        input_data = input;

        let toc_data_record = TableOfContentsDataRecord {
            record_type,
            data_offset,
            _reserved,
        };

        toc_records_vec.push(toc_data_record);
    }
    Ok((input_data, toc_records_vec))
}

/// Parse the bookmark standard data
fn bookmark_standard_data<'a>(
    bookmark_data: &'a [u8],
    toc_record: &TableOfContentsDataRecord,
) -> nom::IResult<&'a [u8], StandardDataRecord> {
    let toc_offset_value: u32 = 4;

    // Subtract toc offset value from data offset since we already nom'd the value
    let offset = (toc_record.data_offset - toc_offset_value) as usize;

    // Nom data til standard data info
    let (input, _) = take(offset)(bookmark_data)?;

    let (input, data_length) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, data_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, record_data) = take(data_length)(input)?;

    let toc_standard_data = StandardDataRecord {
        _data_length: data_length,
        record_data: record_data.to_vec(),
        data_type,
        record_type: toc_record.record_type,
    };

    Ok((input, toc_standard_data))
}

/// Get the offsets for the array data
fn bookmark_array(standard_data: &[u8]) -> nom::IResult<&[u8], Vec<u32>> {
    let mut array_offsets: Vec<u32> = Vec::new();
    let mut input = standard_data;

    loop {
        let (input_data, data_offsets) = nom_unsigned_four_bytes(input, Endian::Le)?;

        array_offsets.push(data_offsets);
        input = input_data;
        if input_data.is_empty() {
            break;
        }
    }
    Ok((input, array_offsets))
}

/// Get the path/strings related to bookmark
fn bookmark_data_type_string(standard_data: &[u8]) -> Result<String, Utf8Error> {
    let path = from_utf8(standard_data)?;
    Ok(path.to_string())
}

/// Get the CNID path for the target
fn bookmark_cnid(standard_data: &[u8]) -> nom::IResult<&[u8], i64> {
    let (data, cnid) = le_i64(standard_data)?;
    Ok((data, cnid))
}

/// Get bookmark target flags
fn bookmark_target_flags(standard_data: &[u8]) -> nom::IResult<&[u8], Vec<u64>> {
    let mut input = standard_data;
    let mut array_flags: Vec<u64> = Vec::new();
    let max_flag_size = 3;

    // Target flags are composed of three (3) 8 byte values
    loop {
        let (data, flags) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        input = data;
        array_flags.push(flags);
        if input.is_empty() || array_flags.len() == max_flag_size {
            break;
        }
    }
    Ok((input, array_flags))
}

/// Get bookmark volume size
fn bookmark_data_type_number_eight(standard_data: &[u8]) -> nom::IResult<&[u8], i64> {
    let (data, size) = le_i64(standard_data)?;
    Ok((data, size))
}

/// Get bookmark folder index
fn bookmark_data_type_number_four(standard_data: &[u8]) -> nom::IResult<&[u8], i32> {
    let (data, index) = le_i32(standard_data)?;
    Ok((data, index))
}

/// Get bookmark creation timestamps
fn bookmark_data_type_date(standard_data: &[u8]) -> nom::IResult<&[u8], f64> {
    //Apple stores timestamps as Big Endian Float64
    let (data, creation) = be_f64(standard_data)?;
    Ok((data, creation))
}

/// Determine Target flags
fn get_target_flags(flags: &[u64]) -> Vec<TargetFlags> {
    let mut target_flags = Vec::new();
    // Only first entry contains the flag data
    if let Some(target) = flags.first() {
        let file = 0x1;
        let dir = 0x2;
        let symbolic = 0x4;
        let volume = 0x8;
        let package = 0x10;
        let immut = 0x20;
        let user_immut = 0x40;
        let hidden = 0x80;
        let hiddent_ext = 0x100;
        let app = 0x200;
        let compressed = 0x400;
        let set_hidden_extension = 0x800;
        let readable = 0x1000;
        let writable = 0x2000;
        let executable = 0x4000;
        let alias_file = 0x8000;
        let mount = 0x10000;

        if (target & file) == file {
            target_flags.push(TargetFlags::RegularFile);
        }
        if (target & dir) == dir {
            target_flags.push(TargetFlags::Directory);
        }
        if (target & symbolic) == symbolic {
            target_flags.push(TargetFlags::SymbolicLink);
        }
        if (target & volume) == volume {
            target_flags.push(TargetFlags::Volume);
        }
        if (target & package) == package {
            target_flags.push(TargetFlags::Package);
        }
        if (target & immut) == immut {
            target_flags.push(TargetFlags::SystemImmutable);
        }
        if (target & user_immut) == user_immut {
            target_flags.push(TargetFlags::UserImmutable);
        }
        if (target & hidden) == hidden {
            target_flags.push(TargetFlags::Hidden);
        }
        if (target & hiddent_ext) == hiddent_ext {
            target_flags.push(TargetFlags::HasHiddenExtension);
        }
        if (target & app) == app {
            target_flags.push(TargetFlags::Application);
        }
        if (target & compressed) == compressed {
            target_flags.push(TargetFlags::Compressed);
        }
        if (target & set_hidden_extension) == set_hidden_extension {
            target_flags.push(TargetFlags::CanSetHiddenExtension);
        }
        if (target & readable) == readable {
            target_flags.push(TargetFlags::Readable);
        }
        if (target & writable) == writable {
            target_flags.push(TargetFlags::Writable);
        }
        if (target & executable) == executable {
            target_flags.push(TargetFlags::Executable);
        }
        if (target & alias_file) == alias_file {
            target_flags.push(TargetFlags::AliasFile);
        }
        if (target & mount) == mount {
            target_flags.push(TargetFlags::MountTrigger);
        }
    }
    target_flags
}

/// Determine Volume flags
fn get_volume_flags(flags: &[u64]) -> Vec<VolumeFlags> {
    let mut volume_flags = Vec::new();
    if let Some(volume) = flags.first() {
        if (volume & 0x1) == 0x1 {
            volume_flags.push(VolumeFlags::Local);
        }
        if (volume & 0x2) == 0x2 {
            volume_flags.push(VolumeFlags::Automount);
        }
        if (volume & 0x4) == 0x4 {
            volume_flags.push(VolumeFlags::DontBrowse);
        }
        if (volume & 0x8) == 0x8 {
            volume_flags.push(VolumeFlags::ReadOnly);
        }
        if (volume & 0x10) == 0x10 {
            volume_flags.push(VolumeFlags::Quarantined);
        }
        if (volume & 0x20) == 0x20 {
            volume_flags.push(VolumeFlags::Ejectable);
        }
        if (volume & 0x40) == 0x40 {
            volume_flags.push(VolumeFlags::Removable);
        }
        if (volume & 0x80) == 0x80 {
            volume_flags.push(VolumeFlags::Internal);
        }
        if (volume & 0x100) == 0x100 {
            volume_flags.push(VolumeFlags::External);
        }
        if (volume & 0x200) == 0x200 {
            volume_flags.push(VolumeFlags::DiskImage);
        }
        if (volume & 0x400) == 0x400 {
            volume_flags.push(VolumeFlags::FileVault);
        }
        if (volume & 0x800) == 0x800 {
            volume_flags.push(VolumeFlags::LocaliDiskMirror);
        }
        if (volume & 0x1000) == 0x1000 {
            volume_flags.push(VolumeFlags::Ipod);
        }
        if (volume & 0x2000) == 0x2000 {
            volume_flags.push(VolumeFlags::Idisk);
        }
        if (volume & 0x4000) == 0x4000 {
            volume_flags.push(VolumeFlags::Cd);
        }
        if (volume & 0x8000) == 0x8000 {
            volume_flags.push(VolumeFlags::Dvd);
        }
        if (volume & 0x10000) == 0x10000 {
            volume_flags.push(VolumeFlags::DeviceFileSystem);
        }
        if (volume & 0x20000) == 0x20000 {
            volume_flags.push(VolumeFlags::TimeMachine);
        }
        if (volume & 0x40000) == 0x40000 {
            volume_flags.push(VolumeFlags::Airport);
        }
        if (volume & 0x80000) == 0x80000 {
            volume_flags.push(VolumeFlags::VideoDisk);
        }
        if (volume & 0x100000) == 0x100000 {
            volume_flags.push(VolumeFlags::DvdVideo);
        }
        if (volume & 0x200000) == 0x200000 {
            volume_flags.push(VolumeFlags::BdVideo);
        }
        if (volume & 0x400000) == 0x400000 {
            volume_flags.push(VolumeFlags::MobileTimeMachine);
        }
        if (volume & 0x800000) == 0x800000 {
            volume_flags.push(VolumeFlags::NetworkOptical);
        }
        if (volume & 0x1000000) == 0x1000000 {
            volume_flags.push(VolumeFlags::BeingRepaired);
        }
        if (volume & 0x2000000) == 0x2000000 {
            volume_flags.push(VolumeFlags::Unmounted);
        }

        // Volume supports
        if (volume & 0x100000000) == 0x100000000 {
            volume_flags.push(VolumeFlags::SupportsPersistentIds);
        }
        if (volume & 0x200000000) == 0x200000000 {
            volume_flags.push(VolumeFlags::SupportsSearchFs);
        }
        if (volume & 0x400000000) == 0x400000000 {
            volume_flags.push(VolumeFlags::SupportsExchange);
        }
        if (volume & 0x1000000000) == 0x1000000000 {
            volume_flags.push(VolumeFlags::SupportsSymbolicLinks);
        }
        if (volume & 0x2000000000) == 0x2000000000 {
            volume_flags.push(VolumeFlags::SupportsDenyModes);
        }
        if (volume & 0x4000000000) == 0x4000000000 {
            volume_flags.push(VolumeFlags::SupportsCopyFile);
        }
        if (volume & 0x8000000000) == 0x8000000000 {
            volume_flags.push(VolumeFlags::SupportsReadDirAttr);
        }
        if (volume & 0x10000000000) == 0x10000000000 {
            volume_flags.push(VolumeFlags::SupportsJournaling);
        }
        if (volume & 0x20000000000) == 0x20000000000 {
            volume_flags.push(VolumeFlags::SupportsRename);
        }
        if (volume & 0x40000000000) == 0x40000000000 {
            volume_flags.push(VolumeFlags::SupportsFastStatFs);
        }
        if (volume & 0x80000000000) == 0x80000000000 {
            volume_flags.push(VolumeFlags::SupportsCaseSensitiveNames);
        }
        if (volume & 0x100000000000) == 0x100000000000 {
            volume_flags.push(VolumeFlags::SupportsCasePreservedNames);
        }
        if (volume & 0x200000000000) == 0x200000000000 {
            volume_flags.push(VolumeFlags::SupportsFlock);
        }
        if (volume & 0x400000000000) == 0x400000000000 {
            volume_flags.push(VolumeFlags::SupportsNoRootDirectoryTimes);
        }
        if (volume & 0x800000000000) == 0x800000000000 {
            volume_flags.push(VolumeFlags::SupportsExtendedSecurity);
        }
        if (volume & 0x1000000000000) == 0x1000000000000 {
            volume_flags.push(VolumeFlags::Supports2TbFileSize);
        }
        if (volume & 0x2000000000000) == 0x2000000000000 {
            volume_flags.push(VolumeFlags::SupportsHardLinks);
        }
        if (volume & 0x4000000000000) == 0x4000000000000 {
            volume_flags.push(VolumeFlags::SupportsMandatoryByteRangeLocks);
        }
        if (volume & 0x8000000000000) == 0x8000000000000 {
            volume_flags.push(VolumeFlags::SupportsPathFromId);
        }
        if (volume & 0x20000000000000) == 0x20000000000000 {
            volume_flags.push(VolumeFlags::SupportsJournaling);
        }
        if (volume & 0x40000000000000) == 0x40000000000000 {
            volume_flags.push(VolumeFlags::SupportsSparseFiles);
        }
        if (volume & 0x80000000000000) == 0x80000000000000 {
            volume_flags.push(VolumeFlags::SupportsZeroRunes);
        }
        if (volume & 0x100000000000000) == 0x100000000000000 {
            volume_flags.push(VolumeFlags::SupportsVolumeSizes);
        }
        if (volume & 0x200000000000000) == 0x200000000000000 {
            volume_flags.push(VolumeFlags::SupportsRemoteEvents);
        }
        if (volume & 0x400000000000000) == 0x400000000000000 {
            volume_flags.push(VolumeFlags::SupportsHiddenFiles);
        }
        if (volume & 0x800000000000000) == 0x800000000000000 {
            volume_flags.push(VolumeFlags::SupportsDecmpFsCompression);
        }
        if (volume & 0x1000000000000000) == 0x1000000000000000 {
            volume_flags.push(VolumeFlags::Has64BitObjectIds);
        }
        if *volume == 0xffffffffffffffff {
            volume_flags.push(VolumeFlags::PropertyFlagsAll);
        }
    }

    volume_flags
}

/// Determine Creation flags
fn get_creation_flags(flags: &i32) -> Vec<CreationFlags> {
    let not_implict = 0x20000000;
    let prefer_id = 0x100;
    let read_only = 0x1000;
    let security = 0x800;
    let suitable = 0x400;
    let minimal = 0x200;

    let mut creation = Vec::new();
    if (flags & not_implict) == not_implict {
        creation.push(CreationFlags::WithoutImplicitSecurityScope);
    }
    if (flags & prefer_id) == prefer_id {
        creation.push(CreationFlags::PreferFileIDResolutionMask);
    }
    if (flags & read_only) == read_only {
        creation.push(CreationFlags::SecurityScopeAllowOnlyReadAccess);
    }
    if (flags & security) == security {
        creation.push(CreationFlags::SecurityScope);
    }
    if (flags & suitable) == suitable {
        creation.push(CreationFlags::SuitableBookmark);
    }
    if (flags & minimal) == minimal {
        creation.push(CreationFlags::MinimalBookmark);
    }
    creation
}

#[cfg(test)]
mod tests {
    use super::{get_target_flags, TableOfContentsDataRecord};
    use crate::artifacts::os::macos::bookmarks::bookmark::{
        bookmark_array, bookmark_array_data, bookmark_cnid, bookmark_data_type_date,
        bookmark_data_type_number_eight, bookmark_data_type_number_four, bookmark_data_type_string,
        bookmark_standard_data, bookmark_target_flags, get_creation_flags, get_volume_flags,
        parse_bookmark_data, parse_bookmark_header, table_of_contents_data,
        table_of_contents_header, table_of_contents_record,
    };
    use common::macos::{CreationFlags, TargetFlags, VolumeFlags};

    #[test]
    fn test_bookmark_header() {
        let test_header = [
            98, 111, 111, 107, 72, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, header) = parse_bookmark_header(&test_header).unwrap();

        assert_eq!(header.signature, 1802465122);
        assert_eq!(header._bookmark_data_length, 584);
        assert_eq!(header.bookmark_data_offset, 48);
        assert_eq!(header._version, 1040);
    }

    #[test]
    fn test_get_target_flags() {
        let results = get_target_flags(&[1]);
        assert_eq!(results[0], TargetFlags::RegularFile);
    }

    #[test]
    fn test_get_volume_flags() {
        let results = get_volume_flags(&[1]);
        assert_eq!(results[0], VolumeFlags::Local);
    }

    #[test]
    fn test_get_creation_flags() {
        let results = get_creation_flags(&0x100);
        assert_eq!(results[0], CreationFlags::PreferFileIDResolutionMask);
    }

    #[test]
    fn test_table_of_contents_header() {
        let test_header = [192, 0, 0, 0, 254, 255, 255, 255];
        let (_, header) = table_of_contents_header(&test_header).unwrap();

        assert_eq!(header.data_length, 192);
        assert_eq!(header._record_type, 65534);
        assert_eq!(header._flags, 65535);
    }

    #[test]
    fn test_bookmark() {
        let test_data = [
            8, 2, 0, 0, 12, 0, 0, 0, 1, 1, 0, 0, 65, 112, 112, 108, 105, 99, 97, 116, 105, 111,
            110, 115, 13, 0, 0, 0, 1, 1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 46, 97,
            112, 112, 0, 0, 0, 8, 0, 0, 0, 1, 6, 0, 0, 4, 0, 0, 0, 24, 0, 0, 0, 8, 0, 0, 0, 4, 3,
            0, 0, 103, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 42, 198, 10, 0, 0, 0, 0, 0, 8,
            0, 0, 0, 1, 6, 0, 0, 64, 0, 0, 0, 80, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 213,
            41, 226, 128, 0, 0, 24, 0, 0, 0, 1, 2, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47,
            47, 12, 0, 0, 0, 1, 1, 0, 0, 77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8,
            0, 0, 0, 4, 3, 0, 0, 0, 96, 127, 115, 37, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 172,
            190, 215, 104, 0, 0, 0, 36, 0, 0, 0, 1, 1, 0, 0, 48, 65, 56, 49, 70, 51, 66, 49, 45,
            53, 49, 68, 57, 45, 51, 51, 51, 53, 45, 66, 51, 69, 51, 45, 49, 54, 57, 67, 51, 54, 52,
            48, 51, 54, 48, 68, 24, 0, 0, 0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0,
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1,
            5, 0, 0, 9, 0, 0, 0, 1, 1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 0, 0, 0,
            166, 0, 0, 0, 1, 2, 0, 0, 54, 52, 99, 98, 55, 101, 97, 97, 57, 97, 49, 98, 98, 99, 99,
            99, 52, 101, 49, 51, 57, 55, 99, 57, 102, 50, 97, 52, 49, 49, 101, 98, 101, 53, 51, 57,
            99, 100, 50, 57, 59, 48, 48, 48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48, 48, 48,
            48, 59, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 50, 48, 59, 99, 111,
            109, 46, 97, 112, 112, 108, 101, 46, 97, 112, 112, 45, 115, 97, 110, 100, 98, 111, 120,
            46, 114, 101, 97, 100, 45, 119, 114, 105, 116, 101, 59, 48, 49, 59, 48, 49, 48, 48, 48,
            48, 48, 52, 59, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 48, 97, 99, 54, 50, 97, 59, 47,
            97, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 115, 47, 115, 121, 110, 99, 116,
            104, 105, 110, 103, 46, 97, 112, 112, 0, 0, 0, 180, 0, 0, 0, 254, 255, 255, 255, 1, 0,
            0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 4, 16, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 5, 16, 0, 0, 96,
            0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 64, 16, 0, 0, 112, 0, 0,
            0, 0, 0, 0, 0, 2, 32, 0, 0, 48, 1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0, 160, 0, 0, 0, 0, 0,
            0, 0, 16, 32, 0, 0, 176, 0, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0, 228, 0, 0, 0, 0, 0, 0, 0,
            18, 32, 0, 0, 196, 0, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 212, 0, 0, 0, 0, 0, 0, 0, 32, 32,
            0, 0, 16, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 60, 1, 0, 0, 0, 0, 0, 0, 23, 240, 0, 0,
            68, 1, 0, 0, 0, 0, 0, 0, 128, 240, 0, 0, 88, 1, 0, 0, 0, 0, 0, 0,
        ];
        let (_, bookmark) = parse_bookmark_data(&test_data).unwrap();

        assert_eq!(bookmark.path.len(), 27);
        assert_eq!(bookmark.cnid_path.len(), 11);
        assert_eq!(bookmark.created, "2022-02-02T05:53:09.000Z");
        assert_eq!(bookmark.volume_created, "2008-08-22T21:48:36.000Z");
        assert_eq!(bookmark.target_flags.len(), 1);
    }

    #[test]
    fn test_table_of_contents_data() {
        let test_data = [
            1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 4, 16, 0, 0, 52, 0, 0, 0, 0, 0, 0, 0, 5, 16, 0, 0,
        ];
        let record_data_size = 192;
        let (_, toc_data) = table_of_contents_data(&test_data, record_data_size).unwrap();

        assert_eq!(toc_data._level, 1);
        assert_eq!(toc_data._next_record_offset, 0);
        assert_eq!(toc_data.number_of_records, 15);
    }

    #[test]
    fn test_table_of_contents_record() {
        let test_record = [
            4, 16, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 5, 16, 0, 0, 96, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0,
            0, 128, 0, 0, 0, 0, 0, 0, 0, 64, 16, 0, 0, 112, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 48,
            1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 176, 0, 0, 0,
            0, 0, 0, 0, 17, 32, 0, 0, 228, 0, 0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 196, 0, 0, 0, 0, 0,
            0, 0, 19, 32, 0, 0, 212, 0, 0, 0, 0, 0, 0, 0, 32, 32, 0, 0, 16, 1, 0, 0, 0, 0, 0, 0,
            48, 32, 0, 0, 60, 1, 0, 0, 0, 0, 0, 0, 23, 240, 0, 0, 68, 1, 0, 0, 0, 0, 0, 0, 128,
            240, 0, 0, 88, 1, 0, 0, 0, 0, 0, 0,
        ];
        let records = 14;
        let (_, record) = table_of_contents_record(&test_record, &records).unwrap();

        assert_eq!(record[0].record_type, 4100);
        assert_eq!(record[0].data_offset, 48);
        assert_eq!(record[0]._reserved, 0);
        assert_eq!(record.len(), records as usize);
    }

    #[test]
    fn test_bookmark_standard_data() {
        let bookmark_data = [
            12, 0, 0, 0, 1, 1, 0, 0, 65, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 115, 13,
            0, 0, 0, 1, 1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 46, 97, 112, 112, 0, 0,
            0, 8, 0, 0, 0, 1, 6, 0, 0, 4, 0, 0, 0, 24, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 103, 0, 0,
            0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 42, 198, 10, 0, 0, 0, 0, 0, 8, 0, 0, 0, 1, 6, 0,
            0, 64, 0, 0, 0, 80, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 213, 41, 226, 128, 0, 0,
            24, 0, 0, 0, 1, 2, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1,
            1, 0, 0, 77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0,
            0, 96, 127, 115, 37, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 172, 190, 215, 104, 0, 0, 0,
            36, 0, 0, 0, 1, 1, 0, 0, 48, 65, 56, 49, 70, 51, 66, 49, 45, 53, 49, 68, 57, 45, 51,
            51, 51, 53, 45, 66, 51, 69, 51, 45, 49, 54, 57, 67, 51, 54, 52, 48, 51, 54, 48, 68, 24,
            0, 0, 0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 9, 0, 0, 0, 1,
            1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 0, 0, 0, 166, 0, 0, 0, 1, 2, 0, 0,
            54, 52, 99, 98, 55, 101, 97, 97, 57, 97, 49, 98, 98, 99, 99, 99, 52, 101, 49, 51, 57,
            55, 99, 57, 102, 50, 97, 52, 49, 49, 101, 98, 101, 53, 51, 57, 99, 100, 50, 57, 59, 48,
            48, 48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 50, 48, 59, 99, 111, 109, 46, 97, 112, 112, 108,
            101, 46, 97, 112, 112, 45, 115, 97, 110, 100, 98, 111, 120, 46, 114, 101, 97, 100, 45,
            119, 114, 105, 116, 101, 59, 48, 49, 59, 48, 49, 48, 48, 48, 48, 48, 52, 59, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 97, 99, 54, 50, 97, 59, 47, 97, 112, 112, 108, 105,
            99, 97, 116, 105, 111, 110, 115, 47, 115, 121, 110, 99, 116, 104, 105, 110, 103, 46,
            97, 112, 112, 0, 0, 0,
        ];
        let toc_record = TableOfContentsDataRecord {
            record_type: 8209,
            data_offset: 228,
            _reserved: 0,
        };
        let (_, std_data) = bookmark_standard_data(&bookmark_data, &toc_record).unwrap();

        assert_eq!(std_data._data_length, 36);
        assert_eq!(std_data.data_type, 257);
        assert_eq!(
            std_data.record_data,
            [
                48, 65, 56, 49, 70, 51, 66, 49, 45, 53, 49, 68, 57, 45, 51, 51, 51, 53, 45, 66, 51,
                69, 51, 45, 49, 54, 57, 67, 51, 54, 52, 48, 51, 54, 48, 68,
            ]
        );
        assert_eq!(std_data.record_type, 8209);
    }

    #[test]
    fn test_bookmark_array_data() {
        let test_data = [
            12, 0, 0, 0, 1, 1, 0, 0, 65, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 115, 13,
            0, 0, 0, 1, 1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 46, 97, 112, 112, 0, 0,
            0, 8, 0, 0, 0, 1, 6, 0, 0, 4, 0, 0, 0, 24, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 103, 0, 0,
            0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 42, 198, 10, 0, 0, 0, 0, 0, 8, 0, 0, 0, 1, 6, 0,
            0, 64, 0, 0, 0, 80, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 213, 41, 226, 128, 0, 0,
            24, 0, 0, 0, 1, 2, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1,
            1, 0, 0, 77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0,
            0, 96, 127, 115, 37, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 172, 190, 215, 104, 0, 0, 0,
            36, 0, 0, 0, 1, 1, 0, 0, 48, 65, 56, 49, 70, 51, 66, 49, 45, 53, 49, 68, 57, 45, 51,
            51, 51, 53, 45, 66, 51, 69, 51, 45, 49, 54, 57, 67, 51, 54, 52, 48, 51, 54, 48, 68, 24,
            0, 0, 0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 9, 0, 0, 0, 1,
            1, 0, 0, 83, 121, 110, 99, 116, 104, 105, 110, 103, 0, 0, 0, 166, 0, 0, 0, 1, 2, 0, 0,
            54, 52, 99, 98, 55, 101, 97, 97, 57, 97, 49, 98, 98, 99, 99, 99, 52, 101, 49, 51, 57,
            55, 99, 57, 102, 50, 97, 52, 49, 49, 101, 98, 101, 53, 51, 57, 99, 100, 50, 57, 59, 48,
            48, 48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48, 48, 48, 48, 59, 48, 48, 48, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 50, 48, 59, 99, 111, 109, 46, 97, 112, 112, 108,
            101, 46, 97, 112, 112, 45, 115, 97, 110, 100, 98, 111, 120, 46, 114, 101, 97, 100, 45,
            119, 114, 105, 116, 101, 59, 48, 49, 59, 48, 49, 48, 48, 48, 48, 48, 52, 59, 48, 48,
            48, 48, 48, 48, 48, 48, 48, 48, 48, 97, 99, 54, 50, 97, 59, 47, 97, 112, 112, 108, 105,
            99, 97, 116, 105, 111, 110, 115, 47, 115, 121, 110, 99, 116, 104, 105, 110, 103, 46,
            97, 112, 112, 0, 0, 0,
        ];
        let test_array_offsets = [4, 24];
        let toc_record = TableOfContentsDataRecord {
            record_type: 4100,
            data_offset: 48,
            _reserved: 0,
        };
        let records = 2;

        let (_, std_record) =
            bookmark_array_data(&test_data, (test_array_offsets).to_vec(), &toc_record).unwrap();

        assert_eq!(std_record[0].record_type, 4100);
        assert_eq!(std_record[0].data_type, 257);
        assert_eq!(
            std_record[0].record_data,
            [65, 112, 112, 108, 105, 99, 97, 116, 105, 111, 110, 115]
        );
        assert_eq!(std_record[0]._data_length, 12);

        assert_eq!(std_record.len(), records);
    }

    #[test]
    fn test_bookmark_array() {
        let test_array = [4, 0, 0, 0, 24, 0, 0, 0];
        let (_, book_array) = bookmark_array(&test_array).unwrap();

        assert_eq!(book_array.len(), 2);
        assert_eq!(book_array[0], 4);
        assert_eq!(book_array[1], 24);
    }

    #[test]
    fn test_bookmark_data_type_string() {
        let test_path = [83, 121, 110, 99, 116, 104, 105, 110, 103];

        let book_path = bookmark_data_type_string(&test_path).unwrap();
        assert_eq!(book_path, "Syncthing");
    }

    #[test]
    fn test_bookmark_cnid() {
        let test_cnid = [42, 198, 10, 0, 0, 0, 0, 0];

        let (_, book_cnid) = bookmark_cnid(&test_cnid).unwrap();
        assert_eq!(book_cnid, 706090);
    }

    #[test]
    fn test_bookmark_target_flags() {
        let test_flags = [
            129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, book_flags) = bookmark_target_flags(&test_flags).unwrap();

        assert_eq!(book_flags.len(), 3);
        assert_eq!(book_flags[0], 4294967425);
        assert_eq!(book_flags[1], 4294972399);
        assert_eq!(book_flags[2], 0);
    }

    #[test]
    fn test_bookmark_data_type_number_eight() {
        let test_volume_size = [0, 96, 127, 115, 37, 0, 0, 0];

        let (_, book_size) = bookmark_data_type_number_eight(&test_volume_size).unwrap();

        assert_eq!(book_size, 160851517440);
    }

    #[test]
    fn test_bookmark_data_type_date() {
        let test_creation = [65, 172, 190, 215, 104, 0, 0, 0];

        let (_, book_creation) = bookmark_data_type_date(&test_creation).unwrap();

        assert_eq!(book_creation, 241134516.0);
    }

    #[test]
    fn test_bookmark_data_type_number_four() {
        let test_creation = [0, 0, 0, 32];

        let (_, creation_options) = bookmark_data_type_number_four(&test_creation).unwrap();
        assert_eq!(creation_options, 536870912);
    }

    #[test]
    fn test_safari_downloads_bookmark() {
        let data = [
            98, 111, 111, 107, 204, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 217, 10, 110, 155, 143, 43,
            6, 0, 139, 200, 168, 230, 42, 214, 22, 102, 103, 228, 112, 159, 141, 163, 20, 27, 36,
            83, 233, 178, 57, 208, 89, 105, 200, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40, 5,
            0, 0, 0, 1, 1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112, 117,
            102, 102, 121, 99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111, 97,
            100, 115, 0, 0, 0, 28, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101,
            108, 108, 45, 55, 46, 50, 46, 52, 45, 111, 115, 120, 45, 120, 54, 52, 46, 112, 107,
            103, 16, 0, 0, 0, 1, 6, 0, 0, 16, 0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0, 0, 0, 8, 0,
            0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11, 128, 5, 0, 0,
            0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0,
            216, 194, 61, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6, 0, 0, 128, 0, 0, 0, 144, 0, 0, 0, 160,
            0, 0, 0, 176, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 196, 48, 15, 162, 9, 145, 58, 24, 0,
            0, 0, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 8, 0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 245, 1, 0,
            0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47, 12, 0, 0, 0, 1, 1, 0, 0,
            77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0, 4, 3, 0, 0, 0, 112,
            196, 208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81, 128, 0, 0, 36, 0,
            0, 0, 1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45, 52, 68, 65,
            50, 45, 56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51, 24, 0, 0,
            0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0, 0, 254,
            255, 255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 104, 0, 0, 0, 0, 0, 0,
            0, 5, 16, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 232, 0, 0, 0, 0, 0, 0, 0, 64,
            16, 0, 0, 216, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 180, 1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0,
            36, 1, 0, 0, 0, 0, 0, 0, 16, 32, 0, 0, 52, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0, 104, 1,
            0, 0, 0, 0, 0, 0, 18, 32, 0, 0, 72, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 88, 1, 0, 0, 0,
            0, 0, 0, 32, 32, 0, 0, 148, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 192, 1, 0, 0, 0, 0, 0,
            0, 1, 192, 0, 0, 8, 1, 0, 0, 0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 18,
            192, 0, 0, 24, 1, 0, 0, 0, 0, 0, 0, 16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (bookmark_data, header) = parse_bookmark_header(&data).unwrap();

        assert_eq!(header.signature, 1802465122);
        assert_eq!(header._bookmark_data_length, 716);
        assert_eq!(header.bookmark_data_offset, 48);
        assert_eq!(header._version, 1040);

        let (_, bookmark) = parse_bookmark_data(bookmark_data).unwrap();

        assert_eq!(bookmark.created, "2022-06-20T03:21:40.000Z");
        assert_eq!(bookmark.volume_created, "2022-02-26T07:05:07.000Z");

        assert_eq!(
            bookmark.path,
            "/Users/puffycid/Downloads/powershell-7.2.4-osx-x64.pkg"
        );
        assert_eq!(bookmark.cnid_path, "/21327/360459/360510/37602008");
        assert_eq!(bookmark.volume_path, "/");
        assert_eq!(bookmark.volume_url, "file:///");
        assert_eq!(bookmark.volume_name, "Macintosh HD");
        assert_eq!(bookmark.volume_uuid, "96FB41C0-6CE9-4DA2-8435-35BC19C735A3");
        assert_eq!(bookmark.volume_size, 2000662327296);
        assert_eq!(
            bookmark.volume_flags,
            vec![
                VolumeFlags::Local,
                VolumeFlags::Internal,
                VolumeFlags::SupportsPersistentIds
            ]
        );
        assert_eq!(bookmark.volume_root, true);
        assert_eq!(bookmark.localized_name, "");
        assert_eq!(bookmark.target_flags, vec![TargetFlags::RegularFile]);
        assert_eq!(bookmark.username, "puffycid");
        assert_eq!(bookmark.folder_index, 2);
        assert_eq!(bookmark.uid, 501);
        assert_eq!(
            bookmark.creation_options,
            vec![
                CreationFlags::WithoutImplicitSecurityScope,
                CreationFlags::SecurityScopeAllowOnlyReadAccess,
                CreationFlags::SecurityScope
            ]
        );
        assert_eq!(bookmark.security_extension_rw, "");
        assert_eq!(bookmark.security_extension_ro, "");
        assert_eq!(bookmark.file_ref_flag, false);
    }
}
