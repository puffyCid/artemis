use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_sixteen_bytes, Endian,
    },
    strings::extract_utf8_string,
};
use nom::bytes::complete::take;
use std::mem::size_of;

#[derive(Debug)]
pub(crate) struct JournalHeader {
    sig: u64,
    compatible_flags: Vec<CompatFlags>,
    pub(crate) incompatible_flags: Vec<IncompatFlags>,
    state: State,
    reserved: Vec<u8>,
    file_id: u128,
    machine_id: u128,
    boot_id: u128,
    seqnum_id: u128,
    header_size: u64,
    arena_size: u64,
    data_hash_table_offset: u64,
    data_hash_table_size: u64,
    field_hash_table_offset: u64,
    field_hash_table_size: u64,
    tail_object_offset: u64,
    n_objects: u64,
    n_entries: u64,
    tail_entry_seqnum: u64,
    head_entry_seqnum: u64,
    pub(crate) entry_array_offset: u64,
    head_entry_realtime: u64,
    tail_entry_realtime: u64,
    tail_entry_monotonic: u64,
    /**Version 187. Header size 216 */
    n_data: u64,
    /**Version 187. Header size 216 */
    n_fields: u64,
    /**Version 189. Header size 232 */
    n_tags: u64,
    /**Version 189. Header size 232 */
    n_entry_arrays: u64,
    /**Version 246. Header size 248 */
    data_hash_chain_depth: u64,
    /**Version 246. Header size 248 */
    field_hash_chain_depth: u64,
    /**Version 252. Header size 264 */
    tail_entry_array_offset: u32,
    /**Version 252. Header size 264 */
    tail_entry_array_n_entries: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum IncompatFlags {
    CompressedXz,
    CompressedLz4,
    KeyedHash,
    CompressedZstd,
    Compact,
}

#[derive(Debug, PartialEq)]
pub(crate) enum CompatFlags {
    Sealed,
}

#[derive(Debug, PartialEq)]
pub(crate) enum State {
    Online,
    Offline,
    Archived,
    Unknown,
}

impl JournalHeader {
    /// Parse the `Journal` header data
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], JournalHeader> {
        let (input, sig) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, compatible_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, incompatible_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, state) = nom_unsigned_one_byte(input, Endian::Le)?;

        let reserved: u8 = 7;
        let (input, reserved_data) = take(reserved)(input)?;
        let (input, file_id) = nom_unsigned_sixteen_bytes(input, Endian::Be)?;
        let (input, machine_id) = nom_unsigned_sixteen_bytes(input, Endian::Be)?;
        let (input, boot_id) = nom_unsigned_sixteen_bytes(input, Endian::Be)?;
        let (input, seqnum_id) = nom_unsigned_sixteen_bytes(input, Endian::Be)?;

        let (input, header_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, arena_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, data_hash_table_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, data_hash_table_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, field_hash_table_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, field_hash_table_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, tail_object_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, n_objects) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, n_entries) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, tail_entry_seqnum) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, head_entry_seqnum) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, entry_array_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, head_entry_realtime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, tail_entry_realtime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (mut input, tail_entry_monotonic) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let version_187 = 216;
        let version_189 = 232;
        let version_246 = 248;
        let version_252 = 264;

        let mut journal_header = JournalHeader {
            sig,
            compatible_flags: JournalHeader::compat_flags(&compatible_flags),
            incompatible_flags: JournalHeader::incompat_flags(&incompatible_flags),
            state: JournalHeader::journal_state(&state),
            reserved: reserved_data.to_vec(),
            file_id,
            machine_id,
            boot_id,
            seqnum_id,
            header_size,
            arena_size,
            data_hash_table_offset,
            data_hash_table_size,
            field_hash_table_offset,
            field_hash_table_size,
            tail_object_offset,
            n_objects,
            n_entries,
            tail_entry_seqnum,
            head_entry_seqnum,
            entry_array_offset,
            head_entry_realtime,
            tail_entry_realtime,
            tail_entry_monotonic,
            n_data: 0,
            n_fields: 0,
            n_tags: 0,
            n_entry_arrays: 0,
            data_hash_chain_depth: 0,
            field_hash_chain_depth: 0,
            tail_entry_array_offset: 0,
            tail_entry_array_n_entries: 0,
        };

        // Now get extra data based on Journal/SystemD version which we can determine based on header size
        if header_size >= version_187 {
            let (remaining_input, n_data) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (remaining_input, n_fields) =
                nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
            input = remaining_input;

            journal_header.n_data = n_data;
            journal_header.n_fields = n_fields;
        }

        if header_size >= version_189 {
            let (remaining_input, n_tags) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (remaining_input, n_entry_arrays) =
                nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
            input = remaining_input;

            journal_header.n_tags = n_tags;
            journal_header.n_entry_arrays = n_entry_arrays;
        }

        if header_size >= version_246 {
            let (remaining_input, data_hash_chain_depth) =
                nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (remaining_input, field_hash_chain_depth) =
                nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
            input = remaining_input;

            journal_header.data_hash_chain_depth = data_hash_chain_depth;
            journal_header.field_hash_chain_depth = field_hash_chain_depth;
        }

        if header_size >= version_252 {
            let (remaining_input, tail_entry_array_offset) =
                nom_unsigned_four_bytes(input, Endian::Le)?;
            let (remaining_input, tail_entry_array_n_entries) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            input = remaining_input;

            journal_header.tail_entry_array_offset = tail_entry_array_offset;
            journal_header.tail_entry_array_n_entries = tail_entry_array_n_entries;
        }

        Ok((input, journal_header))
    }

    /// Get the incompatible flags. Which determine what kind of compression may be used
    pub(crate) fn incompat_flags(flag: &u32) -> Vec<IncompatFlags> {
        let xz = 1;
        let lz4 = 2;
        let keyed = 4;
        let zstd = 8;
        let compact = 16;

        let mut flags: Vec<IncompatFlags> = Vec::new();
        if (flag & xz) == xz {
            flags.push(IncompatFlags::CompressedXz);
        }
        if (flag & lz4) == lz4 {
            flags.push(IncompatFlags::CompressedLz4);
        }
        if (flag & keyed) == keyed {
            flags.push(IncompatFlags::KeyedHash);
        }
        if (flag & zstd) == zstd {
            flags.push(IncompatFlags::CompressedZstd);
        }
        if (flag & compact) == compact {
            flags.push(IncompatFlags::Compact);
        }

        flags
    }

    /// Get the compatible flag. Determines if `Sealed` format is used
    fn compat_flags(flag: &u32) -> Vec<CompatFlags> {
        let sealed = 1;

        let mut flags: Vec<CompatFlags> = Vec::new();
        if (flag & sealed) == sealed {
            flags.push(CompatFlags::Sealed);
        }

        flags
    }

    /// Get state of the `Journal`
    fn journal_state(state: &u8) -> State {
        let offline = 0;
        let online = 1;
        let archive = 2;

        if state == &offline {
            State::Offline
        } else if state == &online {
            State::Online
        } else if state == &archive {
            State::Archived
        } else {
            State::Unknown
        }
    }
}

#[cfg(test)]
mod tests {
    use super::JournalHeader;
    use crate::artifacts::os::linux::journals::header::{
        CompatFlags,
        IncompatFlags::{Compact, CompressedXz, CompressedZstd, KeyedHash},
        State,
    };

    #[test]
    fn test_parse_header() {
        let test_data = [
            76, 80, 75, 83, 72, 72, 82, 72, 0, 0, 0, 0, 28, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 125,
            230, 80, 152, 131, 41, 64, 1, 152, 138, 111, 192, 40, 225, 40, 236, 43, 175, 19, 207,
            178, 139, 76, 98, 163, 154, 141, 146, 224, 128, 195, 72, 72, 125, 242, 168, 45, 207,
            66, 136, 159, 107, 255, 253, 160, 217, 49, 129, 156, 89, 224, 122, 30, 77, 74, 232,
            181, 87, 82, 217, 155, 51, 253, 226, 8, 1, 0, 0, 0, 0, 0, 0, 248, 254, 127, 0, 0, 0, 0,
            0, 248, 21, 0, 0, 0, 0, 0, 0, 128, 227, 56, 0, 0, 0, 0, 0, 24, 1, 0, 0, 0, 0, 0, 0,
            208, 20, 0, 0, 0, 0, 0, 0, 24, 43, 64, 0, 0, 0, 0, 0, 15, 11, 0, 0, 0, 0, 0, 0, 12, 3,
            0, 0, 0, 0, 0, 0, 109, 12, 4, 0, 0, 0, 0, 0, 235, 8, 4, 0, 0, 0, 0, 0, 168, 12, 57, 0,
            0, 0, 0, 0, 3, 147, 7, 238, 98, 255, 5, 0, 44, 153, 42, 239, 98, 255, 5, 0, 43, 43,
            115, 6, 0, 0, 0, 0, 43, 5, 0, 0, 0, 0, 0, 0, 46, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 168, 2, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 32,
            120, 59, 0, 174, 1, 0, 0,
        ];
        let (_, result) = JournalHeader::parse_header(&test_data).unwrap();
        println!("{result:?}");
        assert_eq!(result.sig, 5211307194293375052);
        assert!(result.compatible_flags.is_empty());
        assert_eq!(
            result.incompatible_flags,
            vec![KeyedHash, CompressedZstd, Compact]
        );
        assert_eq!(result.state, State::Online);
        assert_eq!(result.file_id, 0x7de6509883294001988a6fc028e128ec);
        assert_eq!(result.machine_id, 0x2baf13cfb28b4c62a39a8d92e080c348);
        assert_eq!(result.boot_id, 0x487df2a82dcf42889f6bfffda0d93181);
        assert_eq!(result.seqnum_id, 0x9c59e07a1e4d4ae8b55752d99b33fde2);
        assert_eq!(result.header_size, 264);
        assert_eq!(result.n_objects, 2831);
        assert_eq!(result.n_entries, 780);
        assert_eq!(result.data_hash_chain_depth, 1);
        assert_eq!(result.tail_entry_array_n_entries, 430);
    }

    #[test]
    fn test_incompat_flags() {
        let test_data = 1;
        let results = JournalHeader::incompat_flags(&test_data);
        assert_eq!(results[0], CompressedXz);
    }

    #[test]
    fn test_compat_flags() {
        let test_data = 1;
        let results = JournalHeader::compat_flags(&test_data);
        assert_eq!(results[0], CompatFlags::Sealed);
    }

    #[test]
    fn test_journal_state() {
        let test_data = 1;
        let results = JournalHeader::journal_state(&test_data);
        assert_eq!(results, State::Online);
    }
}
