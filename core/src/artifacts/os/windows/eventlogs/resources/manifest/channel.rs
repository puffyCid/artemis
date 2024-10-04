use super::crimson::ManifestTemplate;
use crate::utils::{
    nom_helper::{nom_signed_four_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct Keywords {
    message_id: i32,
    /**Bitmask? */
    id: u32,
    value: String,
    /**Offset from start of the data */
    offset: u32,
}

pub(crate) fn parse_channel<'a>(
    resource: &'a [u8],
    data: &'a [u8],
    template: &mut ManifestTemplate,
) -> nom::IResult<&'a [u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (mut input, keyword_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let mut count = 0;
    while count < keyword_count {
        let (remaining, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        let (remaining, message_id) = nom_signed_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        let (string_start, _) = take(offset)(resource)?;
        let (string_data, size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

        let adjust_size = 4;
        if adjust_size > size {
            // Should not happen
            continue;
        }
        // Size includes size itself
        let (_, value_data) = take(size - adjust_size)(string_data)?;

        let value = extract_utf16_string(value_data);

        let keyword = Keywords {
            message_id,
            id,
            value,
            offset,
        };

        template.keywords.push(keyword);
    }

    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use super::parse_channel;
    use crate::{
        artifacts::os::windows::eventlogs::resources::manifest::{
            crimson::parse_manifest, provider::parse_provider,
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_template() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/pe/resources/wevt_template.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (input, mut template) = parse_manifest(&data).unwrap();
        let (input, _) = parse_provider(
            input,
            template
                .get_mut("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap(),
        )
        .unwrap();

        let (_input, _) = parse_channel(
            &data,
            input,
            template
                .get_mut("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            template
                .get_mut("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap()
                .keywords
                .len(),
            3
        );

        assert_eq!(
            template
                .get_mut("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap()
                .keywords[0]
                .value,
            "Microsoft-Windows-Storage-NvmeDisk/Diagnose"
        );
    }
}
