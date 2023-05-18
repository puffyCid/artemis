use super::binary::parse_binary;
use super::dword::parse_dword;
use super::qword::parse_qword;
use super::stringref::parse_stringref;
use crate::artifacts::os::windows::shimdb::tag::{TagData, Tags};
use crate::artifacts::os::windows::shimdb::tags::word::parse_word;
use crate::utils::encoding::base64_encode_standard;
use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};
use nom::bytes::complete::take;
use std::collections::HashMap;

/// Start parsing the `LIST` tag and store all results in Vec of `TagData`
pub(crate) fn parse_list<'a>(
    data: &'a [u8],
    stringtable_data: &'a [u8],
    tag_values: &HashMap<u16, String>,
) -> nom::IResult<&'a [u8], Vec<TagData>> {
    let mut shim_data: Vec<TagData> = Vec::new();
    let (input, _) = get_list_data(data, stringtable_data, &mut shim_data, tag_values)?;
    Ok((input, shim_data))
}

/// Parse the start of a `LIST` tag and any sublists
fn get_list_data<'a>(
    data: &'a [u8],
    stringtable_data: &'a [u8],
    shim_data: &mut Vec<TagData>,
    tag_values: &HashMap<u16, String>,
) -> nom::IResult<&'a [u8], ()> {
    let (input, list_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (input, mut list_data) = take(list_size)(input)?;

    let mut index_data = TagData {
        data: HashMap::new(),
        list_data: Vec::new(),
    };

    let min_tag_size = 2;
    while list_data.len() > min_tag_size {
        let (sdb_data, (tag, tag_value)) = TagData::get_tag(list_data)?;
        let (tag_data, value) = match tag {
            Tags::String => break, // strings only found in stringtable, which we parse in stringref
            Tags::Binary => parse_binary(sdb_data, &tag_value)?,
            Tags::List => {
                let (sdb_data, mut sublist_data) =
                    parse_sublist(sdb_data, stringtable_data, tag_values)?;
                index_data.list_data.append(&mut sublist_data);
                list_data = sdb_data;

                continue;
            }
            Tags::Stringref => parse_stringref(sdb_data, stringtable_data)?,
            Tags::Qword => parse_qword(sdb_data)?,
            Tags::Dword => parse_dword(sdb_data)?,
            Tags::Null => (sdb_data, String::from("true")),
            Tags::Word => parse_word(sdb_data)?,
            Tags::Unkonwn => {
                index_data
                    .data
                    .insert(format!("{tag_value}"), base64_encode_standard(sdb_data));
                break;
            }
        };
        let tag_name_option = tag_values.get(&tag_value);
        match tag_name_option {
            Some(tag_name) => index_data.data.insert(tag_name.clone(), value),
            // If we do not know the Tag name just provide the number
            _ => index_data.data.insert(format!("{tag_value}"), value),
        };
        list_data = tag_data;
    }

    shim_data.push(index_data);

    Ok((input, ()))
}

/// A list may have zero (0) or more sublists. Recursively parse any sublists and track using `HashMap` and Vec
fn parse_sublist<'a>(
    data: &'a [u8],
    stringtable_data: &'a [u8],
    tag_values: &HashMap<u16, String>,
) -> nom::IResult<&'a [u8], Vec<HashMap<String, String>>> {
    let (input, list_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (input, mut list_data) = take(list_size)(input)?;

    let mut list_entries: HashMap<String, String> = HashMap::new();
    let mut sublist_entries: Vec<HashMap<String, String>> = Vec::new();
    let min_tag_size = 2;
    while list_data.len() > min_tag_size {
        let (sdb_data, (tag, tag_value)) = TagData::get_tag(list_data)?;

        // Parse list data based on tag type (ex: BINARY, STRING, WORD, etc)
        let (tag_data, value) = match tag {
            Tags::String => break, // strings only found in stringtable, which we parse in stringref
            Tags::Binary => parse_binary(sdb_data, &tag_value)?,
            Tags::List => {
                // If we encounter another list recurse that list and add to our list vec tracker
                let (sdb_data, mut sublist_data) =
                    parse_sublist(sdb_data, stringtable_data, tag_values)?;
                sublist_entries.append(&mut sublist_data);
                list_data = sdb_data;
                continue;
            }
            Tags::Stringref => parse_stringref(sdb_data, stringtable_data)?,
            Tags::Qword => parse_qword(sdb_data)?,
            Tags::Dword => parse_dword(sdb_data)?,
            Tags::Null => (sdb_data, String::from("true")),
            Tags::Word => parse_word(sdb_data)?,
            Tags::Unkonwn => {
                list_entries.insert(format!("{tag_value}"), base64_encode_standard(sdb_data));
                break;
            }
        };

        // Add list data to our hashmap tracker
        let tag_name_option = tag_values.get(&tag_value);
        match tag_name_option {
            Some(tag_name) => list_entries.insert(tag_name.clone(), value),
            // If we do not know the Tag name just provide the number
            _ => list_entries.insert(format!("{tag_value}"), value),
        };

        list_data = tag_data;
    }

    sublist_entries.push(list_entries);

    Ok((input, sublist_entries))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        artifacts::os::windows::shimdb::{
            tag::TagData,
            tags::list::{get_list_data, parse_list, parse_sublist},
        },
        filesystem::files::read_file,
    };

    #[test]
    fn test_parse_list() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/indexes.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let table_data = [];
        let tag_values = TagData::generate_tags();

        let (_, result) = parse_list(&buffer, &table_data, &tag_values).unwrap();

        assert_eq!(result[0].data.len(), 0);
        assert_eq!(result[0].list_data[0].len(), 4);

        assert_eq!(
            result[0].list_data[0]
                .get("TAG_INDEX_BITS")
                .unwrap()
                .ends_with("AREI4RUI1Rntw4yMAMDI2QjBERnss5CMAa2lccWcsZeXo5CMA"),
            true
        );
        assert_eq!(
            result[0].list_data[0].get("TAG_INDEX_KEY").unwrap(),
            "24577"
        );
        assert_eq!(result[0].list_data[0].get("TAG_INDEX_FLAGS").unwrap(), "1");
        assert_eq!(
            result[0].list_data[0].get("TAG_INDEX_TAG").unwrap(),
            "28679"
        );
    }

    #[test]
    fn test_get_list_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimdb/win10/indexes.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let table_data = [];
        let tag_values = TagData::generate_tags();
        let mut shim_data: Vec<TagData> = Vec::new();

        let (_, result) = get_list_data(&buffer, &table_data, &mut shim_data, &tag_values).unwrap();
        assert_eq!(result, ());

        assert_eq!(shim_data[0].data.len(), 0);
        assert_eq!(shim_data[0].list_data[0].len(), 4);

        assert_eq!(
            shim_data[0].list_data[1]
                .get("TAG_INDEX_BITS")
                .unwrap()
                .ends_with("8CMAQVBURUdSQVTm8CMA"),
            true
        );
        assert_eq!(
            shim_data[0].list_data[2].get("TAG_INDEX_KEY").unwrap(),
            "16434"
        );
        assert_eq!(
            shim_data[0].list_data[3].get("TAG_INDEX_TAG").unwrap(),
            "28679"
        );
    }

    #[test]
    fn test_parse_sublist() {
        let test_data = [
            6, 2, 0, 0, 2, 56, 23, 112, 3, 56, 1, 96, 1, 152, 248, 1, 0, 0, 73, 77, 58, 68, 73, 77,
            85, 65, 176, 229, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 246, 229, 35, 0, 73, 77, 58,
            68, 73, 77, 85, 65, 60, 230, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 130, 230, 35, 0,
            73, 77, 58, 68, 73, 77, 85, 65, 200, 230, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 14,
            231, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 84, 231, 35, 0, 73, 77, 58, 68, 73, 77, 85,
            65, 154, 231, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 224, 231, 35, 0, 73, 77, 58, 68,
            73, 77, 85, 65, 38, 232, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 108, 232, 35, 0, 73,
            77, 58, 68, 73, 77, 85, 65, 178, 232, 35, 0, 73, 77, 58, 68, 73, 77, 85, 65, 248, 232,
            35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 62, 233, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65,
            132, 233, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 202, 233, 35, 0, 72, 89, 58, 68, 73,
            77, 85, 65, 16, 234, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 86, 234, 35, 0, 72, 89, 58,
            68, 73, 77, 85, 65, 156, 234, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 226, 234, 35, 0,
            72, 89, 58, 68, 73, 77, 85, 65, 40, 235, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 110,
            235, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 180, 235, 35, 0, 72, 89, 58, 68, 73, 77,
            85, 65, 250, 235, 35, 0, 72, 89, 58, 68, 73, 77, 85, 65, 64, 236, 35, 0, 72, 89, 58,
            68, 73, 77, 85, 65, 134, 236, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 204, 236, 35, 0,
            55, 123, 58, 68, 73, 77, 85, 65, 18, 237, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 88,
            237, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 158, 237, 35, 0, 55, 123, 58, 68, 73, 77,
            85, 65, 228, 237, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 42, 238, 35, 0, 55, 123, 58,
            68, 73, 77, 85, 65, 112, 238, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 182, 238, 35, 0,
            55, 123, 58, 68, 73, 77, 85, 65, 252, 238, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 66,
            239, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 136, 239, 35, 0, 55, 123, 58, 68, 73, 77,
            85, 65, 206, 239, 35, 0, 55, 123, 58, 68, 73, 77, 85, 65, 20, 240, 35, 0, 69, 77, 65,
            78, 67, 79, 82, 80, 90, 240, 35, 0, 65, 80, 84, 69, 71, 82, 65, 84, 160, 240, 35, 0,
            65, 80, 84, 69, 71, 82, 65, 84, 230, 240, 35, 0,
        ];
        let table_data = [];
        let tag_values = TagData::generate_tags();

        let (_, result) = parse_sublist(&test_data, &table_data, &tag_values).unwrap();
        assert_eq!(result.len(), 1);

        assert_eq!(
            result[0]
                .get("TAG_INDEX_BITS")
                .unwrap()
                .ends_with("8CMAQVBURUdSQVTm8CMA"),
            true
        );
        assert_eq!(result[0].get("TAG_INDEX_KEY").unwrap(), "24577");
        assert_eq!(result[0].get("TAG_INDEX_TAG").unwrap(), "28695");
    }
}
