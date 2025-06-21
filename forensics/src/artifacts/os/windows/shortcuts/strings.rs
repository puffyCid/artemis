use crate::utils::{
    nom_helper::{Endian, nom_unsigned_two_bytes},
    strings::{extract_utf8_string, extract_utf16_string},
};
use common::windows::DataFlags;
use log::warn;
use nom::bytes::complete::take;

/// Extract strings from `shortcut` data. Most string data will have max length of 260 bytes (520 if Unicode)
/// Only command arguements in the shortcut file are the exception
pub(crate) fn extract_string<'a>(
    data: &'a [u8],
    flags: &[DataFlags],
    is_command_args: &bool,
) -> nom::IResult<&'a [u8], (String, bool)> {
    let (mut input, mut size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    // The Windows implementation of the Shortcut format limits string sizes to 260 bytes (520 if using UTF16)
    // Even though the Shortcut file spec allows string sizes up to 64KB
    let mut max_string_size = 260;
    if input.starts_with(&[0, 0]) {
        // If the size is really big then 2 padding? bytes seem to be added. Seen when greater than 520 bytes
        let (remaining, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;
        input = remaining;
        max_string_size = 259;
    }
    // Size for UTF16 chars (2 bytes)
    let adjust_size = 2;
    let mut is_abnormal = false;

    if size > max_string_size * adjust_size
        && flags.contains(&DataFlags::IsUnicode)
        && !*is_command_args
    {
        // Legit Shortcut files should follow the Windows implementation (strings are limited to 260 bytes)
        // However, Shortcut files that are larger than 260 bytes may have been created manually or using non-Windows standards
        // This is sometimes used by threat actors to hide Shortcut data from forensic tools
        // See: https://harfanglab.io/insidethelab/sadfuture-xdspy-latest-evolution/#tid_specifications_ignored
        warn!(
            "[shortcuts] Got abnormal string size. LNK data could be malformed or possibly malicious"
        );
        size = max_string_size;
        is_abnormal = true;
    } else if size > max_string_size && !flags.contains(&DataFlags::IsUnicode) && !*is_command_args
    {
        warn!(
            "[shortcuts] Got abnormal string size. LNK data could be malformed or possibly malicious"
        );
        size = max_string_size;
        is_abnormal = true;
    }

    if flags.contains(&DataFlags::IsUnicode) {
        let (input, string_data) = take(size * adjust_size)(input)?;
        let data_string = extract_utf16_string(string_data);
        return Ok((input, (data_string, is_abnormal)));
    }

    let (input, string_data) = take(size)(input)?;
    let data_string = extract_utf8_string(string_data);
    Ok((input, (data_string, is_abnormal)))
}

#[cfg(test)]
mod tests {
    use super::extract_string;
    use common::windows::DataFlags;

    #[test]
    fn test_extract_string() {
        let test = [
            41, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92, 0, 46, 0, 46, 0, 92,
            0, 46, 0, 46, 0, 92, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0,
            92, 0, 82, 0, 117, 0, 115, 0, 116, 0, 92, 0, 97, 0, 114, 0, 116, 0, 101, 0, 109, 0,
            105, 0, 115, 0, 45, 0, 99, 0, 111, 0, 114, 0, 101, 0,
        ];
        let (_, (result, is_abnormal)) =
            extract_string(&test, &[DataFlags::IsUnicode], &false).unwrap();
        assert_eq!(result, "..\\..\\..\\..\\..\\Projects\\Rust\\artemis-core");
        assert!(!is_abnormal);
    }

    #[test]
    fn test_malformed_string_utf8() {
        let test = [
            19, 1, 66, 108, 97, 104, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 13, 0, 47, 99, 32, 34, 99, 97, 108, 99, 46, 101, 120, 101, 34, 8, 0, 102, 105,
            108, 101, 46, 112, 100, 102,
        ];
        let (remaining, (result, is_abnormal)) = extract_string(&test, &[], &false).unwrap();
        assert_eq!(result, "Blah");
        assert!(is_abnormal);

        let (_, (result, is_abnormal)) = extract_string(&remaining, &[], &false).unwrap();
        assert_eq!(result, "/c \"calc.exe\"");
        assert!(!is_abnormal);
    }

    #[test]
    fn test_malformed_string_utf16() {
        let test = [
            226, 2, 0, 0, 9, 0, 32, 0, 32, 0, 32, 0, 9, 0, 32, 0, 32, 0, 9, 0, 32, 0, 9, 0, 2, 0,
            2, 0, 32, 0, 32, 0, 2, 0, 9, 0, 32, 0, 32, 0, 32, 0, 2, 0, 2, 0, 32, 0, 32, 0, 9, 0,
            32, 0, 32, 0, 2, 0, 9, 0, 32, 0, 32, 0, 9, 0, 2, 0, 32, 0, 32, 0, 2, 0, 32, 0, 2, 0,
            32, 0, 9, 0, 9, 0, 2, 0, 9, 0, 2, 0, 9, 0, 2, 0, 32, 0, 2, 0, 32, 0, 2, 0, 32, 0, 9, 0,
            2, 0, 9, 0, 9, 0, 9, 0, 32, 0, 9, 0, 32, 0, 32, 0, 32, 0, 2, 0, 9, 0, 9, 0, 9, 0, 9, 0,
            9, 0, 2, 0, 32, 0, 32, 0, 2, 0, 2, 0, 9, 0, 2, 0, 2, 0, 2, 0, 9, 0, 2, 0, 9, 0, 2, 0,
            32, 0, 9, 0, 32, 0, 9, 0, 9, 0, 32, 0, 9, 0, 9, 0, 9, 0, 2, 0, 9, 0, 32, 0, 2, 0, 2, 0,
            32, 0, 9, 0, 32, 0, 2, 0, 9, 0, 2, 0, 32, 0, 2, 0, 9, 0, 9, 0, 9, 0, 2, 0, 32, 0, 9, 0,
            9, 0, 9, 0, 2, 0, 32, 0, 32, 0, 9, 0, 32, 0, 9, 0, 32, 0, 2, 0, 9, 0, 2, 0, 9, 0, 2, 0,
            9, 0, 9, 0, 2, 0, 9, 0, 2, 0, 2, 0, 32, 0, 9, 0, 2, 0, 9, 0, 9, 0, 32, 0, 32, 0, 9, 0,
            32, 0, 9, 0, 2, 0, 32, 0, 32, 0, 9, 0, 9, 0, 2, 0, 32, 0, 32, 0, 32, 0, 32, 0, 32, 0,
            2, 0, 2, 0, 9, 0, 9, 0, 2, 0, 9, 0, 2, 0, 9, 0, 9, 0, 32, 0, 2, 0, 9, 0, 9, 0, 32, 0,
            32, 0, 2, 0, 9, 0, 2, 0, 9, 0, 32, 0, 9, 0, 9, 0, 32, 0, 9, 0, 32, 0, 2, 0, 2, 0, 2, 0,
            9, 0, 2, 0, 9, 0, 2, 0, 9, 0, 9, 0, 32, 0, 32, 0, 9, 0, 9, 0, 9, 0, 2, 0, 32, 0, 32, 0,
            32, 0, 2, 0, 2, 0, 2, 0, 32, 0, 9, 0, 32, 0, 32, 0, 9, 0, 9, 0, 32, 0, 32, 0, 9, 0, 32,
            0, 9, 0, 9, 0, 9, 0, 9, 0, 2, 0, 32, 0, 32, 0, 2, 0, 2, 0, 2, 0, 9, 0, 32, 0, 32, 0,
            32, 0, 2, 0, 2, 0, 9, 0, 9, 0, 2, 0, 2, 0, 32, 0, 2, 0, 9, 0, 32, 0, 9, 0, 2, 0, 32, 0,
            2, 0, 32, 0, 2, 0, 2, 0, 32, 0, 2, 0, 2, 0, 32, 0, 32, 0, 2, 0, 32, 0, 9, 0, 32, 0, 9,
            0, 2, 0, 9, 0, 32, 0, 9, 0, 2, 0, 2, 0, 9, 0, 32, 0, 32, 0, 32, 0, 9, 0, 9, 0, 32, 0,
            2, 0, 189, 1, 9, 0, 9, 0, 2, 0, 32, 0, 2, 0, 2, 0, 9, 0, 2, 0, 9, 0, 2, 0, 2, 0, 32, 0,
            2, 0, 9, 0, 2, 0, 2, 0, 32, 0, 9, 0, 9, 0, 9, 0, 9, 0, 32, 0, 32, 0, 32, 0, 32, 0, 9,
            0, 9, 0, 2, 0, 9, 0, 32, 0, 2, 0, 32, 0, 9, 0, 9, 0, 32, 0, 9, 0, 2, 0, 2, 0, 32, 0, 2,
            0, 32, 0, 32, 0, 32, 0, 9, 0, 9, 0, 9, 0, 32, 0, 32, 0, 9, 0, 2, 0, 2, 0, 9, 0, 2, 0,
            2, 0, 32, 0, 2, 0, 9, 0, 2, 0, 9, 0, 2, 0, 2, 0, 32, 0, 2, 0, 9, 0, 32, 0, 2, 0, 9, 0,
            32, 0, 32, 0, 32, 0, 2, 0, 9, 0, 32, 0, 32, 0, 9, 0, 2, 0, 9, 0, 32, 0, 2, 0, 9, 0, 2,
            0, 9, 0, 2, 0, 9, 0, 2, 0, 2, 0, 2, 0, 9, 0, 32, 0, 9, 0, 9, 0, 9, 0, 32, 0, 2, 0, 9,
            0, 2, 0, 9, 0, 32, 0, 32, 0, 9, 0, 32, 0, 2, 0, 2, 0, 9, 0, 32, 0, 2, 0, 9, 0, 2, 0,
            32, 0, 2, 0, 9, 0, 2, 0, 32, 0, 32, 0, 9, 0, 32, 0, 2, 0, 2, 0, 2, 0, 9, 0, 32, 0, 9,
            0, 9, 0, 9, 0, 9, 0, 2, 0, 9, 0, 32, 0, 9, 0, 2, 0, 32, 0, 9, 0, 9, 0, 32, 0, 32, 0, 9,
            0, 2, 0, 32, 0, 9, 0, 32, 0, 9, 0, 2, 0, 2, 0, 9, 0, 9, 0, 9, 0, 32, 0, 2, 0, 2, 0, 9,
            0, 32, 0, 32, 0, 32, 0, 9, 0, 9, 0, 32, 0, 2, 0, 32, 0, 32, 0, 32, 0, 9, 0, 32, 0, 9,
            0, 2, 0, 32, 0, 32, 0, 32, 0, 2, 0, 2, 0, 9, 0, 9, 0, 2, 0, 9, 0, 2, 0, 2, 0, 32, 0, 2,
            0, 32, 0, 9, 0, 9, 0, 2, 0, 32, 0, 9, 0, 32, 0, 32, 0, 9, 0, 9, 0, 2, 0, 2, 0, 2, 0, 9,
            0, 9, 0, 32, 0, 32, 0, 32, 0, 9, 0, 2, 0, 2, 0, 2, 0, 9, 0, 9, 0, 32, 0, 32, 0, 9, 0,
            32, 0, 9, 0, 32, 0, 9, 0, 2, 0, 2, 0, 2, 0, 9, 0, 9, 0, 32, 0, 9, 0, 32, 0, 9, 0, 32,
            0, 9, 0, 9, 0, 32, 0, 32, 0, 9, 0, 32, 0, 2, 0, 9, 0, 9, 0, 32, 0, 9, 0, 32, 0, 2, 0,
            32, 0, 9, 0, 2, 0, 32, 0, 32, 0, 9, 0, 32, 0, 2, 0, 9, 0, 9, 0, 32, 0, 32, 0, 2, 0, 2,
            0, 9, 0, 2, 0, 2, 0, 32, 0, 2, 0, 2, 0, 32, 0, 9, 0, 9, 0, 2, 0, 9, 0, 9, 0, 32, 0, 9,
            0, 2, 0, 47, 0, 99, 0, 32, 0, 34, 0, 110, 0, 111, 0, 116, 0, 101, 0, 112, 0, 97, 0,
            100, 0, 46, 0, 101, 0, 120, 0, 101, 0, 32, 0, 116, 0, 104, 0, 105, 0, 115, 0, 102, 0,
            105, 0, 108, 0, 101, 0, 100, 0, 111, 0, 101, 0, 115, 0, 110, 0, 111, 0, 116, 0, 101, 0,
            120, 0, 105, 0, 115, 0, 116, 0, 98, 0, 117, 0, 116, 0, 105, 0, 116, 0, 100, 0, 111, 0,
            101, 0, 115, 0, 110, 0, 116, 0, 109, 0, 97, 0, 116, 0, 116, 0, 101, 0, 114, 0, 110, 0,
            111, 0, 116, 0, 101, 0, 112, 0, 97, 0, 100, 0, 119, 0, 105, 0, 108, 0, 108, 0, 111, 0,
            102, 0, 102, 0, 101, 0, 114, 0, 116, 0, 111, 0, 99, 0, 114, 0, 101, 0, 97, 0, 116, 0,
            101, 0, 105, 0, 116, 0, 97, 0, 116, 0, 115, 0, 116, 0, 97, 0, 114, 0, 116, 0, 97, 0,
            110, 0, 100, 0, 121, 0, 111, 0, 117, 0, 99, 0, 97, 0, 110, 0, 101, 0, 118, 0, 101, 0,
            110, 0, 115, 0, 107, 0, 105, 0, 112, 0, 116, 0, 104, 0, 101, 0, 101, 0, 120, 0, 116, 0,
            101, 0, 110, 0, 115, 0, 105, 0, 111, 0, 110, 0, 97, 0, 115, 0, 110, 0, 111, 0, 116, 0,
            101, 0, 112, 0, 97, 0, 100, 0, 105, 0, 115, 0, 110, 0, 105, 0, 99, 0, 101, 0, 101, 0,
            110, 0, 111, 0, 117, 0, 103, 0, 104, 0, 116, 0, 111, 0, 97, 0, 100, 0, 100, 0, 105, 0,
            116, 0, 102, 0, 111, 0, 114, 0, 121, 0, 111, 0, 117, 0, 98, 0, 117, 0, 116, 0, 105, 0,
            119, 0, 105, 0, 108, 0, 108, 0, 112, 0, 101, 0, 114, 0, 115, 0, 111, 0, 110, 0, 97, 0,
            108, 0, 108, 0, 121, 0, 97, 0, 100, 0, 100, 0, 105, 0, 116, 0, 116, 0, 104, 0, 105, 0,
            115, 0, 116, 0, 105, 0, 109, 0, 101, 0, 46, 0, 116, 0, 120, 0, 116, 0, 34, 0, 31, 0,
            67, 0, 58, 0, 92, 0, 87, 0, 105, 0, 110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 92, 0, 83,
            0, 121, 0, 115, 0, 116, 0, 101, 0, 109, 0, 51, 0, 50, 0, 92, 0, 115, 0, 104, 0, 101, 0,
            108, 0, 108, 0, 51, 0, 50, 0, 46, 0, 100, 0, 108, 0, 108, 0,
        ];
        let (remaining, (result, is_abnormal)) =
            extract_string(&test, &[DataFlags::IsUnicode], &false).unwrap();
        assert_eq!(
            result,
            "\t   \t  \t \t\u{2}\u{2}  \u{2}\t   \u{2}\u{2}  \t  \u{2}\t  \t\u{2}  \u{2} \u{2} \t\t\u{2}\t\u{2}\t\u{2} \u{2} \u{2} \t\u{2}\t\t\t \t   \u{2}\t\t\t\t\t\u{2}  \u{2}\u{2}\t\u{2}\u{2}\u{2}\t\u{2}\t\u{2} \t \t\t \t\t\t\u{2}\t \u{2}\u{2} \t \u{2}\t\u{2} \u{2}\t\t\t\u{2} \t\t\t\u{2}  \t \t \u{2}\t\u{2}\t\u{2}\t\t\u{2}\t\u{2}\u{2} \t\u{2}\t\t  \t \t\u{2}  \t\t\u{2}     \u{2}\u{2}\t\t\u{2}\t\u{2}\t\t \u{2}\t\t  \u{2}\t\u{2}\t \t\t \t \u{2}\u{2}\u{2}\t\u{2}\t\u{2}\t\t  \t\t\t\u{2}   \u{2}\u{2}\u{2} \t  \t\t  \t \t\t\t\t\u{2}  \u{2}\u{2}\u{2}\t   \u{2}\u{2}\t\t\u{2}\u{2} \u{2}\t \t\u{2} \u{2} \u{2}\u{2} \u{2}\u{2}  \u{2} \t \t\u{2}\t \t\u{2}\u{2}\t   \t\t \u{2}"
        );
        assert!(is_abnormal);

        let (remaining, (result, is_abnormal)) =
            extract_string(&remaining, &[DataFlags::IsUnicode], &false).unwrap();
        assert_eq!(
            result,
            "\t\t\u{2} \u{2}\u{2}\t\u{2}\t\u{2}\u{2} \u{2}\t\u{2}\u{2} \t\t\t\t    \t\t\u{2}\t \u{2} \t\t \t\u{2}\u{2} \u{2}   \t\t\t  \t\u{2}\u{2}\t\u{2}\u{2} \u{2}\t\u{2}\t\u{2}\u{2} \u{2}\t \u{2}\t   \u{2}\t  \t\u{2}\t \u{2}\t\u{2}\t\u{2}\t\u{2}\u{2}\u{2}\t \t\t\t \u{2}\t\u{2}\t  \t \u{2}\u{2}\t \u{2}\t\u{2} \u{2}\t\u{2}  \t \u{2}\u{2}\u{2}\t \t\t\t\t\u{2}\t \t\u{2} \t\t  \t\u{2} \t \t\u{2}\u{2}\t\t\t \u{2}\u{2}\t   \t\t \u{2}   \t \t\u{2}   \u{2}\u{2}\t\t\u{2}\t\u{2}\u{2} \u{2} \t\t\u{2} \t  \t\t\u{2}\u{2}\u{2}\t\t   \t\u{2}\u{2}\u{2}\t\t  \t \t \t\u{2}\u{2}\u{2}\t\t \t \t \t\t  \t \u{2}\t\t \t \u{2} \t\u{2}  \t \u{2}\t\t  \u{2}\u{2}\t\u{2}\u{2} \u{2}\u{2} \t\t\u{2}\t\t \t\u{2}/c \"notepad.exe thisfiledoesnotexistbutitdoesntmatternotepadwilloffertocreateitatstartandyoucanevenskiptheextensionasnotepadisniceenoughtoadditforyoubutiwillpersonallyadditthistime.txt\""
        );
        assert!(!is_abnormal);

        let (_, (result, is_abnormal)) =
            extract_string(&remaining, &[DataFlags::IsUnicode], &false).unwrap();
        assert_eq!(result, "C:\\Windows\\System32\\shell32.dll");
        assert!(!is_abnormal);
    }
}
