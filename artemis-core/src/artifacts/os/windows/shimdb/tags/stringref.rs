use crate::{
    artifacts::os::windows::shimdb::tag::{get_tag, Tags},
    utils::{
        nom_helper::{nom_unsigned_four_bytes, Endian},
        strings::extract_utf16_string,
    },
};
use log::warn;
use nom::{bytes::complete::take, Needed};

/// Grab and return string from stringtable based on the parsed stringref value
pub(crate) fn parse_stringref<'a>(
    data: &'a [u8],
    stringtable_data: &'a [u8],
) -> nom::IResult<&'a [u8], String> {
    let (input, offset) = nom_unsigned_four_bytes(data, Endian::Le)?;

    // Offset is based on offset of the stringtable list
    if offset as usize > stringtable_data.len() {
        warn!("[shimdb] String ref offset larger than stringtable. Cannot do string lookups");
        return Ok((input, format!("Offset too large for stringtable: {offset}")));
    }

    // We already nommed the first 6 bytes of the stringtable to determine the list (stringtable) and to get the size
    let adjust_offset = 6;
    if offset < adjust_offset {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }
    let (string_entry, _) = take(offset - adjust_offset)(stringtable_data)?;

    // We should now be at the start of the STRING tag associated with the stringref
    let (string_entry, (tag, tag_value)) = get_tag(string_entry)?;
    if tag != Tags::String {
        warn!("[shimdb] Stringtable contained a tag other than STRING. Cannot do string lookups");
        return Ok((
            input,
            format!("Incorrect tag value in stringtable: {tag_value}"),
        ));
    }

    let (string_entry, string_size) = nom_unsigned_four_bytes(string_entry, Endian::Le)?;

    if string_size as usize > string_entry.len() {
        warn!("[shimdb] String size larger than stringtable. Cannot do string lookups");
        return Ok((
            input,
            format!(
                "String size too large for stringtable, size: {string_size}, stringtable size: {}",
                stringtable_data.len()
            ),
        ));
    }

    let (_, string_data) = take(string_size)(string_entry)?;
    let value = extract_utf16_string(string_data);
    Ok((input, value))
}

#[cfg(test)]
mod tests {
    use super::parse_stringref;

    #[test]
    fn test_parse_stringref() {
        let offset_test = [6, 0, 0, 0];
        let table_test = [
            1, 136, 16, 0, 0, 0, 51, 0, 46, 0, 48, 0, 46, 0, 48, 0, 46, 0, 57, 0, 0, 0,
        ];
        let (_, result) = parse_stringref(&offset_test, &table_test).unwrap();

        assert_eq!(result, "3.0.0.9")
    }

    #[test]
    fn test_parse_stringref_bad_string_size() {
        let offset_test = [6, 0, 0, 0];
        let table_test = [
            1, 136, 25, 0, 0, 0, 51, 0, 46, 0, 48, 0, 46, 0, 48, 0, 46, 0, 57, 0, 0, 0,
        ];
        let (_, result) = parse_stringref(&offset_test, &table_test).unwrap();

        assert_eq!(
            result,
            "String size too large for stringtable, size: 25, stringtable size: 22"
        )
    }

    #[test]
    fn test_parse_stringref_bad_tag() {
        let offset_test = [6, 0, 0, 0];
        let table_test = [
            1, 1, 11, 136, 25, 0, 0, 0, 51, 0, 46, 0, 48, 0, 46, 0, 48, 0, 46, 0, 57, 0, 0, 0,
        ];
        let (_, result) = parse_stringref(&offset_test, &table_test).unwrap();

        assert_eq!(result, "Incorrect tag value in stringtable: 0")
    }

    #[test]
    fn test_parse_stringref_bad_offset() {
        let offset_test = [99, 0, 0, 0];
        let table_test = [
            1, 1, 11, 136, 25, 0, 0, 0, 51, 0, 46, 0, 48, 0, 46, 0, 48, 0, 46, 0, 57, 0, 0, 0,
        ];
        let (_, result) = parse_stringref(&offset_test, &table_test).unwrap();

        assert_eq!(result, "Offset too large for stringtable: 99")
    }
}
