use crate::utils::{
    nom_helper::{
        nom_signed_two_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct TemplateElement {
    pub(crate) template_id: String,
    pub(crate) event_data_type: String,
    pub(crate) elements: Vec<Element>,
    pub(crate) guid: String,
}

/// Parse binary xml data to get template info
/**
* Output is similar to
```xml
<template tid="ClassArgs_V1">
 <data name="DeviceGUID" inType="win:GUID"/>
 <data name="DeviceNumber" inType="win:UInt32"/>
 <data name="Model" inType="win:AnsiString"/>
 <data name="FirmwareVersion" inType="win:AnsiString"/>
 <data name="SerialNumber" inType="win:AnsiString"/>
 <data name="RequestDuration_100ns" inType="win:UInt64"/>
 <data name="Irp" inType="win:Pointer"/>
 <data name="CommandType" inType="win:UnicodeString"/>
 <data name="CommandTag" inType="win:UInt32"/>
 <data name="NTStatus" inType="win:HexInt32"/>
 <data name="Command" inType="win:HexInt32"/>
 <data name="CDW10" inType="win:HexInt32"/>
 <data name="CDW11" inType="win:HexInt32"/>
 <data name="CDW12" inType="win:HexInt32"/>
 <data name="CDW13" inType="win:HexInt32"/>
 <data name="CDW14" inType="win:HexInt32"/>
 <data name="CDW15" inType="win:HexInt32"/>
</template>
```
except as `TemplateElement` struct
*/
pub(crate) fn parse_xml(data: &[u8], guid: String) -> nom::IResult<&[u8], TemplateElement> {
    let (input, _token) = fragment_header(data)?;

    // First element is the start header/tag?
    // remaining is the remaining bytes of the template. Will get parsed by parse_template
    let (remaining, (start, mut input)) = element_start(input, &false)?;

    let next_element = 0x41;
    let next_element2 = 0x1;
    let mut template_elements = Vec::new();
    while input
        .first()
        .is_some_and(|x| *x == next_element || *x == next_element2)
    {
        let (remaining, (element, _)) = element_start(input, &false)?;
        input = remaining;
        template_elements.push(element);
    }

    let template = TemplateElement {
        template_id: String::new(),
        event_data_type: start.element_name,
        elements: template_elements,
        guid,
    };

    Ok((remaining, template))
}

/// Parse binary xml header
fn fragment_header(data: &[u8]) -> nom::IResult<&[u8], TokenType> {
    let (input, token) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, _major_version) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, _minor_version) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, _flags) = nom_unsigned_one_byte(input, Endian::Le)?;

    Ok((input, get_token_type(&token)))
}

#[derive(Debug)]
pub(crate) struct Element {
    pub(crate) token: TokenType,
    pub(crate) token_number: u8,
    pub(crate) depedency_id: i16,
    pub(crate) size: u32,
    pub(crate) attribute_list: Vec<Attribute>,
    pub(crate) element_name: String,
}

/// Start parsing elements
fn element_start<'a>(
    data: &'a [u8],
    is_substituion: &bool,
) -> nom::IResult<&'a [u8], (Element, &'a [u8])> {
    let (mut input, token_number) = nom_unsigned_one_byte(data, Endian::Le)?;
    let mut start = Element {
        token: get_token_type(&token_number),
        token_number,
        depedency_id: 0,
        size: 0,
        attribute_list: Vec::new(),
        element_name: String::new(),
    };

    if !*is_substituion {
        let (remaining, depedency_id) = nom_signed_two_bytes(input, Endian::Le)?;
        input = remaining;
        start.depedency_id = depedency_id;
    }

    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    start.size = size;

    let (remaining, element_data) = take(start.size)(input)?;
    let (input, name) = get_name(element_data)?;

    start.element_name = name;

    let no_elements = 0x1;
    if start.token_number == no_elements {
        // We are done. Just get the closing element tag (0x2)
        let (input, _end_element) = nom_unsigned_one_byte(input, Endian::Le)?;
        return Ok((remaining, (start, input)));
    }

    // If token is 0x41, we have attributes to get
    let (data_remaining, attributes) = attribute_list(input)?;
    start.attribute_list = attributes;

    Ok((remaining, (start, data_remaining)))
}

#[derive(Debug)]
pub(crate) struct Attribute {
    pub(crate) attribute_token: TokenType,
    pub(crate) attribute_token_number: u8,
    pub(crate) value: String,
    pub(crate) value_token: TokenType,
    pub(crate) value_token_number: u8,
    pub(crate) name: String,
    pub(crate) input_type: InputType,
    pub(crate) substitution: TokenType,
    pub(crate) substitution_id: u16,
}

/// Attempt to get attributes for the element
fn attribute_list(data: &[u8]) -> nom::IResult<&[u8], Vec<Attribute>> {
    let (remaining_input, size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let ending_data = 6;
    let (mut remaining, mut attribute_data) =
        if (size + ending_data) as usize > remaining_input.len() {
            take(size)(remaining_input)?
        } else {
            take(size + ending_data)(remaining_input)?
        };

    let last_attribute = 0x6;
    let next_attribute = 0x46;

    let next_value = 0x45;

    let mut attributes = Vec::new();
    let min_size = 10;
    while attribute_data.len() > min_size {
        let (input, attribute_token_number) = nom_unsigned_one_byte(attribute_data, Endian::Le)?;
        let (input, name) = get_name(input)?;

        let (input, mut value_token_number) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, value_token_type_number) = nom_unsigned_one_byte(input, Endian::Le)?;
        let value_token_type = get_input_type(&value_token_type_number);

        // This should always be a Unicode type (per: https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#4110-value-text)
        if value_token_type != InputType::Unicode {
            break;
        }
        let (input, value_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let utf16 = 2;
        let (mut value_input, value_data) = take(value_size * utf16)(input)?;
        let mut value = extract_utf16_string(value_data);

        if value_input.is_empty() {
            let attribute = Attribute {
                attribute_token: get_token_type(&attribute_token_number),
                attribute_token_number,
                value,
                value_token: get_token_type(&value_token_number),
                value_token_number,
                name,
                input_type: InputType::Unknown,
                substitution: TokenType::Unknown,
                substitution_id: 0,
            };
            attributes.push(attribute);
            break;
        }

        // Check if we have another value. Have not seen this yet.
        while value_token_number == next_value {
            let (input, next_value_token_number) = nom_unsigned_one_byte(value_input, Endian::Le)?;
            let (input, value_token_type_number) = nom_unsigned_one_byte(input, Endian::Le)?;
            let value_token_type = get_input_type(&value_token_type_number);

            // This should always be a Unicode type (per: https://github.com/libyal/libevtx/blob/main/documentation/Windows%20XML%20Event%20Log%20(EVTX).asciidoc#4110-value-text)
            if value_token_type != InputType::Unicode {
                break;
            }
            let (input, value_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

            let utf16 = 2;
            let (input, value_data) = take(value_size * utf16)(input)?;
            let next_value = extract_utf16_string(value_data);

            // Unsure what this would look like if there are multiple values.
            // This suppose to be psuedo XML
            // ex: <data name="Irp" inType="win:Pointer"/>
            // Where "Irp" is the value
            value = format!("{value};{next_value}");

            value_input = input;
            value_token_number = next_value_token_number;
        }

        if attribute_token_number == next_attribute {
            attribute_data = value_input;
            continue;
        }

        // Is this the ending tag? Seems to always be 0x2
        let (input, _end) = nom_unsigned_one_byte(value_input, Endian::Le)?;
        let (input, substitution_type) = nom_unsigned_one_byte(input, Endian::Le)?;
        let is_substitution = get_token_type(&substitution_type);

        if is_substitution == TokenType::EndElement {
            let attribute = Attribute {
                attribute_token: get_token_type(&attribute_token_number),
                attribute_token_number,
                value,
                value_token: get_token_type(&value_token_number),
                value_token_number,
                name,
                input_type: InputType::Unknown,
                substitution: TokenType::Unknown,
                substitution_id: 0,
            };
            attributes.push(attribute);
            break;
        } else if is_substitution == TokenType::OpenStartElement {
            let attribute = Attribute {
                attribute_token: get_token_type(&attribute_token_number),
                attribute_token_number,
                value,
                value_token: get_token_type(&value_token_number),
                value_token_number,
                name,
                input_type: InputType::Unknown,
                substitution: TokenType::Unknown,
                substitution_id: 0,
            };
            attributes.push(attribute);

            // We were wrong about the remaining data above. Get the actual remaining bytes now
            let (real_remaining, _) = take(size)(remaining_input)?;
            let (real_remaining, _end) = nom_unsigned_one_byte(real_remaining, Endian::Le)?;

            remaining = real_remaining;
            break;
        }

        let (input, substitution_id) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, input_type_data) = nom_unsigned_one_byte(input, Endian::Le)?;

        // Is this ending element tag? Seems to always be 0x4
        let (input, _end_element) = nom_unsigned_one_byte(input, Endian::Le)?;

        let attribute = Attribute {
            attribute_token: get_token_type(&attribute_token_number),
            attribute_token_number,
            value,
            value_token: get_token_type(&value_token_number),
            value_token_number,
            name,
            input_type: get_input_type(&input_type_data),
            substitution: is_substitution,
            substitution_id,
        };

        attributes.push(attribute);

        // If the token is something other last_attribute or next_attribute. Something went wrong
        if attribute_token_number == last_attribute || attribute_token_number != next_attribute {
            break;
        }
        attribute_data = input;
    }

    Ok((remaining, attributes))
}

/// Extract strings
fn get_name(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, _name_hash) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (input, name_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let adjust_size = 2;
    let utf16 = 2;

    let (input, name_data) = take(name_size * utf16 + adjust_size)(input)?;
    let name = extract_utf16_string(name_data);

    Ok((input, name))
}

#[derive(Debug, PartialEq)]
pub(crate) enum TokenType {
    Eof,
    OpenStartElement,
    CloseStartElement,
    CloseEmptyElement,
    EndElement,
    Value,
    Attribute,
    CdataSection,
    CharRef,
    EntityRef,
    ProcessInstructionsTarget,
    ProcessInstructionsData,
    TemplateInstance,
    NormalSubstitution,
    OptionalSubstitution,
    FragmentHeaderToken,
    Unknown,
}

/// Determine token type for element
fn get_token_type(token: &u8) -> TokenType {
    match token {
        0x0 => TokenType::Eof,
        0x1 | 0x41 => TokenType::OpenStartElement,
        0x2 => TokenType::CloseStartElement,
        0x3 => TokenType::CloseEmptyElement,
        0x4 => TokenType::EndElement,
        0x5 | 0x45 => TokenType::Value,
        0x6 | 0x46 => TokenType::Attribute,
        0x7 | 0x47 => TokenType::CdataSection,
        0x8 | 0x48 => TokenType::CharRef,
        0x9 | 0x49 => TokenType::EntityRef,
        0xa => TokenType::ProcessInstructionsTarget,
        0xb => TokenType::ProcessInstructionsData,
        0xc => TokenType::TemplateInstance,
        0xd => TokenType::NormalSubstitution,
        0xe => TokenType::OptionalSubstitution,
        0xf => TokenType::FragmentHeaderToken,
        _ => TokenType::Unknown,
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum InputType {
    Null,
    Unicode,
    Ansi,
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Int64,
    Uint64,
    Float,
    Double,
    Bool,
    Binary,
    Guid,
    Pointer,
    FileTime,
    SystemTime,
    Sid,
    HexInt32,
    HexInt64,
    BinXml,
    Unknown,
}

/// Determine input type for the manifest
fn get_input_type(data: &u8) -> InputType {
    match data {
        0x0 => InputType::Null,
        0x1 => InputType::Unicode,
        0x2 => InputType::Ansi,
        0x3 => InputType::Int8,
        0x4 => InputType::Uint8,
        0x5 => InputType::Int16,
        0x6 => InputType::Uint16,
        0x7 => InputType::Int32,
        0x8 => InputType::Uint32,
        0x9 => InputType::Int64,
        0xa => InputType::Uint64,
        0xb => InputType::Float,
        0xc => InputType::Double,
        0xd => InputType::Bool,
        0xe => InputType::Binary,
        0xf => InputType::Guid,
        0x10 => InputType::Pointer,
        0x11 => InputType::FileTime,
        0x12 => InputType::SystemTime,
        0x13 => InputType::Sid,
        0x14 => InputType::HexInt32,
        0x15 => InputType::HexInt64,
        0x21 => InputType::BinXml,
        _ => InputType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        attribute_list, element_start, fragment_header, get_name, get_token_type, parse_xml,
    };
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::xml::{
            get_input_type, InputType, TokenType,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_xml() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/template_xml.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_xml(&data, String::from("test")).unwrap();
        assert_eq!(results.elements.len(), 17);
        assert_eq!(results.elements[4].element_name, "Data");
        assert_eq!(results.elements[4].attribute_list.len(), 1);
        assert_eq!(results.elements[4].attribute_list[0].value, "SerialNumber");
        assert_eq!(results.elements[4].attribute_list[0].name, "Name");
        assert_eq!(
            results.elements[4].attribute_list[0].input_type,
            InputType::Ansi
        );
    }

    #[test]
    fn test_fragment_header() {
        let test = [15, 1, 1, 0];
        let (_, result) = fragment_header(&test).unwrap();
        assert_eq!(result, TokenType::FragmentHeaderToken)
    }

    #[test]
    fn test_element_start() {
        let test = [
            1, 255, 255, 25, 0, 0, 0, 68, 130, 9, 0, 69, 0, 118, 0, 101, 0, 110, 0, 116, 0, 68, 0,
            97, 0, 116, 0, 97, 0, 0, 0, 2,
        ];

        let (_, (result, _)) = element_start(&test, &false).unwrap();
        assert_eq!(result.depedency_id, -1);
        assert_eq!(result.element_name, "EventData");
        assert_eq!(result.token, TokenType::OpenStartElement);
        assert_eq!(result.token_number, 1);
        assert!(result.attribute_list.is_empty());
        assert_eq!(result.size, 25);
    }

    #[test]
    fn test_get_name() {
        let test = [
            68, 130, 9, 0, 69, 0, 118, 0, 101, 0, 110, 0, 116, 0, 68, 0, 97, 0, 116, 0, 97, 0, 0, 0,
        ];

        let (_, name) = get_name(&test).unwrap();
        assert_eq!(name, "EventData");
    }

    #[test]
    fn test_get_token_type() {
        let test = [
            0x0, 0x1, 0x2, 0x3, 0x4, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf,
        ];

        for entry in test {
            let result = get_token_type(&entry);
            assert!(result != TokenType::Unknown);
        }
    }

    #[test]
    fn test_attribute_list() {
        let test = [
            39, 0, 0, 0, 6, 75, 149, 4, 0, 78, 0, 97, 0, 109, 0, 101, 0, 0, 0, 5, 1, 10, 0, 68, 0,
            101, 0, 118, 0, 105, 0, 99, 0, 101, 0, 71, 0, 85, 0, 73, 0, 68, 0, 2, 13, 0, 0, 15, 4,
        ];
        let (_, result) = attribute_list(&test).unwrap();
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_input_type() {
        let test = [
            0x0, 0x1, 0x2, 0x3, 0x4, 0x6, 0x7, 0x8, 0x9, 0xa, 0xb, 0xc, 0xd, 0xe, 0xf, 0x10, 0x11,
            0x12, 0x13, 0x14, 0x15, 0x21,
        ];

        for entry in test {
            let result = get_input_type(&entry);
            assert!(result != InputType::Unknown);
        }
    }
}
