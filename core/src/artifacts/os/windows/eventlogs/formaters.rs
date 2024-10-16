use nom::bytes::complete::is_a;
use serde_json::Value;

/* Unable to find an example eventlog record that requires formatting.
 * Disable this code until an example can be found.
 */

/// Try to format strings for log messages
pub(crate) fn parse_formater<'a>(formater: &'a str, data: &Value) -> nom::IResult<&'a str, String> {
    let (input, (value_string, value_number)) = get_number(formater)?;

    // Remove exclaimation points. Now we only have formating characters left
    let remaining_string = input.replace('!', "");

    // Get formater flags if any. If we do not have any, do not throw error, we just move on
    let flags_result = get_flags(&remaining_string);
    let (input, flags) = match flags_result {
        Ok(result) => result,
        Err(_err) => (remaining_string.as_str(), Vec::new()),
    };

    // Get formater width if any. If we do not have any, do not throw error, we just move on
    let width_result = get_width(&input);
    let (input, width) = match width_result {
        Ok(result) => result,
        Err(_err) => (
            remaining_string.as_str(),
            FormaterWidth {
                is_asterick: false,
                width: 0,
            },
        ),
    };

    // Get formater precision if any. If we do not have any, do not throw error, we just move on
    let precision_result = get_precision(&input);
    let (input, precision) = match precision_result {
        Ok(result) => result,
        Err(_err) => (
            remaining_string.as_str(),
            FormaterWidth {
                is_asterick: false,
                width: 0,
            },
        ),
    };

    // Get formater size if any. If we do not have any, do not throw error, we just move on
    let size_result = get_size(&input);
    let (input, size) = match size_result {
        Ok(result) => result,
        Err(_err) => (remaining_string.as_str(), FormaterSize::Unknown),
    };

    let formater_type = get_type(input);

    Ok(("", String::new()))
}

fn format_message(
    flags: &[Flags],
    width: &FormaterWidth,
    precision: &FormaterWidth,
    size: &FormaterSize,
    formater_type: &FormaterType,
    number: &u8,
    data: &Value,
) -> Option<String> {
    let sign = String::new();

    let mut param_number = number;
    let adjust_id = 1;

    None
}

// Get the %# number from string. Ex: %1!s! returns: (!s!, (%1, 1))
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
fn get_width(formater: &str) -> nom::IResult<&str, FormaterWidth> {
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

    Ok((input, width_value))
}

/// Get formater precision
fn get_precision(formater: &str) -> nom::IResult<&str, FormaterWidth> {
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

    Ok((input, width_value))
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
fn get_size(formater: &str) -> nom::IResult<&str, FormaterSize> {
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
        _ => {
            panic!("[eventlogs] Unknown size formater: {value_data}");
            FormaterSize::Unknown;
        }
    };

    Ok((input, size))
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
fn get_flags(formater: &str) -> nom::IResult<&str, Vec<Flags>> {
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

    Ok((input, flags))
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
    use super::{get_flags, get_number};
    use crate::artifacts::os::windows::eventlogs::formaters::{
        get_precision, get_size, get_type, get_width, Flags, FormaterSize, FormaterType,
    };

    #[test]
    fn parse_formater() {
        let test = "%1!s!";
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
        assert_eq!(flags, vec![Flags::AlignLeft, Flags::AddSign, Flags::Zeros]);
        assert_eq!(width, "5");
    }

    #[test]
    fn test_get_width() {
        let test = "11.s";
        let (input, width) = get_width(test).unwrap();
        assert_eq!(width.is_asterick, false);
        assert_eq!(width.width, 11);
        assert_eq!(input, ".s");
    }

    #[test]
    fn test_get_precision() {
        let test = ".*s";
        let (input, precision) = get_precision(test).unwrap();
        assert_eq!(precision.is_asterick, true);
        assert_eq!(precision.width, 0);
        assert_eq!(input, "s");
    }

    #[test]
    fn test_get_size() {
        let test = "hhx";
        let (input, size) = get_size(test).unwrap();
        assert_eq!(size, FormaterSize::Char);
        assert_eq!(input, "x");
    }

    #[test]
    fn test_get_type() {
        let test = "x";
        let format_type = get_type(test);
        assert_eq!(format_type, FormaterType::Hex);
    }
}
