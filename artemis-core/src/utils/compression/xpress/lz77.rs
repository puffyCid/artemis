// Full credit to: https://github.com/ForensicRS/frnsc-prefetch/blob/main/src/decompress/lz77.rs - MIT License - 2024-03-07
use crate::utils::compression::error::CompressionError;

/// Decompress LZ77 compressed data. Also referred to as just XPRESS compression
pub fn decompress_lz77(in_buf: &[u8], out_buf: &mut Vec<u8>) -> Result<(), CompressionError> {
    let mut buffered_flags = 0;
    let mut buffered_flag_count = 0;
    let mut input_position = 0;
    let mut output_position = 0;
    let mut last_length_half_byte = 0;

    loop {
        if buffered_flag_count == 0 {
            buffered_flags = u32::from_le_bytes(
                in_buf[input_position..input_position + 4]
                    .try_into()
                    .unwrap_or_default(),
            );
            input_position += 4;
            buffered_flag_count = 32;
        }
        buffered_flag_count -= 1;
        if (buffered_flags & (1 << buffered_flag_count)) == 0 {
            out_buf.push(in_buf[input_position]);
            input_position += 1;
            output_position += 1;
        } else {
            if input_position == in_buf.len() {
                return Ok(());
            }
            let match_bytes = u16::from_le_bytes(
                in_buf[input_position..input_position + 2]
                    .try_into()
                    .unwrap_or_default(),
            ) as u32;
            input_position += 2;
            let mut match_length = match_bytes % 8;
            let match_offset = (match_bytes / 8) + 1;
            if match_length == 7 {
                if last_length_half_byte == 0 {
                    match_length = (in_buf[input_position] as u32) % 16;
                    last_length_half_byte = input_position;
                    input_position += 1;
                } else {
                    match_length = (in_buf[last_length_half_byte] as u32) / 16;
                    last_length_half_byte = 0;
                }
                if match_length == 15 {
                    match_length = in_buf[input_position] as u32;
                    input_position += 1;
                    if match_length == 255 {
                        match_length = u16::from_le_bytes(
                            in_buf[input_position..input_position + 2]
                                .try_into()
                                .unwrap_or_default(),
                        ) as u32;
                        input_position += 2;
                        if match_length == 0 {
                            match_length = u32::from_le_bytes(
                                in_buf[input_position..input_position + 4]
                                    .try_into()
                                    .unwrap_or_default(),
                            );
                            input_position += 4;
                        }
                        if match_length < 22 {
                            return Err(CompressionError::Lz77BadLength);
                        }
                        match_length -= 22;
                    }
                    match_length += 15;
                }
                match_length += 7;
            }
            match_length += 3;
            for _ in 0..match_length {
                out_buf.push(out_buf[output_position - match_offset as usize]);
                output_position += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::compression::xpress::lz77::decompress_lz77;

    #[test]
    fn basic_lz77_decompression() {
        let uncompressed = b"abcdefghijklmnopqrstuvwxyz";
        let encoded: [u8; 30] = [
            0x3f, 0x00, 0x00, 0x00, 0x61, 0x62, 0x63, 0x64, 0x65, 0x66, 0x67, 0x68, 0x69, 0x6a,
            0x6b, 0x6c, 0x6d, 0x6e, 0x6f, 0x70, 0x71, 0x72, 0x73, 0x74, 0x75, 0x76, 0x77, 0x78,
            0x79, 0x7a,
        ];

        let mut decoded_value = Vec::with_capacity(1024);
        decompress_lz77(&encoded, &mut decoded_value).unwrap();
        assert_eq!(uncompressed, &decoded_value[..]);
    }

    #[test]
    fn basic_lz77_decompression_2() {
        let uncompressed = b"abcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabcabc";
        let encoded: [u8; 13] = [
            0xff, 0xff, 0xff, 0x1f, 0x61, 0x62, 0x63, 0x17, 0x00, 0x0f, 0xff, 0x26, 0x01,
        ];

        let mut decoded_value = Vec::with_capacity(1024);
        decompress_lz77(&encoded, &mut decoded_value).unwrap();
        assert_eq!(uncompressed, &decoded_value[..]);
    }

    #[test]
    fn test_decompress_ese_data() {
        let test = [
            65, 0, 0, 0, 80, 0, 69, 0, 67, 0, 109, 0, 100, 0, 32, 0, 118, 0, 101, 0, 114, 0, 115,
            0, 105, 0, 111, 0, 110, 120, 0, 49, 0, 46, 0, 52, 24, 0, 88, 81, 4, 68, 48, 26, 0, 13,
            0, 10, 26, 0, 65, 0, 117, 0, 116, 0, 104, 24, 1, 114, 0, 58, 40, 1, 69, 56, 0, 105, 0,
            99, 72, 0, 90, 56, 0, 109, 8, 0, 41, 2, 109, 0, 97, 136, 162, 136, 173, 26, 2, 40, 120,
            2, 97, 138, 0, 25, 1, 122, 40, 0, 15, 1, 132, 64, 0, 103, 74, 0, 105, 0, 108, 40, 3,
            99, 168, 2, 109, 0, 41, 58, 3, 104, 40, 3, 116, 0, 112, 248, 1, 58, 0, 47, 106, 221,
            99, 177, 8, 0, 103, 24, 1, 185, 3, 117, 0, 98, 78, 1, 47, 238, 3, 223, 3, 47, 0, 80,
            232, 0, 219, 6, 121, 2, 25, 0, 67, 170, 1, 251, 0, 137, 7, 108, 168, 1, 110, 152, 1,
            57, 6, 45, 138, 0, 67, 88, 0, 92, 162, 43, 42, 0, 0, 87, 0, 73, 0, 78, 0, 68, 0, 79,
            72, 0, 83, 120, 0, 80, 168, 2, 101, 0, 102, 24, 0, 116, 88, 3, 104, 136, 0, 105, 2, 25,
            0, 75, 152, 0, 121, 0, 119, 184, 2, 114, 189, 42, 122, 212, 248, 1, 89, 5, 32, 24, 1,
            101, 24, 3, 112, 0, 44, 90, 0, 73, 0, 73, 1, 25, 0, 76, 56, 1, 111, 0, 107, 170, 3,
            103, 232, 0, 102, 104, 0, 114, 56, 0, 112, 40, 0, 255, 2, 242, 201, 0, 137, 8, 101,
            136, 2, 240, 253, 90, 107, 32, 72, 0, 73, 10, 39, 223, 4, 16, 39, 90, 3, 29, 0, 70, 24,
            3, 117, 8, 2, 233, 6, 50, 152, 14, 54, 56, 0, 159, 1, 38, 127, 3, 153, 1, 25, 0, 25, 1,
            111, 232, 0, 153, 0, 153, 16, 139, 5, 47, 4, 15, 17, 65, 0, 77, 0, 85, 171, 70, 85, 95,
            8, 1, 69, 168, 7, 84, 104, 0, 95, 24, 1, 65, 72, 0, 67, 0, 72, 88, 0, 217, 18, 51, 8,
            0, 57, 56, 0, 49, 184, 4, 54, 8, 0, 41, 19, 46, 120, 1, 45, 152, 5, 66, 168, 0, 49, 40,
            0, 171, 213, 174, 86, 56, 216, 0, 69, 168, 0, 112, 216, 2, 191, 6, 67, 106, 3, 97, 88,
            3, 101, 186, 6, 121, 21, 201, 11, 50, 8, 2, 25, 7, 45, 56, 0, 53, 40, 0, 50, 248, 21,
            169, 0, 51, 232, 0, 50, 88, 2, 58, 216, 0, 75, 22, 171, 138, 87, 93, 77, 152, 1, 100,
            168, 6, 185, 7, 31, 2, 255, 31, 76, 8, 4, 115, 24, 4, 32, 56, 0, 99, 8, 0, 75, 9, 111,
            2, 31, 25, 0, 69, 0, 120, 248, 1, 99, 120, 13, 116, 152, 2, 98, 138, 12, 32, 72, 2, 97,
            40, 18, 43, 21, 87, 181, 186, 215, 175, 10, 111, 33, 249, 2, 72, 72, 2, 115, 232, 12,
            73, 2, 31, 11, 249, 0, 70, 104, 8, 155, 3, 137, 15, 122, 74, 0, 40, 56, 4, 121, 120, 4,
            233, 6, 41, 202, 1, 56, 200, 22, 52, 200, 2, 52, 154, 1, 86, 200, 0, 175, 33, 32, 249,
            0, 175, 187, 245, 110, 87, 88, 0, 121, 19, 111, 184, 24, 249, 21, 15, 7, 82, 216, 6,
            105, 22, 249, 29, 73, 0, 116, 138, 1, 49, 218, 0, 127, 10, 240, 114, 218, 0, 201, 0,
            255, 9, 9, 48, 88, 0, 233, 1, 25, 0, 86, 168, 2, 108, 216, 1, 105, 9, 187, 25, 11, 27,
            170, 162, 186, 189, 137, 31, 116, 120, 0, 11, 5, 73, 1, 25, 0, 35, 216, 1, 25, 3, 78,
            232, 0, 137, 1, 89, 0, 92, 24, 2, 79, 72, 4, 85, 56, 11, 69, 0, 123, 8, 1, 49, 72, 6,
            48, 56, 2, 57, 40, 1, 49, 40, 0, 99, 94, 109, 177, 170, 56, 4, 56, 232, 7, 49, 184, 3,
            52, 136, 4, 100, 40, 0, 57, 40, 0, 123, 11, 102, 0, 125, 40, 2, 83, 104, 2, 185, 36,
            97, 104, 4, 169, 2, 68, 26, 1, 68, 28, 1, 70, 136, 0, 15, 22, 4, 25, 1, 73, 7, 49, 127,
            255, 117, 219, 216, 2, 73, 7, 57, 40, 0, 233, 6, 32, 88, 0, 73, 7, 53, 168, 1, 75, 17,
            89, 2, 105, 218, 1, 99, 216, 1, 217, 6, 249, 21, 187, 35, 201, 30, 15, 14, 25, 1, 121,
            28, 57, 0, 110, 104, 1, 45, 1, 25, 6, 233, 7, 25, 0, 95, 2, 172, 223, 1, 85, 117, 85,
            188, 27, 5, 54, 170, 1, 25, 0, 203, 9, 111, 9, 255, 43, 92, 0, 36, 216, 1, 88, 24, 22,
            69, 72, 12, 68, 234, 2, 49, 239, 2, 50, 79, 37, 244, 233, 2, 50, 239, 2, 64, 92, 24, 0,
            79, 24, 12, 84, 104, 0, 65, 88, 23, 69, 200, 0, 255, 255, 95, 85, 73, 152, 0, 84, 88,
            0, 73, 72, 27, 85, 72, 0, 73, 8, 1, 78, 58, 4, 201, 14, 63, 4, 15, 103,
        ];
        let mut out = Vec::with_capacity(2048);
        decompress_lz77(&test, &mut out).unwrap();
        assert_eq!(out.len(), 2048);
    }
}
