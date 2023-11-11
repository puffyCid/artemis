use crate::{
    artifacts::os::windows::shellitems::items::detect_shellitem,
    utils::nom_helper::{nom_unsigned_two_bytes, Endian},
};
use common::windows::ShellItem;
use log::error;
use nom::{bytes::complete::take, Needed};

/// Parse the `ShellItems` that are in the `Shortcut` data
pub(crate) fn parse_lnk_shellitems(data: &[u8]) -> nom::IResult<&[u8], Vec<ShellItem>> {
    let (input, total_size) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (remaining_input, mut input) = take(total_size)(input)?;
    let mut shellitems_vec: Vec<ShellItem> = Vec::new();

    let end_of_shellitems = [0, 0];
    // The remaining shellitems are back to back
    while !input.is_empty() && input != end_of_shellitems {
        let (shell_input, item_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        // Size includes size itself
        let adjust_size = 2;
        if item_size < adjust_size {
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }
        let (remaining_input, shellitem_data) = take(item_size - adjust_size)(shell_input)?;
        let item_result = detect_shellitem(shellitem_data);
        let shellitem = match item_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[shortcuts] Could not parse shellitem");
                break;
            }
        };
        shellitems_vec.push(shellitem);

        input = remaining_input;
    }

    Ok((remaining_input, shellitems_vec))
}

#[cfg(test)]
mod tests {
    use super::parse_lnk_shellitems;
    use common::windows::ShellType;

    #[test]
    fn test_parse_lnk_shellitem() {
        let test = [
            202, 2, 58, 0, 31, 68, 71, 26, 3, 89, 114, 63, 167, 68, 137, 197, 85, 149, 254, 107,
            48, 238, 38, 0, 1, 0, 38, 0, 239, 190, 16, 0, 0, 0, 125, 27, 164, 100, 217, 236, 216,
            1, 138, 50, 33, 201, 182, 36, 217, 1, 127, 224, 20, 210, 183, 36, 217, 1, 20, 0, 134,
            0, 116, 0, 30, 0, 67, 70, 83, 70, 24, 0, 49, 0, 0, 0, 0, 0, 62, 82, 204, 166, 16, 0,
            80, 114, 111, 106, 101, 99, 116, 115, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72,
            141, 103, 23, 51, 188, 238, 40, 186, 197, 205, 250, 223, 159, 103, 86, 65, 137, 71,
            197, 199, 107, 192, 182, 127, 66, 0, 9, 0, 4, 0, 239, 190, 85, 79, 123, 22, 62, 82,
            204, 166, 46, 0, 0, 0, 13, 117, 3, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 87, 118, 218, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0, 0,
            0, 68, 0, 78, 0, 49, 0, 0, 0, 0, 0, 99, 85, 46, 17, 16, 0, 82, 117, 115, 116, 0, 0, 58,
            0, 9, 0, 4, 0, 239, 190, 88, 85, 66, 13, 43, 86, 212, 35, 46, 0, 0, 0, 79, 76, 17, 0,
            0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 26, 88, 14, 0, 82, 0, 117, 0,
            115, 0, 116, 0, 0, 0, 20, 0, 98, 0, 49, 0, 0, 0, 0, 0, 42, 86, 214, 40, 16, 0, 65, 82,
            84, 69, 77, 73, 126, 49, 0, 0, 74, 0, 9, 0, 4, 0, 239, 190, 99, 85, 46, 17, 43, 86, 46,
            37, 46, 0, 0, 0, 159, 49, 12, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            155, 94, 138, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0, 105, 0, 115, 0, 45, 0, 99, 0,
            111, 0, 114, 0, 101, 0, 0, 0, 24, 0, 80, 0, 49, 0, 0, 0, 0, 0, 42, 86, 147, 40, 16, 0,
            116, 101, 115, 116, 115, 0, 60, 0, 9, 0, 4, 0, 239, 190, 99, 85, 47, 17, 43, 86, 32,
            37, 46, 0, 0, 0, 157, 51, 12, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            3, 151, 220, 0, 116, 0, 101, 0, 115, 0, 116, 0, 115, 0, 0, 0, 20, 0, 92, 0, 49, 0, 0,
            0, 0, 0, 146, 85, 183, 171, 16, 0, 84, 69, 83, 84, 95, 68, 126, 49, 0, 0, 68, 0, 9, 0,
            4, 0, 239, 190, 99, 85, 47, 17, 43, 86, 27, 37, 46, 0, 0, 0, 159, 51, 12, 0, 0, 0, 20,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 85, 95, 85, 0, 116, 0, 101, 0, 115, 0,
            116, 0, 95, 0, 100, 0, 97, 0, 116, 0, 97, 0, 0, 0, 24, 0, 86, 0, 49, 0, 0, 0, 0, 0, 42,
            86, 157, 40, 16, 0, 119, 105, 110, 100, 111, 119, 115, 0, 64, 0, 9, 0, 4, 0, 239, 190,
            99, 85, 90, 24, 43, 86, 27, 37, 46, 0, 0, 0, 210, 164, 12, 0, 0, 0, 15, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 75, 115, 72, 0, 119, 0, 105, 0, 110, 0, 100, 0, 111, 0,
            119, 0, 115, 0, 0, 0, 22, 0, 86, 0, 49, 0, 0, 0, 0, 0, 43, 86, 195, 41, 16, 0, 97, 109,
            99, 97, 99, 104, 101, 0, 64, 0, 9, 0, 4, 0, 239, 190, 43, 86, 195, 41, 43, 86, 195, 41,
            46, 0, 0, 0, 235, 120, 7, 0, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
            200, 27, 1, 97, 0, 109, 0, 99, 0, 97, 0, 99, 0, 104, 0, 101, 0, 0, 0, 22, 0, 0, 0,
        ];

        let (_, results) = parse_lnk_shellitems(&test).unwrap();

        assert_eq!(results.len(), 8);

        assert_eq!(results[0].value, "59031a47-3f72-44a7-89c5-5595fe6b30ee");
        assert_eq!(results[0].shell_type, ShellType::RootFolder);
        assert_eq!(results[1].value, "Projects");
        assert_eq!(results[2].value, "Rust");
        assert_eq!(results[3].value, "artemis-core");
        assert_eq!(results[4].value, "tests");
        assert_eq!(results[5].value, "test_data");
        assert_eq!(results[6].value, "windows");
        assert_eq!(results[7].value, "amcache");
        assert_eq!(results[7].shell_type, ShellType::Directory);
        assert_eq!(results[7].created, 1673414046);
        assert_eq!(results[7].modified, 1673414046);
        assert_eq!(results[7].accessed, 1673414046);
        assert_eq!(results[7].mft_entry, 489707);
        assert_eq!(results[7].mft_sequence, 26);
    }
}
