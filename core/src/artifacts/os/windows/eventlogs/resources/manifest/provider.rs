use super::crimson::ManifestTemplate;
use crate::utils::nom_helper::{nom_signed_four_bytes, nom_unsigned_four_bytes, Endian};

pub(crate) fn parse_provider<'a>(
    data: &'a [u8],
    template: &mut ManifestTemplate,
) -> nom::IResult<&'a [u8], ()> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    // Size includes sig and size itself
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    // -1 if not set
    let (input, message_id) = nom_signed_four_bytes(input, Endian::Le)?;
    template.message_table_id = message_id;
    let (input, provider_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (mut input, unknown_count) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let adjust_count = 1;

    let mut count = 0;
    while count < provider_count - adjust_count {
        let (remaining, element_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, _unknown) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        input = remaining;
        count += 1;

        // Will need to loop through and jump to each offset
        template.element_offsets.push(element_offset);
    }

    let (input, last_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
    template.element_offsets.push(last_offset);

    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::eventlogs::resources::manifest::{
        crimson::parse_manifest, provider::parse_provider,
    };

    #[test]
    fn test_parse_manifest() {
        let test = [
            67, 82, 73, 77, 208, 56, 0, 0, 5, 0, 1, 0, 1, 0, 0, 0, 108, 39, 153, 151, 4, 251, 232,
            71, 132, 94, 54, 148, 96, 69, 194, 24, 36, 0, 0, 0, 87, 69, 86, 84, 172, 56, 0, 0, 255,
            255, 255, 255, 8, 0, 0, 0, 5, 0, 0, 0, 116, 0, 0, 0, 7, 0, 0, 0, 204, 1, 0, 0, 13, 0,
            0, 0, 4, 48, 0, 0, 2, 0, 0, 0, 80, 48, 0, 0, 0, 0, 0, 0, 120, 49, 0, 0, 1, 0, 0, 0,
            220, 49, 0, 0, 3, 0, 0, 0, 20, 50, 0, 0, 4, 0, 0, 0, 88, 53, 0, 0,
        ];

        let (input, mut template) = parse_manifest(&test).unwrap();
        assert_eq!(template.len(), 1);

        let (_, _) = parse_provider(
            input,
            template
                .get_mut("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap(),
        )
        .unwrap();

        assert_eq!(
            template
                .get("9799276c-fb04-47e8-845e-36946045c218")
                .unwrap()
                .element_offsets
                .len(),
            7
        );
    }
}
