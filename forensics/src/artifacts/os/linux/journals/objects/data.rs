use super::header::ObjectFlag;
use crate::utils::{
    compression::decompress::{decompress_lz4, decompress_xz, decompress_zstd},
    encoding::base64_encode_standard,
    nom_helper::{
        Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
    },
    strings::extract_utf8_string,
};
use log::error;
use nom::bytes::complete::take_until;

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
            if let Ok((_, message)) = DataObject::extract_message(&decompress_data) {
                data_object.message = message;
            }
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
            if let Ok((_, message)) = DataObject::extract_message(&decompress_data) {
                data_object.message = message;
            }
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
            if let Ok((_, message)) = DataObject::extract_message(&decompress_data) {
                data_object.message = message;
            }
        } else {
            if let Ok((_, message)) = DataObject::extract_message(input) {
                data_object.message = message;
            }
        }

        Ok((input, data_object))
    }

    /// Grab the journal message and handle any binary blobs in the message
    fn extract_message(input: &[u8]) -> nom::IResult<&[u8], String> {
        let mut message = extract_utf8_string(input);
        // Messages are suppose to be UTF8 strings
        // However raw binary blobs have been observed
        // Ex: COREDUMP_PROC_AUXV
        // Before returning base64 blob. Try one last time to extract at the message key
        if message.starts_with("[strings] Failed to get UTF8 string: ") {
            // "="
            let delimiter = [61];
            let (remaining_input, key_bytes) = take_until(delimiter.as_slice())(input)?;
            let (remaining_input, _) = nom_unsigned_one_byte(remaining_input, Endian::Le)?;
            let blob = base64_encode_standard(remaining_input);
            let key = extract_utf8_string(key_bytes);
            message = format!("{key}={blob}");
        }

        Ok((input, message))
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

    #[test]
    fn test_parse_data_object_binary_message() {
        let test_data = [
            67, 79, 82, 69, 68, 85, 77, 80, 95, 80, 82, 79, 67, 95, 65, 85, 88, 86, 61, 33, 0, 0,
            0, 0, 0, 0, 0, 0, 240, 69, 220, 48, 127, 0, 0, 51, 0, 0, 0, 0, 0, 0, 0, 48, 14, 0, 0,
            0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 255, 251, 235, 191, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0,
            0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 100, 0, 0, 0, 0, 0, 0, 0, 3, 0,
            0, 0, 0, 0, 0, 0, 64, 224, 218, 130, 205, 85, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 56, 0, 0,
            0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0,
            0, 16, 70, 220, 48, 127, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 9, 0, 0,
            0, 0, 0, 0, 0, 176, 227, 218, 130, 205, 85, 0, 0, 11, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0,
            0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0, 0, 0, 13, 0, 0, 0, 0, 0, 0,
            0, 232, 3, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 232, 3, 0, 0, 0, 0, 0, 0, 23, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 89, 75, 89, 142,
            253, 127, 0, 0, 26, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 31, 0, 0, 0, 0, 0, 0,
            0, 204, 95, 89, 142, 253, 127, 0, 0, 15, 0, 0, 0, 0, 0, 0, 0, 105, 75, 89, 142, 253,
            127, 0, 0, 27, 0, 0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 0, 28, 0, 0, 0, 0, 0, 0, 0,
            32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = DataObject::extract_message(&test_data).unwrap();
        assert_eq!(
            result,
            "COREDUMP_PROC_AUXV=IQAAAAAAAAAA8EXcMH8AADMAAAAAAAAAMA4AAAAAAAAQAAAAAAAAAP/7678AAAAABgAAAAAAAAAAEAAAAAAAABEAAAAAAAAAZAAAAAAAAAADAAAAAAAAAEDg2oLNVQAABAAAAAAAAAA4AAAAAAAAAAUAAAAAAAAADQAAAAAAAAAHAAAAAAAAAAAQRtwwfwAACAAAAAAAAAAAAAAAAAAAAAkAAAAAAAAAsOPags1VAAALAAAAAAAAAOgDAAAAAAAADAAAAAAAAADoAwAAAAAAAA0AAAAAAAAA6AMAAAAAAAAOAAAAAAAAAOgDAAAAAAAAFwAAAAAAAAAAAAAAAAAAABkAAAAAAAAAWUtZjv1/AAAaAAAAAAAAAAIAAAAAAAAAHwAAAAAAAADMX1mO/X8AAA8AAAAAAAAAaUtZjv1/AAAbAAAAAAAAABwAAAAAAAAAHAAAAAAAAAAgAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="
        );
    }
}
