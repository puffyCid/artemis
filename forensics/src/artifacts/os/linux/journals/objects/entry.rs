use super::header::{ObjectHeader, ObjectType};
use crate::artifacts::os::linux::journals::objects::data::DataObject;
use crate::utils::nom_helper::{
    Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_sixteen_bytes,
};
use log::{error, warn};
use std::fs::File;

#[derive(Debug)]
pub(crate) struct Entry {
    pub(crate) seqnum: u64,
    pub(crate) realtime: u64,
    _monotonic: u64,
    _boot_id: u128,
    _xor_hash: u64,
    pub(crate) data_objects: Vec<DataObject>,
}

impl Entry {
    /// Parse Entry data in `Journal`
    pub(crate) fn parse_entry<'a>(
        reader: &mut File,
        data: &'a [u8],
        is_compact: &bool,
    ) -> nom::IResult<&'a [u8], Entry> {
        let (input, seqnum) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, realtime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, monotonic) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, boot_id) = nom_unsigned_sixteen_bytes(input, Endian::Le)?;
        let (mut input, xor_hash) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let mut entry = Entry {
            seqnum,
            realtime,
            _monotonic: monotonic,
            _boot_id: boot_id,
            _xor_hash: xor_hash,
            data_objects: Vec::new(),
        };
        let min_size = 4;
        while !input.is_empty() && input.len() >= min_size {
            let (_hash, offset) = if !*is_compact {
                let (remaining_input, offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
                let (remaining_input, hash) =
                    nom_unsigned_eight_bytes(remaining_input, Endian::Le)?;
                input = remaining_input;
                (hash, offset)
            } else {
                let (remaining_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
                input = remaining_input;
                (0, offset as u64)
            };

            let object_result = ObjectHeader::parse_header(reader, offset);
            let object_header = match object_result {
                Ok(result) => result,
                Err(err) => {
                    error!["[journal] Could not parse object header for data object: {err:?}"];
                    continue;
                }
            };

            if object_header.obj_type != ObjectType::Data {
                warn!("[journal] Did not get Data object type!");
                continue;
            }

            let data_result = DataObject::parse_data_object(
                &object_header.payload,
                is_compact,
                &object_header.flag,
            );
            let data_object = match data_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[journal] Could not parse data object");
                    continue;
                }
            };
            entry.data_objects.push(data_object);
        }

        Ok((input, entry))
    }
}

#[cfg(test)]
mod tests {
    use super::Entry;
    use crate::filesystem::files::file_reader;
    use std::path::PathBuf;

    #[test]
    fn test_parse_entry() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let test_data = [
            141, 6, 0, 0, 0, 0, 0, 0, 59, 199, 35, 233, 138, 255, 5, 0, 174, 136, 41, 4, 0, 0, 0,
            0, 5, 169, 105, 239, 87, 254, 73, 52, 144, 11, 89, 140, 131, 246, 45, 118, 31, 75, 110,
            249, 237, 186, 166, 71, 120, 249, 56, 0, 0, 250, 56, 0, 152, 250, 56, 0, 24, 251, 56,
            0, 184, 251, 56, 0, 72, 252, 56, 0, 232, 252, 56, 0, 144, 253, 56, 0, 72, 254, 56, 0,
            224, 254, 56, 0, 104, 255, 56, 0, 240, 255, 56, 0, 120, 0, 57, 0, 0, 1, 57, 0, 152, 1,
            57, 0, 56, 2, 57, 0, 200, 2, 57, 0, 144, 3, 57, 0, 32, 4, 57, 0, 184, 4, 57, 0, 128, 5,
            57, 0, 32, 6, 57, 0, 192, 6, 57, 0, 104, 7, 57, 0, 8, 8, 57, 0, 176, 8, 57, 0, 112, 9,
            57, 0, 48, 10, 57, 0, 216, 10, 57, 0, 136, 11, 57, 0, 24, 12, 57, 0,
        ];
        let (_, result) = Entry::parse_entry(&mut reader, &test_data, &true).unwrap();
        assert_eq!(result._boot_id, 0x762DF6838C590B903449FE57EF69A905);
        assert_eq!(result.seqnum, 1677);
        assert_eq!(result._monotonic, 69830830);
        assert_eq!(result.realtime, 1688346965559099);
        assert_eq!(result._xor_hash, 5163019554081622815);
        assert_eq!(result.data_objects.len(), 31);
        assert_eq!(result.data_objects[2].message, "TID=1712");
        assert_eq!(result.data_objects[8].message, "_TRANSPORT=journal");
        assert_eq!(result.data_objects[30].message, "_RUNTIME_SCOPE=system");
    }
}
