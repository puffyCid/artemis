use crate::utils::{
    nom_helper::{
        nom_signed_four_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf8_string,
};
use nom::bytes::complete::{take, take_while};
use std::collections::HashMap;

/// Parse and gather index entries
pub(crate) fn parse_index(data: &[u8]) -> nom::IResult<&[u8], HashMap<u32, IndexBody>> {
    let page_size = 8192;
    let mut input = data;

    let mut page_info = HashMap::new();

    while input.len() >= page_size {
        let (remaining, page_data) = take(page_size)(input)?;
        let (_, (header, body)) = parse_page(page_data)?;

        input = remaining;
        if header.page_type == PageType::Deleted || header.page_type == PageType::Unknown {
            panic!("hmm? {:?}", header.page_type);
        }
        page_info.insert(header.mapped_number, body);
    }
    Ok((data, page_info))
}

/// Parse Index page data
fn parse_page(data: &[u8]) -> nom::IResult<&[u8], (IndexHeader, IndexBody)> {
    let (input, header) = parse_header(data)?;
    let (input, body) = parse_body(input)?;
    Ok((input, (header, body)))
}

struct IndexHeader {
    page_type: PageType,
    mapped_number: u32,
    _unknown: u32,
    _mapped_root_number: u32,
}

#[derive(PartialEq, Debug)]
enum PageType {
    Active,
    Adminstrative,
    Deleted,
    Unknown,
}

/// Parse the Index.btr page header
fn parse_header(data: &[u8]) -> nom::IResult<&[u8], IndexHeader> {
    let (input, type_data) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, mapped_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, mapped_root_number) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let page_type = match type_data {
        0xaccc => PageType::Active,
        0xaddd => PageType::Adminstrative,
        0xbadd => PageType::Deleted,
        _ => PageType::Unknown,
    };

    let header = IndexHeader {
        page_type,
        mapped_number,
        _unknown: unknown,
        _mapped_root_number: mapped_root_number,
    };

    Ok((input, header))
}

#[derive(Debug)]
pub(crate) struct IndexBody {
    array_sub_pages: Vec<i32>,
    array_key_offset: Vec<u16>,
    key_data: Vec<u16>,
    array_value_offsets: Vec<u16>,
    pub(crate) value_data: Vec<String>,
}

/// Parse the Index body
fn parse_body(data: &[u8]) -> nom::IResult<&[u8], IndexBody> {
    let (input, number_keys) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let size = 4;
    let (input, _array_unknown) = take(number_keys * size)(input)?;

    let mut body = IndexBody {
        array_sub_pages: Vec::new(),
        array_key_offset: Vec::new(),
        key_data: Vec::new(),
        array_value_offsets: Vec::new(),
        value_data: Vec::new(),
    };

    let (input, mut sub_pages_data) = take((number_keys + 1) * size)(input)?;
    // Get sub pages values
    while sub_pages_data.len() >= size as usize {
        let (page, subpage) = nom_signed_four_bytes(sub_pages_data, Endian::Le)?;
        body.array_sub_pages.push(subpage);
        sub_pages_data = page;
    }

    let key_size = 2;
    let (input, mut key_offsets_data) = take(number_keys * key_size)(input)?;
    while key_offsets_data.len() >= key_size as usize {
        let (key, key_offset) = nom_unsigned_two_bytes(key_offsets_data, Endian::Le)?;
        body.array_key_offset.push(key_offset);
        key_offsets_data = key;
    }

    let (input, number_keys) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, mut key_indexes_data) = take(number_keys * key_size as u16)(input)?;
    while key_indexes_data.len() >= key_size as usize {
        let (key, key_data) = nom_unsigned_two_bytes(key_indexes_data, Endian::Le)?;
        body.key_data.push(key_data);
        key_indexes_data = key;
    }

    let (input, number_values) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, mut value_array_data) = take(number_values * key_size as u16)(input)?;
    while value_array_data.len() >= key_size as usize {
        let (value, value_data) = nom_unsigned_two_bytes(value_array_data, Endian::Le)?;
        body.array_value_offsets.push(value_data);
        value_array_data = value;
    }

    let (input, value_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, mut value_data) = take(value_size)(input)?;

    while value_data.len() > 1 {
        let (input, string_data) = take_while(|b| b != 0)(value_data)?;
        // Nom the zero
        let (input, _eol) = nom_unsigned_one_byte(input, Endian::Le)?;

        let value = extract_utf8_string(string_data);
        body.value_data.push(value);
        value_data = input;
    }

    Ok((input, body))
}

#[cfg(test)]
mod tests {
    use super::{parse_header, parse_index};
    use crate::{artifacts::os::windows::wmi::index::PageType, filesystem::files::read_file};
    use std::path::PathBuf;

    #[test]
    fn test_parse_header() {
        let data = [204, 172, 0, 0, 34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0];
        let (_, header) = parse_header(&data).unwrap();
        assert_eq!(header.page_type, PageType::Active);
        assert_eq!(header.mapped_number, 34);
    }

    #[test]
    fn test_parse_index() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/wmi/index_page.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_index(&data).unwrap();

        assert_eq!(results.get(&34).unwrap().value_data.len(), 46);
    }

    #[test]
    fn test_parse_index_file() {
        let data = read_file("C:\\Windows\\System32\\wbem\\Repository\\INDEX.BTR").unwrap();
        let (_, results) = parse_index(&data).unwrap();

        assert!(results.len() > 10);
    }
}
