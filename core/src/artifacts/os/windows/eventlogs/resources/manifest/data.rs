use super::wevt::SigType;
use crate::utils::{
    nom_helper::{
        nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian,
    },
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct ManifestData {
    pub(crate) message_id: i32,
    /**Bitmask? */
    pub(crate) id: u64,
    pub(crate) value: String,
}

/// Parse similar template data signatures
pub(crate) fn parse_manifest_data<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    sig_type: &SigType,
) -> nom::IResult<&'a [u8], Vec<ManifestData>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let empty = 0;
    if size == empty {
        return Ok((input, Vec::new()));
    }

    let (input, data_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    // Only some Signature types have similar format
    match sig_type {
        SigType::Chan => parse_channel(resource, input, &data_count),
        SigType::Levl | SigType::Opco => parse_opcode(resource, input, &data_count),
        SigType::Keyw => parse_keyword(resource, input, &data_count),
        _ => Ok((input, Vec::new())),
    }
}

/// Parse Opcode data
fn parse_opcode<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    data_count: &u32,
) -> nom::IResult<&'a [u8], Vec<ManifestData>> {
    let mut input = data;
    let mut count = 0;
    let mut data_vec = Vec::new();
    while count < *data_count {
        let (remaining, id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        let (string_start, _) = take(offset)(resource)?;
        let (string_data, size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

        let adjust_size = 4;
        if adjust_size > size {
            // Should not happen
            break;
        }
        // Size includes size itself
        let (_, value_data) = take(size - adjust_size)(string_data)?;

        let value = extract_utf16_string(value_data);

        let manifest: ManifestData = ManifestData {
            message_id,
            id: id as u64,
            value,
        };

        data_vec.push(manifest);
    }

    Ok((input, data_vec))
}

/// Parse Channel data
fn parse_channel<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    data_count: &u32,
) -> nom::IResult<&'a [u8], Vec<ManifestData>> {
    let mut input = data;
    let mut count = 0;
    let mut data_vec = Vec::new();

    while count < *data_count {
        let (remaining, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        let (string_start, _) = take(offset)(resource)?;
        let (string_data, size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

        let adjust_size = 4;
        if adjust_size > size {
            // Should not happen
            continue;
        }
        // Size includes size itself
        let (_, value_data) = take(size - adjust_size)(string_data)?;

        let value = extract_utf16_string(value_data);

        let manifest: ManifestData = ManifestData {
            message_id,
            id: id as u64,
            value,
        };

        data_vec.push(manifest);
    }

    Ok((input, data_vec))
}

/// Parse Keyword data
fn parse_keyword<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    data_count: &u32,
) -> nom::IResult<&'a [u8], Vec<ManifestData>> {
    let mut input = data;
    let mut count = 0;
    let mut data_vec = Vec::new();

    while count < *data_count {
        let (remaining, id) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        let (string_start, _) = take(offset)(resource)?;
        let (string_data, size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

        let adjust_size = 4;
        if adjust_size > size {
            // Should not happen
            continue;
        }
        // Size includes size itself
        let (_, value_data) = take(size - adjust_size)(string_data)?;

        let value = extract_utf16_string(value_data);

        let manifest = ManifestData {
            message_id,
            id,
            value,
        };

        data_vec.push(manifest);
    }

    Ok((input, data_vec))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::{
            data::parse_manifest_data, wevt::SigType,
        },
        filesystem::files::read_file,
        utils::nom_helper::nom_data,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_manifest_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let start = 12368;
        let (table_start, _) = nom_data(&data, start).unwrap();
        let (_input, opcodes) = parse_manifest_data(&data, table_start, &SigType::Opco).unwrap();
        assert_eq!(opcodes.len(), 8);
        assert_eq!(opcodes[5].value, "Retry");
        assert_eq!(opcodes[5].message_id, 805306472);
        assert_eq!(opcodes[5].id, 6815744);
    }
}
