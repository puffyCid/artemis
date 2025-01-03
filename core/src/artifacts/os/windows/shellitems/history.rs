use crate::utils::strings::extract_utf16_string;
use common::windows::ShellItem;
use common::windows::ShellType::History;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse `History` `ShelliItems` data
pub(crate) fn parse_history(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;

    let value = extract_utf16_string(input);
    let shellitem = ShellItem {
        value,
        shell_type: History,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, shellitem))
}

#[cfg(test)]
mod tests {
    use super::parse_history;
    use common::windows::ShellType;

    #[test]
    fn test_parse_history() {
        let test = [
            99, 77, 0, 83, 0, 72, 0, 105, 0, 115, 0, 116, 0, 48, 0, 49, 0, 50, 0, 48, 0, 50, 0, 51,
            0, 48, 0, 49, 0, 50, 0, 48, 0, 50, 0, 48, 0, 50, 0, 51, 0, 48, 0, 49, 0, 50, 0, 49, 0,
            0, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_history(&test).unwrap();

        assert_eq!(result.value, "MSHist012023012020230121");
        assert_eq!(result.shell_type, ShellType::History);
    }

    #[test]
    fn test_parse_history_directory() {
        let test = [
            99, 84, 0, 104, 0, 105, 0, 115, 0, 32, 0, 80, 0, 67, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_history(&test).unwrap();

        assert_eq!(result.value, "This PC");
        assert_eq!(result.shell_type, ShellType::History);
    }
}
