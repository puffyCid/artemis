use super::resources::manifest::xml::Element;
use nom::bytes::complete::is_a;
use serde_json::{Map, Value};

pub(crate) fn formater_message_table<'a>(
    formater: &'a str,
    values: &[Value],
) -> nom::IResult<&'a str, String> {
    let (input, (_value_string, value_number)) = get_number(formater)?;
    // Index number starts at 0
    let adjust_id = 1;
    let value;
    if let Some(result) = values.get((value_number - adjust_id) as usize) {
        value = result;
    } else {
        return Ok(("", String::from("Failed to get element index")));
    }

    // Remove exclaimation points. Now we only have formating characters left
    let remaining_string = input.replace('!', "");
    let text_result = parse_formats(&remaining_string, value, &value_number);
    let text = match text_result {
        Ok((_, result)) => result,
        Err(_err) => String::from("Failed to get element index"),
    };

    Ok(("", text))
}

/// Try to format strings for log messages. This is uncommon?
pub(crate) fn formater_message<'a>(
    formater: &'a str,
    values: &Map<String, Value>,
    elements: &[Element],
) -> nom::IResult<&'a str, String> {
    let (input, (_value_string, value_number)) = get_number(formater)?;
    // Index number starts at 0
    let adjust_id = 1;
    let element;
    if let Some(result) = elements.get((value_number - adjust_id) as usize) {
        element = result;
    } else {
        return Ok(("", String::from("Failed to get element index")));
    }

    let mut data = &Value::Null;
    if element.attribute_list.is_empty() {
        data = values.get(&element.element_name).unwrap_or(&Value::Null);
    } else {
        for attr in &element.attribute_list {
            if let Some(result) = values.get(&attr.value) {
                data = result;
                break;
            }
        }
    }

    // Remove exclaimation points. Now we only have formating characters left
    let remaining_string = input.replace('!', "");
    let text_result = parse_formats(&remaining_string, data, &value_number);
    let text = match text_result {
        Ok((_, result)) => result,
        Err(_err) => String::from("Failed to get element index"),
    };

    Ok(("", text))
}

fn parse_formats<'a>(
    input: &'a str,
    data: &Value,
    value_number: &u8,
) -> nom::IResult<&'a str, String> {
    // Get formater flags if any. If we do not have any, do not throw error, we just move on
    let flags_result = get_flags(input);
    let (input, flags) = match flags_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    // Get formater width if any. If we do not have any, do not throw error, we just move on
    let width_result = get_width(input);
    let (input, width) = match width_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    // Get formater precision if any. If we do not have any, do not throw error, we just move on
    let precision_result = get_precision(input);
    let (input, precision) = match precision_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    // Get formater size if any. If we do not have any, do not throw error, we just move on
    let size_result = get_size(input);
    let (input, size) = match size_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    let formater_type = get_type(input);

    let options = FormatOptions {
        flags,
        width,
        precision,
        _size: size,
    };

    Ok((
        "",
        format_message(&options, &formater_type, value_number, data),
    ))
}

struct FormatOptions {
    flags: Option<Vec<Flags>>,
    width: Option<FormaterWidth>,
    precision: Option<FormaterWidth>,
    _size: Option<FormaterSize>,
}

fn format_message(
    options: &FormatOptions,
    _formater_type: &FormaterType,
    _number: &u8,
    data: &Value,
) -> String {
    let mut plus_option = String::new();
    let mut width_value = 0;
    let mut precision_value = 0;
    let mut _width_asterick = false;
    let mut _precision_asterick = false;
    let message;

    if options
        .flags
        .as_ref()
        .is_some_and(|f| f.contains(&Flags::AddSign))
    {
        plus_option = String::from("+");
    }

    if let Some(width_opt) = &options.width {
        width_value = width_opt.width;
        _width_asterick = width_opt.is_asterick;
    }

    if let Some(precision_opt) = &options.precision {
        precision_value = precision_opt.width;
        _precision_asterick = precision_opt.is_asterick;
    }

    if options
        .flags
        .as_ref()
        .is_some_and(|f| f.contains(&Flags::AlignLeft) && f.contains(&Flags::Spaces))
    {
        message = format!(
            "{plus_symbol}{:<width$.precision$}",
            &serde_json::from_value(data.clone()).unwrap_or(data.to_string()),
            width = width_value as usize,
            precision = precision_value as usize,
            plus_symbol = plus_option
        );
    } else if options
        .flags
        .as_ref()
        .is_some_and(|f| f.contains(&Flags::AlignLeft) && f.contains(&Flags::Zeros))
    {
        message = format!(
            "{plus_symbol}{:0<width$.precision$}",
            &serde_json::from_value(data.clone()).unwrap_or(data.to_string()),
            width = width_value as usize,
            precision = precision_value as usize,
            plus_symbol = plus_option
        );
    } else {
        message = serde_json::from_value(data.clone()).unwrap_or(data.to_string());
    }

    message
}

/// Get the %# number from string. Ex: %1!s! returns: (!s!, (%1, 1))
fn get_number(formater: &str) -> nom::IResult<&str, (&str, u8)> {
    let value_chars = "%1234567890";
    let (input, value_data) = is_a(value_chars)(formater)?;

    let number_str = value_data.get(1..).unwrap_or("1");
    let number = number_str.parse().unwrap_or(1);

    Ok((input, (value_data, number)))
}

struct FormaterWidth {
    is_asterick: bool,
    width: u32,
}

/// Get formater width
fn get_width(formater: &str) -> nom::IResult<&str, Option<FormaterWidth>> {
    let width_chars = "*1234567890";
    let (input, value_data) = is_a(width_chars)(formater)?;

    let mut is_asterick = false;
    let width;
    if value_data.starts_with("*") {
        is_asterick = true;
        let number_str = value_data.get(1..).unwrap_or("0");
        width = number_str.parse().unwrap_or(0);
    } else {
        width = value_data.parse().unwrap_or(0);
    }

    let width_value = FormaterWidth { is_asterick, width };

    Ok((input, Some(width_value)))
}

/// Get formater precision
fn get_precision(formater: &str) -> nom::IResult<&str, Option<FormaterWidth>> {
    let precision_chars = ".*1234567890";
    let (input, value_data) = is_a(precision_chars)(formater)?;

    let mut is_asterick = false;
    let width;

    if value_data.starts_with(".*") {
        is_asterick = true;
        let number_str = value_data.get(2..).unwrap_or("");
        width = number_str.parse().unwrap_or(0);
    } else {
        let number_str = value_data.get(1..).unwrap_or("0");
        width = number_str.parse().unwrap_or(0);
    }

    let width_value = FormaterWidth { is_asterick, width };

    Ok((input, Some(width_value)))
}

#[derive(Debug, PartialEq)]
enum FormaterSize {
    Char,
    ShortInt,
    Int,
    Int64,
    Long,
    LongLong,
    Size,
    Ptr,
    Wide,
    Unknown,
}

/// Determine formater size
fn get_size(formater: &str) -> nom::IResult<&str, Option<FormaterSize>> {
    let size_chars = "hI3264jlLtzw";
    let (input, value_data) = is_a(size_chars)(formater)?;

    let size = match value_data {
        "hh" => FormaterSize::Char,
        "h" => FormaterSize::ShortInt,
        "I32" => FormaterSize::Int,
        "I64" | "J" => FormaterSize::Int64,
        "l" | "L" => FormaterSize::Long,
        "ll" => FormaterSize::LongLong,
        "t" | "I" => FormaterSize::Ptr,
        "z" => FormaterSize::Size,
        "w" => FormaterSize::Wide,
        _ => FormaterSize::Unknown,
    };

    Ok((input, Some(size)))
}

#[derive(Debug, PartialEq)]
enum Flags {
    AlignLeft,
    /**Integer value. Either + or - */
    AddSign,
    Zeros,
    Spaces,
    AddHex,
}

/// Get formatter flags
fn get_flags(formater: &str) -> nom::IResult<&str, Option<Vec<Flags>>> {
    let flags_char = "-+0 #";
    let (input, flags_data) = is_a(flags_char)(formater)?;

    let mut flags = Vec::new();
    for flag in flags_data.chars() {
        match flag {
            '-' => flags.push(Flags::AlignLeft),
            '+' => flags.push(Flags::AddSign),
            ' ' => flags.push(Flags::Spaces),
            '#' => flags.push(Flags::AddHex),
            '0' => flags.push(Flags::Zeros),
            _ => break,
        }
    }

    Ok((input, Some(flags)))
}

#[derive(Debug, PartialEq)]
enum FormaterType {
    Char,
    SignedDecimal,
    UnsignedDecimal,
    Octal,
    Hex,
    HexUpper,
    Float,
    FloatHex,
    PointerInt,
    PointerType,
    String,
    Unicode,
    Unknown,
}

/// Determine the formater type
fn get_type(formater: &str) -> FormaterType {
    match formater {
        "c" | "C" => FormaterType::Char,
        "d" | "i" => FormaterType::SignedDecimal,
        "o" => FormaterType::Octal,
        "u" => FormaterType::UnsignedDecimal,
        "x" => FormaterType::Hex,
        "X" => FormaterType::HexUpper,
        "e" | "E" | "f" | "F" | "g" | "G" => FormaterType::Float,
        "a" | "A" => FormaterType::FloatHex,
        "n" => FormaterType::PointerInt,
        "P" => FormaterType::PointerType,
        "s" | "S" => FormaterType::String,
        "Z" => FormaterType::Unicode,
        _ => FormaterType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{formater_message, get_flags, get_number};
    use crate::artifacts::os::windows::eventlogs::{
        formaters::{
            Flags, FormaterSize, FormaterType, get_precision, get_size, get_type, get_width,
        },
        resources::manifest::xml::{Element, InputType, TokenType},
    };
    use serde_json::{Map, Value};

    #[test]
    fn test_formater_message() {
        let test = "%1!s!";
        let mut value = Map::new();
        value.insert(
            String::from("test"),
            Value::String(String::from("hello rust!")),
        );

        let element = Element {
            token: TokenType::Attribute,
            token_number: 0,
            depedency_id: 0,
            size: 2,
            attribute_list: Vec::new(),
            element_name: String::from("test"),
            input_type: InputType::Unknown,
            substitution: TokenType::Unknown,
            substitution_id: 0,
        };
        let (_, result) = formater_message(test, &value, &[element]).unwrap();
        assert_eq!(result, "hello rust!");
    }

    #[test]
    fn test_get_number() {
        let test = "%1!s!";
        let (input, (value, number)) = get_number(test).unwrap();
        assert_eq!(input, "!s!");
        assert_eq!(value, "%1");
        assert_eq!(number, 1);
    }

    #[test]
    fn test_get_flags() {
        let test = "-+05";
        let (width, flags) = get_flags(test).unwrap();
        assert_eq!(
            flags.unwrap(),
            vec![Flags::AlignLeft, Flags::AddSign, Flags::Zeros]
        );
        assert_eq!(width, "5");
    }

    #[test]
    fn test_get_width() {
        let test = "11.s";
        let (input, width) = get_width(test).unwrap();
        assert_eq!(width.as_ref().unwrap().is_asterick, false);
        assert_eq!(width.unwrap().width, 11);
        assert_eq!(input, ".s");
    }

    #[test]
    fn test_get_precision() {
        let test = ".*s";
        let (input, precision) = get_precision(test).unwrap();
        assert_eq!(precision.as_ref().unwrap().is_asterick, true);
        assert_eq!(precision.unwrap().width, 0);
        assert_eq!(input, "s");
    }

    #[test]
    fn test_get_size() {
        let test = "hhx";
        let (input, size) = get_size(test).unwrap();
        assert_eq!(size.unwrap(), FormaterSize::Char);
        assert_eq!(input, "x");
    }

    #[test]
    fn test_get_type() {
        let test = "x";
        let format_type = get_type(test);
        assert_eq!(format_type, FormaterType::Hex);
    }
}
