use crate::utils::nom_helper::nom_unsigned_eight_bytes;
use crate::utils::{
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian},
    uuid::format_guid_le_bytes,
};
use common::windows::ShellItem;
use common::windows::ShellType::GameFolder;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `Game` `ShellItem` type. Contains a GUID.
pub(crate) fn parse_game(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, _sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, guid) = take(size_of::<u128>())(input)?;
    let guid_string = format_guid_le_bytes(guid);
    let (input, _empty) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let game_item = ShellItem {
        value: guid_string,
        shell_type: GameFolder,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, game_item))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::game::parse_game;

    #[test]
    fn test_parse_game() {
        let data = [
            0, 71, 70, 83, 73, 229, 134, 82, 32, 242, 245, 6, 67, 189, 177, 134, 66, 69, 227, 50,
            39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, results) = parse_game(&data).unwrap();
        assert_eq!(results.value, "205286e5-f5f2-4306-bdb1-864245e33227");
    }
}
