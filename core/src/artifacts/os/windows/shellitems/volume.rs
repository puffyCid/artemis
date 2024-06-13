use crate::utils::strings::extract_utf8_string;
use common::windows::ShellItem;
use common::windows::ShellType::Volume;
use nom::bytes::complete::take;

/// Grab the Volume Drive
pub(crate) fn parse_drive(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    // Drive shellitem just contains a drive letter
    let drive = extract_utf8_string(data);
    let shellitem = ShellItem {
        value: drive,
        shell_type: Volume,
        created: String::new(),
        modified: String::new(),
        accessed: String::new(),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    let (input, _) = take(data.len())(data)?;
    Ok((input, shellitem))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::volume::parse_drive;
    use common::windows::ShellType;

    #[test]
    fn test_parse_root() {
        let test_data = [
            67, 58, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = parse_drive(&test_data).unwrap();
        assert_eq!(result.value, "C:\\");
        assert_eq!(result.shell_type, ShellType::Volume);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, "");
        assert_eq!(result.modified, "");
        assert_eq!(result.accessed, "");
    }
}
