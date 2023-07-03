use crate::{
    artifacts::os::linux::journals::header::IncompatFlags,
    utils::{
        nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
        strings::extract_utf8_string,
    },
};
use std::fs::File;

#[derive(Debug)]
pub(crate) struct DataObject {
    hash: u64,
    next_hash_offset: u64,
    next_field_offset: u64,
    entry_offset: u64,
    entry_array_offset: u64,
    n_entries: u64,
    tail_entry_array_offset: u32,
    tail_entry_array_n_entries: u32,
    /**May be compressed with XZ, LZ4, or ZSTD */
    pub(crate) message: String,
}

impl DataObject {
    pub(crate) fn parse_data_object<'a>(
        reader: &mut File,
        data: &'a [u8],
        is_compact: bool,
        compress_type: &[IncompatFlags],
    ) -> nom::IResult<&'a [u8], DataObject> {
        let (input, hash) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, next_hash_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, next_field_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, entry_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, entry_array_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (mut input, n_entries) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let mut data_object = DataObject {
            hash,
            next_hash_offset,
            next_field_offset,
            entry_offset,
            entry_array_offset,
            n_entries,
            tail_entry_array_offset: 0,
            tail_entry_array_n_entries: 0,
            message: String::new(),
        };
        if is_compact {
            let (remaining_input, tail_entry_array_offset) =
                nom_unsigned_four_bytes(input, Endian::Le)?;
            let (remaining_input, tail_entry_array_n_entries) =
                nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            input = remaining_input;

            data_object.tail_entry_array_offset = tail_entry_array_offset;
            data_object.tail_entry_array_n_entries = tail_entry_array_n_entries;
        }

        if compress_type.contains(&IncompatFlags::CompressedLz4) {
            panic!("lz4!");
        } else if compress_type.contains(&IncompatFlags::CompressedXz) {
            panic!("xz!");
        } else if compress_type.contains(&IncompatFlags::CompressedZstd) {
            panic!("zstd!");
        }

        let message = extract_utf8_string(input);
        data_object.message = message;

        Ok((input, data_object))
    }
}

#[cfg(test)]
mod tests {
    use super::DataObject;
    use crate::filesystem::files::file_reader;
    use std::path::PathBuf;

    #[test]
    fn test_parse_data_object() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let test_data = [
            46, 164, 30, 11, 52, 117, 233, 93, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 176,
            12, 57, 0, 0, 0, 0, 0, 48, 20, 57, 0, 0, 0, 0, 0, 69, 1, 0, 0, 0, 0, 0, 0, 136, 200,
            59, 0, 208, 0, 0, 0, 80, 82, 73, 79, 82, 73, 84, 89, 61, 54,
        ];

        let (_, result) =
            DataObject::parse_data_object(&mut reader, &test_data, true, &[]).unwrap();
        assert_eq!(result.entry_array_offset, 3740720);
        assert_eq!(result.hash, 6767068781486187566);
        assert_eq!(result.next_field_offset, 0);
        assert_eq!(result.next_hash_offset, 0);
        assert_eq!(result.n_entries, 325);
        assert_eq!(result.tail_entry_array_n_entries, 208);
        assert_eq!(result.tail_entry_array_offset, 3917960);
        assert_eq!(result.message, "PRIORITY=6");
        assert_eq!(result.entry_offset, 3738800);
    }
}
