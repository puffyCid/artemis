use crate::utils::nom_helper::Endian;
use crate::utils::nom_helper::nom_unsigned_four_bytes;
use crate::utils::uuid::format_guid_le_bytes;
use common::windows::ShellItem;
use common::windows::ShellType;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `Control Panel` `ShellItem` type
pub(crate) fn parse_control_panel(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, _signature) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, panel) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let value = match panel {
        0 => "All Control Panel Items",
        1 => "Appearance and Personalization",
        2 => "Hardware and Sound",
        3 => "Network and Internet",
        4 => "Sounds, Speech, and Audio Devices",
        5 => "System and Security",
        6 => "Clock, Language, and Region",
        7 => "Ease of Access",
        8 => "Programs",
        9 => "User Accounts",
        10 => "Security Center",
        11 => "Mobile PC",
        _ => "Unknown Control Panel",
    };

    let panel_item = ShellItem {
        value: value.to_string(),
        shell_type: ShellType::ControlPanel,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, panel_item))
}

/// Parse a `Control Panel Entry` `ShellItem` type
pub(crate) fn parse_control_panel_entry(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    // Always 0x80
    let (input, _unknown) = take(size_of::<u8>())(data)?;

    // Always [0;10];
    let unknown_size: u8 = 10;
    let guid_size: u8 = 16;

    // Some Control Panel entries do not have 10 bytes of empty space
    // Seen with Windows Fonts under Control Panel -> Appearence -> Fonts
    if input.len() < (unknown_size + guid_size) as usize {
        let (input, guid_data) = take(guid_size)(input)?;
        let panel_item = ShellItem {
            value: format_guid_le_bytes(guid_data),
            shell_type: ShellType::ControlPanelEntry,
            created: String::from("1970-01-01T00:00:00.000Z"),
            modified: String::from("1970-01-01T00:00:00.000Z"),
            accessed: String::from("1970-01-01T00:00:00.000Z"),
            mft_entry: 0,
            mft_sequence: 0,
            stores: Vec::new(),
        };

        return Ok((input, panel_item));
    }

    let (input, _unknown2) = take(unknown_size)(input)?;

    let (input, guid_data) = take(guid_size)(input)?;
    let panel_item = ShellItem {
        value: format_guid_le_bytes(guid_data),
        shell_type: ShellType::ControlPanelEntry,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, panel_item))
}

#[cfg(test)]
mod tests {
    use super::parse_control_panel;
    use crate::artifacts::os::windows::shellitems::controlpanel::parse_control_panel_entry;
    use common::windows::ShellType;

    #[test]
    fn test_parse_control_panel() {
        let test_data = [0, 132, 33, 222, 57, 0, 0, 0, 0, 0, 0];

        let (_, result) = parse_control_panel(&test_data).unwrap();
        assert_eq!(result.value, "All Control Panel Items");
        assert_eq!(result.shell_type, ShellType::ControlPanel);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
    }

    #[test]
    fn test_parse_control_panel_entry() {
        let test_data = [
            128, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 229, 245, 115, 156, 231, 122, 50, 78, 168, 232, 141,
            35, 184, 82, 85, 191, 0, 0,
        ];

        let (_, result) = parse_control_panel_entry(&test_data).unwrap();
        assert_eq!(result.value, "9c73f5e5-7ae7-4e32-a8e8-8d23b85255bf");
        assert_eq!(result.shell_type, ShellType::ControlPanelEntry);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
    }

    #[test]
    fn test_parse_control_panel_entry_font() {
        let test_data = [
            128, 137, 37, 65, 147, 212, 116, 78, 78, 173, 14, 224, 203, 98, 20, 64, 253, 0, 0,
        ];

        let (_, result) = parse_control_panel_entry(&test_data).unwrap();
        // 93412589-74d4-4e4e-ad0e-e0cb621440fd = Windows Fonts
        assert_eq!(result.value, "93412589-74d4-4e4e-ad0e-e0cb621440fd");
        assert_eq!(result.shell_type, ShellType::ControlPanelEntry);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
    }
}
