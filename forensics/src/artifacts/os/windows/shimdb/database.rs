use super::tag::get_tag;
use super::tags::binary::get_binary;
use super::tags::dword::get_dword;
use super::tags::qword::get_qword;
use super::tags::stringref::parse_stringref;
use crate::artifacts::os::windows::shimdb::tag::Tags;
use crate::artifacts::os::windows::shimdb::tags::binary::parse_binary;
use crate::artifacts::os::windows::shimdb::tags::dword::parse_dword;
use crate::artifacts::os::windows::shimdb::tags::list::parse_list;
use crate::artifacts::os::windows::shimdb::tags::qword::parse_qword;
use crate::artifacts::os::windows::shimdb::tags::word::parse_word;
use crate::utils::encoding::base64_encode_standard;
use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};
use crate::utils::time::{filetime_to_unixepoch, unixepoch_to_iso};
use crate::utils::uuid::format_guid_le_bytes;
use common::windows::DatabaseData;
use nom::bytes::complete::take;
use std::collections::HashMap;

/// Get the database data associated with a sdb file
pub(crate) fn get_data(data: &[u8]) -> nom::IResult<&[u8], Vec<u8>> {
    let (input, list_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (input, list_data) = take(list_size)(input)?;
    Ok((input, list_data.to_vec()))
}

/// Parse database in a sdb file. Including any metadata
pub(crate) fn parse_db<'a>(
    db_data: &'a [u8],
    stringtable_data: &'a [u8],
    tag_values: &HashMap<u16, String>,
) -> nom::IResult<&'a [u8], DatabaseData> {
    // db metadata
    let tag_time = 0x5001;
    let tag_compiler_version = 0x6022;
    let tag_name = 0x6001;
    let tag_runtime_platform = 0x4021;
    let tag_db_id = 0x9007;

    let mut input = db_data;
    let mut database_data = DatabaseData {
        sdb_version: String::new(),
        compile_time: String::new(),
        compiler_version: String::new(),
        name: String::new(),
        platform: 0,
        database_id: String::new(),
        additional_metadata: HashMap::new(),
        list_data: Vec::new(),
    };

    while !input.is_empty() {
        let (sdb_data, (tag, tag_value)) = get_tag(input)?;
        if tag_value == tag_time {
            let (sdb_data, compile_time) = get_db_time(sdb_data)?;

            input = sdb_data;
            database_data.compile_time = unixepoch_to_iso(&filetime_to_unixepoch(&compile_time));
            continue;
        } else if tag_value == tag_compiler_version {
            let (sdb_data, compiler_version) = get_compiler_version(sdb_data, stringtable_data)?;

            input = sdb_data;
            database_data.compiler_version = compiler_version;
            continue;
        } else if tag_value == tag_name {
            let (sdb_data, name) = get_name(sdb_data, stringtable_data)?;

            input = sdb_data;
            database_data.name = name;
            continue;
        } else if tag_value == tag_runtime_platform {
            let (sdb_data, platform) = get_platform(sdb_data)?;

            input = sdb_data;
            database_data.platform = platform;
            continue;
        } else if tag_value == tag_db_id {
            let (sdb_data, db_id) = get_db_guid(sdb_data)?;

            input = sdb_data;
            database_data.database_id = db_id;
            continue;
        }
        let (tag_data, value) = match tag {
            Tags::String => break, // strings only found in stringtable, which we parse in stringref
            Tags::Binary => parse_binary(sdb_data, &tag_value)?,
            Tags::List => {
                let (sdb_data, mut index_data) =
                    parse_list(sdb_data, stringtable_data, tag_values)?;
                input = sdb_data;
                database_data.list_data.append(&mut index_data);
                continue;
            }
            Tags::Stringref => parse_stringref(sdb_data, stringtable_data)?,
            Tags::Qword => parse_qword(sdb_data)?,
            Tags::Dword => parse_dword(sdb_data)?,
            Tags::Null => (sdb_data, String::from("true")),
            Tags::Word => parse_word(sdb_data)?,
            Tags::Unkonwn => {
                database_data
                    .additional_metadata
                    .insert(format!("{tag_value}"), base64_encode_standard(sdb_data));
                break;
            }
        };
        let tag_name_option = tag_values.get(&tag_value);
        match tag_name_option {
            Some(tag_name) => database_data
                .additional_metadata
                .insert(tag_name.clone(), value),
            // If we do not know the Tag name just provide the number
            _ => database_data
                .additional_metadata
                .insert(format!("{tag_value}"), value),
        };
        input = tag_data;
    }
    Ok((input, database_data))
}

/// The timestamp associated with the db. Its a QWORD tag
fn get_db_time(data: &[u8]) -> nom::IResult<&[u8], u64> {
    get_qword(data)
}

/// The sdb compiler associated with the db. Its a stringref tag
fn get_compiler_version<'a>(
    data: &'a [u8],
    stringtable_data: &'a [u8],
) -> nom::IResult<&'a [u8], String> {
    parse_stringref(data, stringtable_data)
}

/// The name associated with the db. Its a stringref tag
fn get_name<'a>(data: &'a [u8], stringtable_data: &'a [u8]) -> nom::IResult<&'a [u8], String> {
    parse_stringref(data, stringtable_data)
}

/// The platform associated with the db. Its a dword tag
fn get_platform(data: &[u8]) -> nom::IResult<&[u8], u32> {
    get_dword(data)
}

/// The GUID associated with the db. Its a GUID (16 bytes) stored in a binary tag
fn get_db_guid(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, db_id_data) = get_binary(data)?;
    let db_id = format_guid_le_bytes(db_id_data);
    Ok((input, db_id))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::shimdb::{
            database::{
                get_compiler_version, get_data, get_db_guid, get_db_time, get_name, get_platform,
                parse_db,
            },
            stringtable::get_stringtable_data,
            tag::generate_tags,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_data() {
        let test_data = [
            4, 1, 0, 0, 1, 80, 0, 64, 128, 91, 39, 68, 209, 1, 34, 96, 6, 0, 0, 0, 1, 96, 28, 0, 0,
            0, 33, 64, 6, 0, 0, 0, 7, 144, 16, 0, 0, 0, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
            17, 17, 17, 17, 17, 2, 112, 132, 85, 1, 0, 3, 112, 6, 0, 0, 0, 3, 96, 148, 0, 0, 0, 3,
            112, 6, 0, 0, 0, 3, 96, 180, 0, 0, 0, 3, 112, 6, 0, 0, 0, 3, 96, 208, 0, 0, 0, 3, 112,
            8, 0, 0, 0, 1, 16, 3, 96, 240, 0, 0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 16, 1, 0, 0,
            3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 44, 1, 0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 74, 1,
            0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 104, 1, 0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96,
            134, 1, 0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 166, 1, 0, 0, 3, 112, 8, 0, 0, 0, 1,
            16, 3, 96, 196, 1, 0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 222, 1, 0, 0, 3, 112, 8, 0,
            0, 0, 1, 16, 3, 96, 248, 1, 0, 0, 3, 112, 8, 0, 0, 0, 1, 16, 3, 96, 24, 2, 0, 0, 3,
            112, 8, 0, 0, 0, 1, 16, 3, 96, 56, 2, 0, 0,
        ];
        let (_, result) = get_data(&test_data).unwrap();

        assert_eq!(result.len(), 260);
    }

    #[test]
    fn test_parse_db() {
        let tag_values = generate_tags();

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/stringtable.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, table_data) = get_stringtable_data(&buffer).unwrap();
        assert_eq!(table_data.len(), 1687580);

        test_location.pop();
        test_location.push("database.raw");
        let test_data = read_file(&test_location.display().to_string()).unwrap();

        let (_, result) = parse_db(&test_data, &table_data, &tag_values).unwrap();

        assert_eq!(result.additional_metadata.len(), 0);
        assert_eq!(result.compile_time, "2016-01-01T00:00:00.000Z");
        assert_eq!(result.platform, 6);
        assert_eq!(result.compiler_version, "3.0.0.9");
        assert_eq!(
            result.name,
            "Microsoft Windows Application Compatibility Fix Database"
        );
        assert_eq!(result.sdb_version, "");
        assert_eq!(result.database_id, "11111111-1111-1111-1111-111111111111");
        assert_eq!(result.list_data.len(), 13581);
    }

    #[test]
    fn test_get_db_time() {
        let test_data = [0, 64, 128, 91, 39, 68, 209, 1];
        let (_, result) = get_db_time(&test_data).unwrap();
        assert_eq!(result, 130960800000000000) // FILETIME
    }

    #[test]
    fn test_get_compiler_version() {
        let test_data = [6, 0, 0, 0];
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/stringtable.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, table_data) = get_stringtable_data(&buffer).unwrap();
        assert_eq!(table_data.len(), 1687580);
        let (_, result) = get_compiler_version(&test_data, &table_data).unwrap();
        assert_eq!(result, "3.0.0.9")
    }

    #[test]
    fn test_get_name() {
        let test_data = [28, 0, 0, 0];
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/stringtable.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, table_data) = get_stringtable_data(&buffer).unwrap();
        assert_eq!(table_data.len(), 1687580);
        let (_, result) = get_name(&test_data, &table_data).unwrap();
        assert_eq!(
            result,
            "Microsoft Windows Application Compatibility Fix Database"
        )
    }

    #[test]
    fn test_get_platform() {
        let test_data = [6, 0, 0, 0];

        let (_, result) = get_platform(&test_data).unwrap();
        assert_eq!(result, 6)
    }

    #[test]
    fn test_get_db_guid() {
        let test_data = [
            16, 0, 0, 0, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17,
        ];

        let (_, result) = get_db_guid(&test_data).unwrap();
        assert_eq!(result, "11111111-1111-1111-1111-111111111111")
    }
}
