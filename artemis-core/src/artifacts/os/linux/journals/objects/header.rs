use crate::{
    artifacts::os::linux::journals::error::JournalError,
    utils::nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_one_byte, Endian},
};
use log::error;
use nom::bytes::complete::take;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

#[derive(Debug)]
pub(crate) struct ObjectHeader {
    pub(crate) obj_type: ObjectType,
    pub(crate) flag: ObjectFlag,
    _reserved: Vec<u8>,
    size: u64,
    pub(crate) payload: Vec<u8>,
}

#[derive(PartialEq, Debug)]
pub(crate) enum ObjectType {
    Unused,
    Data,
    Field,
    Entry,
    DataHashTable,
    FieldHashTable,
    EntryArray,
    Tag,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ObjectFlag {
    CompressedXz,
    CompressedLz4,
    CompressedZstd,
    None,
}

impl ObjectHeader {
    /// Get the `Journal` object header
    pub(crate) fn parse_header(
        reader: &mut File,
        offset: u64,
    ) -> Result<ObjectHeader, JournalError> {
        if reader.seek(SeekFrom::Start(offset)).is_err() {
            error!("[journal] Could not seek to object header offset: {offset}");
            return Err(JournalError::SeekError);
        }

        let mut header_buff = [0; 16];
        if reader.read(&mut header_buff).is_err() {
            error!("[journal] Could not read minimum object header size");
            return Err(JournalError::ReadError);
        }

        let result = ObjectHeader::parse_header_data(&header_buff);
        let mut header = match result {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[journal] Could not parse object header: {err:?}");
                return Err(JournalError::ObjectHeader);
            }
        };

        let header_meta_size = 16;
        let mut payload_data: Vec<u8> = vec![0; (header.size - header_meta_size as u64) as usize];

        if reader.read(&mut payload_data).is_err() {
            error!("[journal] Could not read payload datafrom object header");
            return Err(JournalError::ReadError);
        }

        header.payload = payload_data;
        Ok(header)
    }

    /// Parse the header data
    fn parse_header_data(data: &[u8]) -> nom::IResult<&[u8], ObjectHeader> {
        let (input, obj_type) = nom_unsigned_one_byte(data, Endian::Le)?;
        let (input, flag) = nom_unsigned_one_byte(input, Endian::Le)?;

        let reserved_size: u8 = 6;
        let (input, reserved_data) = take(reserved_size)(input)?;
        let (input, size) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        // Size includes the header which we have already nom'd
        //let adjust_size = 16;
        //let (input, payload) = take(size - adjust_size)(input)?;

        let object_header = ObjectHeader {
            obj_type: ObjectHeader::object_type(&obj_type),
            flag: ObjectHeader::object_flag(&flag),
            _reserved: reserved_data.to_vec(),
            size,
            payload: Vec::new(),
        };

        Ok((input, object_header))
    }

    /// Get the Object flag in header
    fn object_flag(flag: &u8) -> ObjectFlag {
        let xz = 1;
        let lz4 = 2;
        let zstd = 4;

        if (flag & xz) == xz {
            ObjectFlag::CompressedXz
        } else if (flag & lz4) == lz4 {
            ObjectFlag::CompressedLz4
        } else if (flag & zstd) == zstd {
            ObjectFlag::CompressedZstd
        } else {
            ObjectFlag::None
        }
    }

    /// Determine the Object type
    fn object_type(obj_type: &u8) -> ObjectType {
        let data = 1;
        let field = 2;
        let entry = 3;
        let data_table = 4;
        let field_table = 5;
        let entry_array = 6;
        let tag = 7;

        if obj_type == &data_table {
            ObjectType::DataHashTable
        } else if obj_type == &data {
            ObjectType::Data
        } else if obj_type == &field {
            ObjectType::Field
        } else if obj_type == &entry {
            ObjectType::Entry
        } else if obj_type == &field_table {
            ObjectType::FieldHashTable
        } else if obj_type == &entry_array {
            ObjectType::EntryArray
        } else if obj_type == &tag {
            ObjectType::Tag
        } else {
            ObjectType::Unused
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectHeader;
    use crate::{
        artifacts::os::linux::journals::objects::header::{ObjectFlag, ObjectType},
        filesystem::files::file_reader,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/objects/objectheader.raw");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();

        let result = ObjectHeader::parse_header(&mut reader, 0).unwrap();
        assert_eq!(result.flag, ObjectFlag::None);
        assert_eq!(result.obj_type, ObjectType::EntryArray);
        assert_eq!(result.size, 0x28);
        assert_eq!(
            result.payload,
            vec![0, 0, 0, 0, 0, 0, 0, 0, 240, 41, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_parse_header_data() {
        let test_data = [
            6, 0, 0, 0, 0, 0, 0, 0, 40, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 41, 64,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, result) = ObjectHeader::parse_header_data(&test_data).unwrap();
        assert_eq!(result.flag, ObjectFlag::None);
        assert_eq!(result.obj_type, ObjectType::EntryArray);
        assert_eq!(result.size, 0x28);
        assert!(result.payload.is_empty());
    }

    #[test]
    fn test_object_type() {
        let result = ObjectHeader::object_type(&1);
        assert_eq!(result, ObjectType::Data)
    }

    #[test]
    fn test_object_flag() {
        let result = ObjectHeader::object_flag(&1);
        assert_eq!(result, ObjectFlag::CompressedXz)
    }
}
