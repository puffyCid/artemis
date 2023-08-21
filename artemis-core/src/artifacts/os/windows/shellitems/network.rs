use super::items::ShellItem;
use crate::artifacts::os::windows::shellitems::items::ShellType::Network;
use crate::utils::strings::extract_utf8_string;
use nom::bytes::complete::{take, take_while};
use std::mem::size_of;

/// Parse `Network` `ShellItems`
pub(crate) fn parse_network(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, _flags) = take(size_of::<u8>())(input)?;
    let end_of_string = 0;
    let (input, path_data) = take_while(|b| b != end_of_string)(input)?;
    let path = extract_utf8_string(path_data);

    // If the flag is either 0x80 or 0x40, there may be a comment or description of the network path
    // Currently not parsing this

    let network = ShellItem {
        value: path,
        shell_type: Network,
        created: 0,
        modified: 0,
        accessed: 0,
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, network))
}

#[cfg(test)]
mod tests {
    use super::parse_network;
    use crate::artifacts::os::windows::shellitems::items::ShellType;

    #[test]
    fn test_parse_network() {
        let test_data = [
            1, 129, 92, 92, 118, 109, 119, 97, 114, 101, 45, 104, 111, 115, 116, 92, 83, 104, 97,
            114, 101, 100, 32, 70, 111, 108, 100, 101, 114, 115, 0, 86, 77, 119, 97, 114, 101, 32,
            83, 104, 97, 114, 101, 100, 32, 70, 111, 108, 100, 101, 114, 115, 0, 63, 0, 0, 0,
        ];

        let (_, result) = parse_network(&test_data).unwrap();
        assert_eq!(result.value, "\\\\vmware-host\\Shared Folders");
        assert_eq!(result.shell_type, ShellType::Network);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }
}
