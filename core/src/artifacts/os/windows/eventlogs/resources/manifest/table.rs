use super::crimson::ManifestTemplate;
use crate::utils::{
    nom_helper::{nom_signed_four_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf16_string,
    uuid::format_guid_le_bytes,
};
use log::warn;
use nom::bytes::complete::take;

pub(crate) fn parse_table<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    template: &mut ManifestTemplate,
) -> nom::IResult<&'a [u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, template_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    println!("temp count: {template_count}");
    while count < template_count {
        let (remaining, _) = parse_template(input, template)?;
        input = remaining;
        count += 1;
    }
    Ok((input, ()))
}

fn parse_template<'a>(
    data: &'a [u8],
    template: &mut ManifestTemplate,
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
    let (input, name_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, template_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    // Possibly EventType. Not 100% sure
    let (input, event_type_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let guid_size: u8 = 16;
    let (input, guid_bytes) = take(guid_size)(input)?;

    let guid = format_guid_le_bytes(guid_bytes);
    let event_type = get_event_type(&event_type_data);
    println!("template GUID: {guid} - EventType: {event_type:?}");

    // Binary XML remaining
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
    use super::{parse_table, parse_template};
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
        let (_input, _) = parse_table(&data, table_start, manifest).unwrap();
    }
}
