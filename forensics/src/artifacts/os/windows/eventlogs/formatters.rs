use super::resources::manifest::xml::Element;
use nom::bytes::complete::is_a;
use serde_json::{Map, Value};

pub(crate) fn formatter_message_table<'a>(
    formatter: &'a str,
    values: &[Value],
) -> nom::IResult<&'a str, String> {
    let (input, (_value_string, value_number)) = get_number(formatter)?;
    // Index number starts at 0
    let adjust_id = 1;
    let value;
    if let Some(result) = values.get((value_number - adjust_id) as usize) {
        value = result;
    } else {
        return Ok(("", String::from("Failed to get element index")));
    }

    // Remove exclamation points. Now we only have formatting characters left
    let remaining_string = input.replace('!', "");
    let text_result = parse_formats(&remaining_string, value, value_number);
    let text = match text_result {
        Ok((_, result)) => result,
        Err(_err) => String::from("Failed to get element index"),
    };

    Ok(("", text))
}

/// Try to format strings for log messages. This is uncommon?
pub(crate) fn formatter_message<'a>(
    formatter: &'a str,
    values: &Map<String, Value>,
    elements: &[Element],
) -> nom::IResult<&'a str, String> {
    let (input, (_value_string, value_number)) = get_number(formatter)?;
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

    // Remove exclamation points. Now we only have formatting characters left
    let remaining_string = input.replace('!', "");
    let text_result = parse_formats(&remaining_string, data, value_number);
    let text = match text_result {
        Ok((_, result)) => result,
        Err(_err) => String::from("Failed to get element index"),
    };

    Ok(("", text))
}

fn parse_formats<'a>(
    input: &'a str,
    data: &Value,
    value_number: u8,
) -> nom::IResult<&'a str, String> {
    // Get formatter flags if any. If we do not have any, do not throw error, we just move on
    let flags_result = get_flags(input);
    let (input, flags) = match flags_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    // Get formatter width if any. If we do not have any, do not throw error, we just move on
    let width_result = get_width(input);
    let (input, width) = match width_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    // Get formatter precision if any. If we do not have any, do not throw error, we just move on
    let precision_result = get_precision(input);
    let (input, precision) = match precision_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    // Get formatter size if any. If we do not have any, do not throw error, we just move on
    let size_result = get_size(input);
    let (input, size) = match size_result {
        Ok(result) => result,
        Err(_err) => (input, None),
    };

    let formatter_type = get_type(input);

    let options = FormatOptions {
        flags,
        width,
        precision,
        _size: size,
    };

    Ok((
        "",
        format_message(&options, &formatter_type, value_number, data),
    ))
}

struct FormatOptions {
    flags: Option<Vec<Flags>>,
    width: Option<FormatterWidth>,
    precision: Option<FormatterWidth>,
    _size: Option<FormatterSize>,
}

fn format_message(
    options: &FormatOptions,
    _formatter_type: &FormatterType,
    _number: u8,
    data: &Value,
) -> String {
    let mut plus_option = String::new();
    let mut width_value = 0;
    let mut precision_value = 0;
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
    }

    if let Some(precision_opt) = &options.precision {
        precision_value = precision_opt.width;
    }

    if options
        .flags
        .as_ref()
        .is_some_and(|f| f.contains(&Flags::AlignLeft) && f.contains(&Flags::Spaces))
    {
        message = format!(
            "{plus_symbol}{:<width$.precision$}",
            serde_json::from_value(data.clone()).unwrap_or(data.to_string()),
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
            serde_json::from_value(data.clone()).unwrap_or(data.to_string()),
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
fn get_number(formatter: &str) -> nom::IResult<&str, (&str, u8)> {
    let value_chars = "%1234567890";
    let (input, value_data) = is_a(value_chars)(formatter)?;

    let number_str = value_data.get(1..).unwrap_or("1");
    let number = number_str.parse().unwrap_or(1);

    Ok((input, (value_data, number)))
}

struct FormatterWidth {
    width: u32,
}

/// Get formatter width
fn get_width(formatter: &str) -> nom::IResult<&str, Option<FormatterWidth>> {
    let width_chars = "*1234567890";
    let (input, value_data) = is_a(width_chars)(formatter)?;

    let width = if value_data.starts_with("*") {
        let number_str = value_data.get(1..).unwrap_or("0");
        number_str.parse().unwrap_or(0)
    } else {
        value_data.parse().unwrap_or(0)
    };

    let width_value = FormatterWidth { width };

    Ok((input, Some(width_value)))
}

/// Get formatter precision
fn get_precision(formatter: &str) -> nom::IResult<&str, Option<FormatterWidth>> {
    let precision_chars = ".*1234567890";
    let (input, value_data) = is_a(precision_chars)(formatter)?;

    let width = if value_data.starts_with(".*") {
        let number_str = value_data.get(2..).unwrap_or("");
        number_str.parse().unwrap_or(0)
    } else {
        let number_str = value_data.get(1..).unwrap_or("0");
        number_str.parse().unwrap_or(0)
    };

    let width_value = FormatterWidth { width };

    Ok((input, Some(width_value)))
}

#[derive(Debug, PartialEq)]
enum FormatterSize {
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

/// Determine formatter size
fn get_size(formatter: &str) -> nom::IResult<&str, Option<FormatterSize>> {
    let size_chars = "hI3264jlLtzw";
    let (input, value_data) = is_a(size_chars)(formatter)?;

    let size = match value_data {
        "hh" => FormatterSize::Char,
        "h" => FormatterSize::ShortInt,
        "I32" => FormatterSize::Int,
        "I64" | "J" => FormatterSize::Int64,
        "l" | "L" => FormatterSize::Long,
        "ll" => FormatterSize::LongLong,
        "t" | "I" => FormatterSize::Ptr,
        "z" => FormatterSize::Size,
        "w" => FormatterSize::Wide,
        _ => FormatterSize::Unknown,
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
fn get_flags(formatter: &str) -> nom::IResult<&str, Option<Vec<Flags>>> {
    let flags_char = "-+0 #";
    let (input, flags_data) = is_a(flags_char)(formatter)?;

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
enum FormatterType {
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

/// Determine the formatter type
fn get_type(formatter: &str) -> FormatterType {
    match formatter {
        "c" | "C" => FormatterType::Char,
        "d" | "i" => FormatterType::SignedDecimal,
        "o" => FormatterType::Octal,
        "u" => FormatterType::UnsignedDecimal,
        "x" => FormatterType::Hex,
        "X" => FormatterType::HexUpper,
        "e" | "E" | "f" | "F" | "g" | "G" => FormatterType::Float,
        "a" | "A" => FormatterType::FloatHex,
        "n" => FormatterType::PointerInt,
        "P" => FormatterType::PointerType,
        "s" | "S" => FormatterType::String,
        "Z" => FormatterType::Unicode,
        _ => FormatterType::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{formatter_message, get_flags, get_number};
    use crate::artifacts::os::windows::eventlogs::{
        formatters::{
            Flags, FormatterSize, FormatterType, get_precision, get_size, get_type, get_width,
        },
        resources::manifest::xml::{Element, InputType, TokenType},
    };
    use serde_json::{Map, Value};

    #[test]
    fn test_formatter_message() {
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
        let (_, result) = formatter_message(test, &value, &[element]).unwrap();
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
        assert_eq!(width.unwrap().width, 11);
        assert_eq!(input, ".s");
    }

    #[test]
    fn test_get_precision() {
        let test = ".*s";
        let (input, precision) = get_precision(test).unwrap();
        assert_eq!(precision.unwrap().width, 0);
        assert_eq!(input, "s");
    }

    #[test]
    fn test_get_size() {
        let test = "hhx";
        let (input, size) = get_size(test).unwrap();
        assert_eq!(size.unwrap(), FormatterSize::Char);
        assert_eq!(input, "x");
    }

    #[test]
    fn test_get_type() {
        let test = "x";
        let format_type = get_type(test);
        assert_eq!(format_type, FormatterType::Hex);
    }
}
