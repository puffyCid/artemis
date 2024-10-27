use super::data::ManifestData;
use crate::utils::{
    nom_helper::{nom_signed_four_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf16_string,
};
use nom::bytes::complete::{take, take_while};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct MapInfo {
    pub(crate) name: String,
    pub(crate) data: HashMap<u32, ManifestData>,
}

/// Parse map entries from `WEVE_TEMPLATE`. Can be used to lookup strings in `MESSAGETABLE`
pub(crate) fn parse_map<'a>(
    resource: &'a [u8],
    data: &'a [u8],
) -> nom::IResult<&'a [u8], Vec<MapInfo>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let empty = 0;
    if size == empty {
        return Ok((input, Vec::new()));
    }

    let (mut input, data_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let mut offsets = Vec::new();

    let mut count = 0;
    while count < data_count {
        let (remaining, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        input = remaining;
        offsets.push(offset);
        count += 1;
    }

    let mut maps = Vec::new();
    for offset in offsets {
        let (map_start, _) = take(offset)(resource)?;
        let (input, _sig) = nom_unsigned_four_bytes(map_start, Endian::Le)?;
        let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let empty = 0;
        if size == empty {
            continue;
        }

        let (input, name_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (name_start, _) = take(name_offset)(resource)?;
        let (string_data, string_size) = nom_unsigned_four_bytes(name_start, Endian::Le)?;
        let adjust_size = 4;

        if string_size < adjust_size {
            continue;
        }
        let (_, string_data) = take(string_size - adjust_size)(string_data)?;
        let name = extract_utf16_string(string_data);

        let mut map = MapInfo {
            name,
            data: HashMap::new(),
        };

        // Padding? or unknown value?
        let (input, _) = take_while(|b| b == 0)(input)?;

        let (mut input, entries_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let mut entry_count = 0;
        while entry_count < entries_count {
            let (remaining, id) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;

            let manifest = ManifestData {
                id: id as u64,
                message_id,
                value: String::new(),
            };
            input = remaining;
            entry_count += 1;
            map.data.insert(id, manifest);
        }

        maps.push(map);
    }

    Ok((&[], maps))
}

#[cfg(test)]
mod tests {
    use super::parse_map;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_map() {
        let test = [
            77, 65, 80, 83, 240, 0, 0, 0, 1, 0, 0, 0, 16, 1, 0, 0, 86, 77, 65, 80, 188, 0, 0, 0,
            204, 1, 0, 0, 0, 0, 0, 0, 21, 0, 0, 0, 8, 19, 0, 0, 15, 0, 0, 208, 24, 19, 0, 0, 14, 0,
            0, 208, 72, 19, 0, 0, 13, 0, 0, 208, 104, 19, 0, 0, 12, 0, 0, 208, 136, 19, 0, 0, 1, 0,
            0, 208, 141, 19, 0, 0, 2, 0, 0, 208, 152, 19, 0, 0, 3, 0, 0, 208, 168, 19, 0, 0, 4, 0,
            0, 208, 184, 19, 0, 0, 5, 0, 0, 208, 200, 19, 0, 0, 6, 0, 0, 208, 216, 19, 0, 0, 7, 0,
            0, 208, 232, 19, 0, 0, 8, 0, 0, 208, 237, 19, 0, 0, 9, 0, 0, 208, 248, 19, 0, 0, 10, 0,
            0, 208, 8, 20, 0, 0, 11, 0, 0, 208, 136, 35, 0, 0, 16, 0, 0, 208, 137, 35, 0, 0, 17, 0,
            0, 208, 138, 35, 0, 0, 18, 0, 0, 208, 139, 35, 0, 0, 19, 0, 0, 208, 140, 35, 0, 0, 20,
            0, 0, 208, 141, 35, 0, 0, 21, 0, 0, 208, 36, 0, 0, 0, 73, 0, 110, 0, 115, 0, 116, 0,
            97, 0, 108, 0, 108, 0, 83, 0, 116, 0, 97, 0, 116, 0, 101, 0, 77, 0, 97, 0, 112, 0, 0,
            0, 84, 84, 66, 76, 252, 34, 0, 0, 12, 0, 0, 0, 84, 69, 77, 80, 200, 4, 0, 0, 9, 0, 0,
            0, 9, 0, 0, 0, 208, 4, 0, 0, 2, 0, 0, 0, 53,
        ];

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/cbsmsg_wevt.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, map) = parse_map(&data, &test).unwrap();
        assert_eq!(map[0].name, "InstallStateMap")
    }
}
