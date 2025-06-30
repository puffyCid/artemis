use super::{
    map::parse_map,
    property::{parse_property, property_header},
};
use crate::{
    artifacts::os::macos::{
        artifacts::output_data,
        spotlight::{dbstr::meta::SpotlightMeta, error::SpotlightError},
    },
    structs::toml::Output,
    utils::{
        nom_helper::{Endian, nom_unsigned_four_bytes},
        strings::extract_utf8_string,
    },
};
use common::macos::SpotlightEntries;
use log::error;
use nom::bytes::complete::take;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

/// Parse the Spotlight store.db and extract entries
pub(crate) async fn parse_store(
    reader: &mut File,
    meta: &SpotlightMeta,
    output: &mut Output,
    start_time: u64,
    filter: bool,
) -> Result<(), SpotlightError> {
    let (blocks, dir) = get_blocks(reader)?;
    let offset_size = 0x1000;
    let mut entries = Vec::new();

    // Spotlight contains a massive amount of metadata. To limit memory usage we dump our entries array once we hit 10,0000
    let limit = 10000;

    let prop_header_size = 20;
    for block in blocks {
        let offset = block * offset_size;
        if reader.seek(SeekFrom::Start(offset as u64)).is_err() {
            error!("[spotlight] Could not seek to store offset");
            continue;
        }

        let mut prop_header_data = vec![0; prop_header_size];
        if reader.read(&mut prop_header_data).is_err() {
            return Err(SpotlightError::StoreRead);
        };

        let prop_header_result = property_header(&prop_header_data);
        let prop_header = match prop_header_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[spotlight] Could not parse store prop header");
                continue;
            }
        };

        let mut prop_data = vec![0; prop_header.page_size as usize - prop_header_size];
        if reader.read(&mut prop_data).is_err() {
            return Err(SpotlightError::StoreRead);
        }

        let data_result = parse_property(&prop_data, meta, prop_header.uncompressed_size, &dir);
        let mut spotlight_data = match data_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[spotlight] Could not parse store prop");
                continue;
            }
        };

        entries.append(&mut spotlight_data);

        if entries.len() >= limit {
            let serde_data_result = serde_json::to_value(&entries);
            let mut serde_data = match serde_data_result {
                Ok(results) => results,
                Err(err) => {
                    error!("[spotlight] Failed to serialize spotlight data: {err:?}");
                    continue;
                }
            };
            let result =
                output_data(&mut serde_data, "spotlight", output, start_time, filter).await;
            if result.is_err() {
                error!(
                    "[spotlight] Could not output spotlight data: {:?}",
                    result.unwrap_err()
                );
            }

            entries = Vec::new();
        }
    }

    if !entries.is_empty() {
        let serde_data_result = serde_json::to_value(&entries);
        let mut serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[spotlight] Failed to serialize last spotlight data: {err:?}");
                return Err(SpotlightError::Serialize);
            }
        };
        let result = output_data(&mut serde_data, "spotlight", output, start_time, filter).await;
        if result.is_err() {
            error!(
                "[spotlight] Could not output last spotlight data: {:?}",
                result.unwrap_err()
            );
        }
    }

    Ok(())
}

/**
* Parse the Spotlight database in blocks. This allows for a **little** more flexible JS scripting.
* Instead of returning all Spotlight data (potentially 5GB+)
* This function instead will parse 10 blocks at a time and return the Spotlight data
*/
pub(crate) fn parse_store_blocks(
    reader: &mut File,
    meta: &SpotlightMeta,
    blocks: &[u32],
    offset: u32,
    dir: &str,
) -> Result<Vec<SpotlightEntries>, SpotlightError> {
    let offset_size = 0x1000;
    let mut entries = Vec::new();

    // Spotlight contains a massive amount of metadata. To limit memory usage we only parse 10 blocks at a time
    let limit = 10;

    let prop_header_size = 20;
    let mut count = 0;
    let mut start_offset = 0;
    for block in blocks {
        // Make sure we start at specific offset
        if start_offset < offset {
            start_offset += 1;
            continue;
        }

        if count == limit {
            break;
        }
        count += 1;

        let offset = block * offset_size;
        if reader.seek(SeekFrom::Start(offset as u64)).is_err() {
            error!("[spotlight] Could not seek to store offset");
            continue;
        }

        let mut prop_header_data = vec![0; prop_header_size];
        if reader.read(&mut prop_header_data).is_err() {
            return Err(SpotlightError::StoreRead);
        };

        let prop_header_result = property_header(&prop_header_data);
        let prop_header = match prop_header_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[spotlight] Could not parse store prop header");
                continue;
            }
        };

        let mut prop_data = vec![0; prop_header.page_size as usize - prop_header_size];
        if reader.read(&mut prop_data).is_err() {
            return Err(SpotlightError::StoreRead);
        }

        let data_result = parse_property(&prop_data, meta, prop_header.uncompressed_size, dir);
        let mut spotlight_data = match data_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[spotlight] Could not parse store prop");
                continue;
            }
        };

        entries.append(&mut spotlight_data);
    }

    Ok(entries)
}

/// Get blocks and spotlight directory associated with Spotlight store
pub(crate) fn get_blocks(reader: &mut File) -> Result<(Vec<u32>, String), SpotlightError> {
    let header_size = 4096;
    let mut header_data = vec![0; header_size];
    if reader.read(&mut header_data).is_err() {
        return Err(SpotlightError::StoreRead);
    }

    let header_result = parse_header(&header_data);
    let header = match header_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[spotlight] Could not parse store header");
            return Err(SpotlightError::StoreHeader);
        }
    };

    let mut map_data = vec![0; header.map_size as usize];
    if reader.read(&mut map_data).is_err() {
        return Err(SpotlightError::StoreRead);
    }

    let blocks_result = parse_map(&map_data);
    let blocks = match blocks_result {
        Ok((_, result)) => result,
        Err(_err) => {
            error!("[spotlight] Could not parse store map");
            return Err(SpotlightError::StoreMap);
        }
    };

    Ok((blocks, header.path))
}

#[derive(Debug)]
struct StoreHeader {
    _sig: u32,
    _flags: u32,
    _map_offset: u32,
    map_size: u32,
    _page_size: u32,
    _meta_attr_type_block_number: u32,
    _meta_attr_value_block_number: u32,
    _property_table_block_number: u32,
    _meta_attr_list_block_number: u32,
    _meta_attr_strings_block_number: u32,
    path: String,
}

/// Parse Store header info
fn parse_header(data: &[u8]) -> nom::IResult<&[u8], StoreHeader> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown2) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown4) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown5) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown6) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown7) = nom_unsigned_four_bytes(input, Endian::Le)?;

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
        _sig: sig,
        _flags: flags,
        _map_offset: map_offset,
        map_size,
        _page_size: page_size,
        _meta_attr_type_block_number: meta_attr_type_block_number,
        _meta_attr_value_block_number: meta_attr_value_block_number,
        _property_table_block_number: property_table_block_number,
        _meta_attr_list_block_number: meta_attr_list_block_number,
        _meta_attr_strings_block_number: meta_attr_strings_block_number,
        path,
    };

    Ok((input, header))
}

#[cfg(test)]
mod tests {
    use super::{get_blocks, parse_header, parse_store, parse_store_blocks};
    use crate::{
        artifacts::os::macos::spotlight::dbstr::meta::get_spotlight_meta,
        filesystem::{
            files::{file_reader, read_file},
            metadata::glob_paths,
        },
        structs::toml::Output,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
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

    #[tokio::test]
    async fn test_parse_store_db() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();
        test_location.pop();
        test_location.push("store.db");

        let mut data = file_reader(test_location.to_str().unwrap()).unwrap();

        let meta = get_spotlight_meta(&paths).unwrap();
        let mut output = output_options("spotlight_test", "local", "./tmp", false);

        parse_store(&mut data, &meta, &mut output, 0, false)
            .await
            .unwrap();
    }

    #[test]
    fn test_parse_store_blocks() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();
        test_location.pop();
        test_location.push("store.db");

        let mut data = file_reader(test_location.to_str().unwrap()).unwrap();

        let meta = get_spotlight_meta(&paths).unwrap();
        let (blocks, dir) = get_blocks(&mut data).unwrap();

        let entries = parse_store_blocks(&mut data, &meta, &blocks, 0, &dir).unwrap();
        assert_eq!(entries.len(), 1022);
        assert_eq!(entries[0].inode, 1);
    }

    #[test]
    fn test_get_blocks() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/store.db");

        let mut data = file_reader(test_location.to_str().unwrap()).unwrap();

        let (result, dir) = get_blocks(&mut data).unwrap();
        assert_eq!(result.len(), 714);
        assert_eq!(
            dir,
            "/System/Volumes/Data/.Spotlight-V100/Store-V2/32D12D36-11C0-4B7C-B98D-99D23D5544E2/store.db"
        );
    }

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/header.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_header(&data).unwrap();
        assert_eq!(results._sig, 1685287992);
    }
}
