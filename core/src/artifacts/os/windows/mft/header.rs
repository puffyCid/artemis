use crate::utils::nom_helper::{
    nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
};

#[derive(Debug)]
pub(crate) struct MftHeader {
    pub(crate) sig: u32,
    pub(crate) fix_up_value_offset: u16,
    pub(crate) fix_up_count: u16,
    pub(crate) transaction_seq: u64,
    pub(crate) sequence: u16,
    pub(crate) ref_count: u16,
    attrib_offset: u16,
    pub(crate) entry_flags: Vec<EntryFlags>,
    pub(crate) used_size: u32,
    pub(crate) total_size: u32,
    pub(crate) mft_base_index: u32,
    pub(crate) mft_base_seq: u16,
    first_attrib: u16,
    wfixup_patter: u16,
    pub(crate) index: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum EntryFlags {
    InUse,
    Directory,
    Extend,
    Index,
}

impl MftHeader {
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], MftHeader> {
        let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, fix_up_value_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, fix_up_count) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, transaction_seq) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, sequence) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, ref_count) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, attrib_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, entry_data) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, used_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, total_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, mft_base_index) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, mft_base_seq) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (input, first_attrib) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, wfixup_patter) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, index) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let header = MftHeader {
            sig,
            fix_up_value_offset,
            fix_up_count,
            transaction_seq,
            sequence,
            ref_count,
            attrib_offset,
            entry_flags: MftHeader::get_flags(&entry_data),
            used_size,
            total_size,
            mft_base_index,
            mft_base_seq,
            first_attrib,
            wfixup_patter,
            index,
        };

        Ok((input, header))
    }

    fn get_flags(data: &u16) -> Vec<EntryFlags> {
        let mut flags = Vec::new();
        if (data & 0x1) == 0x1 {
            flags.push(EntryFlags::InUse);
        }
        if (data & 0x2) == 0x2 {
            flags.push(EntryFlags::Directory);
        }
        if (data & 0x4) == 0x4 {
            flags.push(EntryFlags::Extend);
        }
        if (data & 0x8) == 0x8 {
            flags.push(EntryFlags::Index);
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use super::MftHeader;
    use crate::artifacts::os::windows::mft::header::EntryFlags;

    #[test]
    fn test_parse_header() {
        let test = [
            70, 73, 76, 69, 48, 0, 3, 0, 182, 200, 59, 224, 6, 0, 0, 0, 1, 0, 1, 0, 56, 0, 1, 0,
            80, 2, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 23, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = MftHeader::parse_header(&test).unwrap();
        assert_eq!(result.used_size, 592);
        assert_eq!(result.total_size, 1024);
        assert_eq!(result.transaction_seq, 29531818166);
        assert_eq!(result.sequence, 1);
        assert_eq!(result.ref_count, 1);
        assert_eq!(result.mft_base_index, 0);
        assert_eq!(result.entry_flags, vec![EntryFlags::InUse]);
    }

    #[test]
    fn test_get_flags() {
        let test = 8;
        let result = MftHeader::get_flags(&test);
        assert_eq!(result, vec![EntryFlags::Index]);
    }
}
