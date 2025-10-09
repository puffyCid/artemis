use super::xml::TemplateElement;
use crate::{
    artifacts::os::windows::eventlogs::resources::manifest::xml::parse_xml,
    utils::{
        nom_helper::{
            Endian, nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes,
        },
        uuid::format_guid_le_bytes,
    },
};
use log::warn;
use nom::bytes::complete::{take, take_while};

/// Parse template containing binary XML
pub(crate) fn parse_template(data: &[u8]) -> nom::IResult<&[u8], TemplateElement> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size includes sig and size itself
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let adjust_size = 8;
    if adjust_size > size {
        warn!("[eventlogs] Template size is too small: {size}. Ending parsing");
        let temp = TemplateElement {
            template_id: String::new(),
            event_data_type: String::new(),
            elements: Vec::new(),
            guid: String::new(),
        };
        return Ok((&[], temp));
    }
    let (remaing_template, input) = take(size - adjust_size)(input)?;
    let (input, descriptor_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _name_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _template_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    // Possibly EventType. Not 100% sure
    let (input, event_type_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let guid_size: u8 = 16;
    let (input, guid_bytes) = take(guid_size)(input)?;

    let guid = format_guid_le_bytes(guid_bytes);
    let _event_type = get_event_type(event_type_data);
    // Binary XML slightly different from EVTX files
    let (remaining, template) = parse_xml(input, guid)?;
    let (input, _padding) = take_while(|b: u8| b == 0)(remaining)?;
    if descriptor_count != 0 {
        // Get first descriptor
        let (input, _input_type) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _output_type) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, _values_count) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _value_data_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        // Offset is from the start of the file
        let (_desc_input, _template_name_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    }

    // Remaining parts is just Template item descriptors and Template item names. Not needed? Skipping for now
    Ok((remaing_template, template))
}

#[derive(Debug, PartialEq)]
enum EventType {
    EventData,
    UserData,
    DebugData,
    BinaryEventData,
    ProcessingErrorData,
    Unknown,
}

/// Get `EventLog` data type
fn get_event_type(event: u32) -> EventType {
    match event {
        1 => EventType::EventData,
        2 => EventType::UserData,
        3 => EventType::DebugData,
        4 => EventType::BinaryEventData,
        5 => EventType::ProcessingErrorData,
        _ => EventType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_template;
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::table::{
            EventType, get_event_type,
        },
        filesystem::files::read_file,
        utils::nom_helper::nom_data,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_template() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let start = 472;
        let (table_start, _) = nom_data(&data, start).unwrap();

        let (_, result) = parse_template(&table_start).unwrap();
        assert_eq!(result.elements.len(), 17);
    }

    #[test]
    fn test_get_event_type() {
        assert_eq!(get_event_type(3), EventType::DebugData);
    }
}
