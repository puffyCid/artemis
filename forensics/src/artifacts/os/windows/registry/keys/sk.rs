use crate::{
    artifacts::os::windows::{
        registry::cell::{CellType, get_cell_type, is_allocated},
        securitydescriptor::descriptor::Descriptor,
    },
    utils::nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub(crate) struct SecurityKey {
    pub(crate) reference_count: u32,
    pub(crate) info: Descriptor,
}

impl SecurityKey {
    /// Parse the Security Key information on a Key. Contains ACLs and SIDs
    pub(crate) fn parse_security_key(
        reg_data: &[u8],
        offset: u32,
    ) -> nom::IResult<&[u8], SecurityKey> {
        let (offset_start, _) = take(offset)(reg_data)?;
        let (input, (is_allocated, size)) = is_allocated(offset_start)?;

        let mut sk_info = SecurityKey {
            reference_count: 0,
            info: Descriptor {
                control_flags: Vec::new(),
                sacls: Vec::new(),
                dacls: Vec::new(),
                owner_sid: String::new(),
                group_sid: String::new(),
            },
        };
        if !is_allocated {
            return Ok((reg_data, sk_info));
        }
        let adjust_size = 4;
        // Size includes the size itself. We already nom'd that
        let (_, input) = take(size - adjust_size)(input)?;
        let (input, cell_type) = get_cell_type(input)?;

        if cell_type != CellType::Sk {
            return Ok((reg_data, sk_info));
        }

        let (input, _sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, _previous_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _next_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, reference_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, nt_descriptor_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, descriptor_data) = take(nt_descriptor_size)(input)?;
        let (_descriptor_data, security_info) = Descriptor::parse_descriptor(descriptor_data)?;

        sk_info.reference_count = reference_count;
        sk_info.info = security_info;

        Ok((input, sk_info))
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::registry::keys::sk::SecurityKey;

    #[test]
    fn test_parse_security_key() {
        let test = [
            8, 255, 255, 255, 115, 107, 0, 0, 168, 10, 93, 0, 208, 250, 205, 0, 1, 0, 0, 0, 224, 0,
            0, 0, 1, 0, 20, 156, 196, 0, 0, 0, 212, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0, 0, 2, 0, 176, 0,
            6, 0, 0, 0, 0, 2, 24, 0, 25, 0, 2, 0, 1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 33, 2, 0, 0,
            0, 2, 24, 0, 63, 0, 15, 0, 1, 2, 0, 0, 0, 0, 0, 5, 32, 0, 0, 0, 32, 2, 0, 0, 0, 2, 20,
            0, 63, 0, 15, 0, 1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0, 0, 2, 20, 0, 63, 0, 15, 0, 1, 1,
            0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 2, 24, 0, 25, 0, 2, 0, 1, 2, 0, 0, 0, 0, 0, 15, 2, 0,
            0, 0, 1, 0, 0, 0, 0, 2, 56, 0, 25, 0, 2, 0, 1, 10, 0, 0, 0, 0, 0, 15, 3, 0, 0, 0, 0, 4,
            0, 0, 176, 49, 128, 63, 108, 188, 99, 76, 60, 224, 80, 209, 151, 12, 161, 98, 15, 1,
            203, 25, 126, 122, 166, 192, 250, 230, 151, 241, 25, 163, 12, 206, 1, 2, 0, 0, 0, 0, 0,
            5, 32, 0, 0, 0, 32, 2, 0, 0, 1, 1, 0, 0, 0, 0, 0, 5, 18, 0, 0, 0,
        ];
        let (_, results) = SecurityKey::parse_security_key(&test, 0).unwrap();

        assert_eq!(results.reference_count, 1);
        assert_eq!(results.info.owner_sid, "S-1-5-32-544");
    }
}
