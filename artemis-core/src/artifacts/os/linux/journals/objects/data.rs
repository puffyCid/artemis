use super::header::ObjectFlag;
use crate::utils::{
    compression::decompress::{decompress_lz4, decompress_xz, decompress_zstd},
    encoding::base64_encode_standard,
    nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf8_string,
};
use log::error;

#[derive(Debug)]
pub(crate) struct DataObject {
    _hash: u64,
    _next_hash_offset: u64,
    _next_field_offset: u64,
    _entry_offset: u64,
    _entry_array_offset: u64,
    _n_entries: u64,
    tail_entry_array_offset: u32,
    tail_entry_array_n_entries: u32,
    /**May be compressed with XZ, LZ4, or ZSTD */
    pub(crate) message: String,
}

impl DataObject {
    /// Parse Data object in `Journal`
    pub(crate) fn parse_data_object<'a>(
        data: &'a [u8],
        is_compact: bool,
        compress_type: &ObjectFlag,
    ) -> nom::IResult<&'a [u8], DataObject> {
        let (input, hash) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, next_hash_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, next_field_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, entry_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, entry_array_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (mut input, n_entries) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let mut data_object = DataObject {
            _hash: hash,
            _next_hash_offset: next_hash_offset,
            _next_field_offset: next_field_offset,
            _entry_offset: entry_offset,
            _entry_array_offset: entry_array_offset,
            _n_entries: n_entries,
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

        if compress_type == &ObjectFlag::CompressedLz4 {
            let (remaining_input, decom_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let decompress_result = decompress_lz4(remaining_input, decom_size as usize, &[]);
            let decompress_data = match decompress_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[journal] Could not decompress lz4 data: {err:?}");
                    data_object.message = format!(
                        "Failed to decompress lz4 data: {}",
                        base64_encode_standard(input)
                    );
                    return Ok((input, data_object));
                }
            };
            let message = extract_utf8_string(&decompress_data);
            data_object.message = message;
        } else if compress_type == &ObjectFlag::CompressedXz {
            let decompress_result = decompress_xz(input);
            let decompress_data = match decompress_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[journal] Could not decompress xz data: {err:?}");
                    data_object.message = format!(
                        "Failed to decompress xz data: {}",
                        base64_encode_standard(input)
                    );
                    return Ok((input, data_object));
                }
            };
            let message = extract_utf8_string(&decompress_data);
            data_object.message = message;
        } else if compress_type == &ObjectFlag::CompressedZstd {
            let decompress_result = decompress_zstd(input);
            let decompress_data = match decompress_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[journal] Could not decompress zstd data: {err:?}");
                    data_object.message = format!(
                        "Failed to decompress zstd data: {}",
                        base64_encode_standard(input)
                    );
                    return Ok((input, data_object));
                }
            };
            let message = extract_utf8_string(&decompress_data);
            data_object.message = message;
        } else {
            let message = extract_utf8_string(input);
            data_object.message = message;
        }

        Ok((input, data_object))
    }
}

#[cfg(test)]
mod tests {
    use super::DataObject;
    use crate::artifacts::os::linux::journals::objects::header::ObjectFlag;

    #[test]
    fn test_parse_data_object() {
        let test_data = [
            46, 164, 30, 11, 52, 117, 233, 93, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 176,
            12, 57, 0, 0, 0, 0, 0, 48, 20, 57, 0, 0, 0, 0, 0, 69, 1, 0, 0, 0, 0, 0, 0, 136, 200,
            59, 0, 208, 0, 0, 0, 80, 82, 73, 79, 82, 73, 84, 89, 61, 54,
        ];

        let (_, result) =
            DataObject::parse_data_object(&test_data, true, &ObjectFlag::None).unwrap();
        assert_eq!(result._entry_array_offset, 3740720);
        assert_eq!(result._hash, 6767068781486187566);
        assert_eq!(result._next_field_offset, 0);
        assert_eq!(result._next_hash_offset, 0);
        assert_eq!(result._n_entries, 325);
        assert_eq!(result.tail_entry_array_n_entries, 208);
        assert_eq!(result.tail_entry_array_offset, 3917960);
        assert_eq!(result.message, "PRIORITY=6");
        assert_eq!(result._entry_offset, 3738800);
    }
}
