use super::{
    entry::Entry,
    header::{ObjectHeader, ObjectType},
};
use crate::utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian};
use log::{error, warn};
use std::fs::File;

#[derive(Debug)]
pub(crate) struct EntryArray {
    /**Log messages */
    pub(crate) entries: Vec<Entry>,
    pub(crate) next_entry_array_offset: u64,
}

impl EntryArray {
    /// Walk through Array of entries
    pub(crate) fn walk_entries<'a>(
        reader: &mut File,
        data: &'a [u8],
        is_compact: bool,
    ) -> nom::IResult<&'a [u8], EntryArray> {
        let (mut input, next_entry_array_offset) = nom_unsigned_eight_bytes(data, Endian::Le)?;

        let min_size = 4;
        let mut entry_array = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset,
        };
        let last_entry = 0;
        while !input.is_empty() && input.len() >= min_size {
            let (remaining_input, offset) = if is_compact {
                let (remaining_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
                (remaining_input, offset as u64)
            } else {
                nom_unsigned_eight_bytes(input, Endian::Le)?
            };
            input = remaining_input;

            if offset == last_entry {
                break;
            }
            let object_result = ObjectHeader::parse_header(reader, offset);
            let object_header = match object_result {
                Ok(result) => result,
                Err(err) => {
                    error!["[journal] Could not parse object header for entry in array: {err:?}"];
                    continue;
                }
            };

            if object_header.obj_type != ObjectType::Entry {
                warn!("[journal] Did not get Entry object type!");
                continue;
            }

            let entry_result = Entry::parse_entry(reader, &object_header.payload, is_compact);
            let entry = match entry_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[journal] Could not parse entry data");
                    continue;
                }
            };
            entry_array.entries.push(entry);
        }

        Ok((input, entry_array))
    }
}

#[cfg(test)]
mod tests {
    use super::EntryArray;
    use crate::{
        artifacts::os::linux::journals::{
            header::{IncompatFlags, JournalHeader},
            objects::header::{ObjectHeader, ObjectType},
        },
        filesystem::files::file_reader,
    };
    use std::{io::Read, path::PathBuf};

    #[test]
    fn test_walk_entries() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut buff = [0; 264];
        let _ = reader.read(&mut buff).unwrap();

        let (_, header) = JournalHeader::parse_header(&buff).unwrap();
        let object = ObjectHeader::parse_header(&mut reader, header.entry_array_offset).unwrap();
        assert_eq!(object.obj_type, ObjectType::EntryArray);
        let is_compact = if header.incompatible_flags.contains(&IncompatFlags::Compact) {
            true
        } else {
            false
        };
        let (_, result) =
            EntryArray::walk_entries(&mut reader, &object.payload, is_compact).unwrap();
        assert_eq!(result.next_entry_array_offset, 3744448);
        assert_eq!(result.entries.len(), 4);
    }
}
