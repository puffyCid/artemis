use super::{map::parse_map, property::parse_property};
use crate::{
    artifacts::os::macos::{artifacts::output_data, spotlight::dbstr::meta::SpotlightMeta},
    structs::toml::Output,
    utils::{
        nom_helper::{nom_unsigned_four_bytes, Endian},
        strings::extract_utf8_string,
    },
};
use log::error;
use nom::bytes::complete::take;

pub(crate) fn parse_store_db<'a>(
    data: &'a [u8],
    meta: &SpotlightMeta,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> nom::IResult<&'a [u8], ()> {
    let header_size: u16 = 4096;
    let (input, header_data) = take(header_size)(data)?;
    let (_, header) = parse_header(header_data)?;

    let (input, map_data) = take(header.map_size)(input)?;

    let (_, blocks) = parse_map(map_data)?;

    let offset_size = 0x1000;

    let limit = 1000;
    for block in blocks {
        let offset = block * offset_size;
        let (prop_data, _) = take(offset)(data)?;
        let (_, mut spotlight_data) = parse_property(prop_data, meta)?;

        println!("prop data len: {}", prop_data.len());
        println!("entries: {}", spotlight_data.len());

        let serde_data_result = serde_json::to_value(&spotlight_data);
        let serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[spotlight] Failed to serialize spotlight data: {err:?}");
                continue;
            }
        };
        let result = output_data(&serde_data, "spotlight", output, start_time, filter);
        if result.is_err() {
            error!(
                "[spotlight] Could not output spotlight data: {:?}",
                result.unwrap_err()
            );
        }
    }

    Ok((input, ()))
}

#[derive(Debug)]
struct StoreHeader {
    sig: u32,
    flags: u32,
    map_offset: u32,
    map_size: u32,
    page_size: u32,
    meta_attr_type_block_number: u32,
    meta_attr_value_block_number: u32,
    property_table_block_number: u32,
    meta_attr_list_block_number: u32,
    meta_attr_strings_block_number: u32,
    path: String,
}

fn parse_header(data: &[u8]) -> nom::IResult<&[u8], StoreHeader> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown4) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown5) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown6) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown7) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, map_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, map_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, page_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, meta_attr_type_block_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, meta_attr_value_block_number) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, property_table_block_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, meta_attr_list_block_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, meta_attr_strings_block_number) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let unknown_size: u16 = 256;
    let (input, _unknown8) = take(unknown_size)(input)?;
    let (input, path_data) = take(unknown_size)(input)?;
    let path = extract_utf8_string(path_data);

    let header = StoreHeader {
        sig,
        flags,
        map_offset,
        map_size,
        page_size,
        meta_attr_type_block_number,
        meta_attr_value_block_number,
        property_table_block_number,
        meta_attr_list_block_number,
        meta_attr_strings_block_number,
        path,
    };

    Ok((input, header))
}

#[cfg(test)]
mod tests {
    use super::{parse_header, parse_store_db};
    use crate::{
        artifacts::os::macos::spotlight::dbstr::meta::get_spotlight_meta,
        filesystem::{files::read_file, metadata::glob_paths},
        structs::toml::Output,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_parse_store_db() {
        let test = "/Users/puffycid/Downloads/store.db";
        let data = read_file(test).unwrap();
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();

        let meta = get_spotlight_meta(&paths).unwrap();
        let mut output = output_options("spotlight_test", "local", "./tmp", false);

        let (_, results) = parse_store_db(&data, &meta, &mut output, &0, &false).unwrap();
    }

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/header.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_header(&data).unwrap();
        assert_eq!(results.sig, 1685287992);
    }
}
