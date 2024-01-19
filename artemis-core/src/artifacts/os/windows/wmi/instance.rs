// https://github.com/libyal/dtformats/blob/main/documentation/WMI%20repository%20file%20format.asciidoc#instance-object-record---version-22
// See https://github.com/libyal/dtformats/blob/main/documentation/WMI%20repository%20file%20format.asciidoc#instance_block

use std::mem::size_of;
use nom::bytes::complete::take;
use serde_json::Value;
use crate::utils::{
    nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf16_string,
};
use super::class::{CimType, Qualifier};

#[derive(Debug)]
pub(crate) struct InstanceRecord {
    hash_name: String,
    unknown_filetime: u64,
    unknown_filetime2: u64,
    property_values: Vec<u8>,
    qualifier: Qualifier,
    block_type: u8,
    dynamic_properties: Vec<u8>,
    values: Vec<u8>,
    data: Vec<u8>
}

pub(crate) fn parse_instance_record(data: &[u8]) -> nom::IResult<&[u8], InstanceRecord> {
    let hash_size:u8 = 128;
    let (input, hash_data) = take(hash_size)(data)?;
    let hash_name = extract_utf16_string(hash_data);
    println!("{hash_name}");

    let (input, unknown_filetime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, unknown_filetime2) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let (input, block_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_block = 4;
    // Size includes block size itself. Which has already been nom'd
    let (input, block_data) = take(block_size-adjust_block)(input)?;

    let mut instance = InstanceRecord {
        hash_name,
        unknown_filetime,
        unknown_filetime2,
        property_values: Vec::new(),
        qualifier: Qualifier {
            name: String::new(),
            value_data_type: CimType::Unknown,
            data: Value::Null,
        },
        block_type: 0,
        dynamic_properties: Vec::new(),
        values: Vec::new(),
        data: block_data.to_vec(),
    };

    Ok((input, instance))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use crate::filesystem::files::read_file;
    use super::parse_instance_record;

    #[test]
    fn test_parse_objects() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/instance.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_instance_record(&data).unwrap();

    }
}