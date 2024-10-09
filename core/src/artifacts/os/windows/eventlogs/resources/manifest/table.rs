use super::crimson::ManifestTemplate;
use crate::{
    artifacts::os::windows::eventlogs::resources::manifest::{self, xml::parse_xml},
    utils::{
        nom_helper::{
            nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
        },
        uuid::format_guid_le_bytes,
    },
};
use log::warn;
use nom::bytes::complete::{take, take_while};

pub(crate) fn parse_table<'a>(
    data: &'a [u8],
    manifest: &mut ManifestTemplate,
) -> nom::IResult<&'a [u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, template_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    while count < template_count {
        let (remaining, _) = parse_template(input, manifest)?;
        input = remaining;
        count += 1;
    }
    Ok((input, ()))
}

fn parse_template<'a>(
    data: &'a [u8],
    manifest: &mut ManifestTemplate,
) -> nom::IResult<&'a [u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size includes sig and size itself
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let adjust_size = 8;
    if adjust_size > size {
        panic!("[eventlogs] Template size is too small: {size}. Ending parsing");
        return Ok((&[], ()));
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
    let _event_type = get_event_type(&event_type_data);

    // Binary XML slightly different from EVTX files
    let (remaining, template) = parse_xml(input, guid)?;
    manifest.templates.push(template);
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
    Ok((remaing_template, ()))
}

#[derive(Debug)]
enum EventType {
    EventData,
    UserData,
    DebugData,
    BinaryEventData,
    ProcessingErrorData,
    Unknown,
}

fn get_event_type(event: &u32) -> EventType {
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
    use super::parse_table;
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::{
            channel::parse_channel, crimson::parse_manifest, provider::parse_provider,
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
        let (input, mut template) = parse_manifest(&data).unwrap();
        let manifest = template
            .get_mut("9799276c-fb04-47e8-845e-36946045c218")
            .unwrap();

        let (input, _) = parse_provider(input, manifest).unwrap();
        let _ = parse_channel(&data, input, manifest).unwrap();

        let start = 0x1cc;
        let (table_start, _) = nom_data(&data, start).unwrap();
        let (_input, _) = parse_table(table_start, manifest).unwrap();
        assert_eq!(manifest.templates.len(), 9);
    }
}
