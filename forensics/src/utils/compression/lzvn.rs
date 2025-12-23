// Heavily based on https://github.com/keramics/keramics/blob/main/keramics-compression/src/lzvn.rs
// References:
//  - https://github.com/keramics/keramics/blob/main/keramics-compression/src/lzvn.rs
//  - https://github.com/fox-it/dissect.util/blob/main/dissect/util/compression/lzvn.py
//  - https://github.com/lzfse/lzfse/blob/master/src/lzvn_decode_base.c
use crate::utils::compression::error::CompressionError;

/// Decompress LZVN data. Primarily seen on Apple platforms
pub(crate) fn decompress_lzvn(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let codes = lzvn_opcodes();
    let mut offset = 0;
    let mut decom_offset = 0;
    let size = data.len();
    let mut decom_buf = Vec::new();
    let mut distance: u32 = 0;

    while offset <= size {
        let op = data[offset];
        offset += 1;
        // Safe because we have array of u8. And our codes array length is 256
        if codes[op as usize] == LzvnOpcodes::EndOfStream {
            break;
        } else if codes[op as usize] == LzvnOpcodes::Nop {
            continue;
        }

        let (literal, match_size) = decom_operation(op, &mut offset, data, &codes, &mut distance)?;
        if literal > 0 {
            let end_offset = offset + literal as usize;
            let decom_end_offset = decom_offset + literal as usize;

            decom_buf.append(&mut data[offset..end_offset].to_vec());
            offset = end_offset;
            decom_offset = decom_end_offset;
        }

        if match_size > 0 {
            let mut match_offset = decom_offset - distance as usize;

            for _value in 0..match_size {
                if decom_offset >= decom_buf.len() {
                    decom_buf.push(decom_buf[match_offset]);
                } else {
                    decom_buf[decom_offset] = decom_buf[match_offset];
                }
                match_offset += 1;
                decom_offset += 1;
            }
        }
    }
    Ok(decom_buf)
}

/// Start decompressing bytes
fn decom_operation(
    op: u8,
    offset: &mut usize,
    data: &[u8],
    codes: &[LzvnOpcodes],
    distance: &mut u32,
) -> Result<(u8, u32), CompressionError> {
    let mut literal = 0;
    let mut match_size: u32 = 0;
    let byte_width = 8;
    let large_width: u8 = 16;
    match &codes[op as usize] {
        LzvnOpcodes::SmallDistance => {
            literal = extract(op, byte_width, 6, 2);
            match_size = extract(op, byte_width, 3, 3) as u32 + 3;
            *distance = (extract(op, byte_width, 0, 3) as u32) << 8 | op_byte(data, offset)? as u32;
            *offset += 1;
        }
        LzvnOpcodes::LargeDistance => {
            let op_value = op_byte(data, offset)?;

            *offset += 1;

            literal = extract(op, byte_width, 6, 2);
            match_size = extract(op, byte_width, 3, 3) as u32 + 3;
            *distance = ((op_byte(data, offset)? as u32) << 8) | op_value as u32;

            *offset += 1;
        }
        LzvnOpcodes::EndOfStream | LzvnOpcodes::Nop => {}
        LzvnOpcodes::Undefined => return Err(CompressionError::LzvnUndefined),
        LzvnOpcodes::PreviousDistance => {
            literal = extract(op, byte_width, 6, 2);
            match_size = extract(op, byte_width, 3, 3) as u32 + 3;
        }
        LzvnOpcodes::MediumDistance => {
            let op_value = op_byte(data, offset)?;
            *offset += 1;

            //literal = (op & 0x18) >> 3;
            literal = extract(op, byte_width, 3, 2);
            //match_size = ((((op & 0x7) << 2) | (op_value & 0x3)) + 3) as u32;
            match_size = ((extract(op, byte_width, 0, 3) as u32) << 2)
                | (((extract(op_value, large_width, 0, 2)) as u32) + 3);
            *distance = ((op_byte(data, offset)? as u32) << 6) | (op_value as u32 & 0xfc) >> 2;
            *offset += 1;
        }
        LzvnOpcodes::SmallLiteral => {
            let small = 0xf;
            literal = op & small;
        }
        LzvnOpcodes::LargeLiteral => {
            let large = 16;
            literal = op_byte(data, offset)? + large;
            *offset += 1;
        }
        LzvnOpcodes::SmallMatch => {
            match_size = extract(op, byte_width, 0, 4) as u32;
        }
        LzvnOpcodes::LargeMatch => {
            let large = 16;
            let value = op_byte(data, offset)?;
            match_size = value as u32 + large;
            *offset += 1;
        }
    }
    Ok((literal, match_size))
}

#[derive(PartialEq, Debug)]
enum LzvnOpcodes {
    SmallDistance,
    LargeDistance,
    EndOfStream,
    Nop,
    Undefined,
    PreviousDistance,
    MediumDistance,
    SmallLiteral,
    LargeLiteral,
    SmallMatch,
    LargeMatch,
}

/// Array of LZVN opcodes. Used to determine byte operation to perform
fn lzvn_opcodes() -> Vec<LzvnOpcodes> {
    // https://github.com/lzfse/lzfse/blob/master/src/lzvn_decode_base.c#L50
    // https://github.com/keramics/keramics/blob/main/keramics-compression/src/lzvn.rs#L38
    vec![
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::EndOfStream,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Nop,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Nop,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::MediumDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::SmallDistance,
        LzvnOpcodes::PreviousDistance,
        LzvnOpcodes::LargeDistance,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::Undefined,
        LzvnOpcodes::LargeLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::SmallLiteral,
        LzvnOpcodes::LargeMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
        LzvnOpcodes::SmallMatch,
    ]
}

/// Get the compressed byte
fn op_byte(data: &[u8], offset: &mut usize) -> Result<u8, CompressionError> {
    if let Some(value) = data.get(*offset) {
        return Ok(*value);
    }

    Err(CompressionError::LzvnBadOffset)
}

/// Perform the bitwise operations against compressed byte
fn extract(op: u8, value_width: u8, lsb: u8, width: u8) -> u8 {
    if value_width == width {
        return op;
    }
    (op >> lsb) & ((1 << width) - 1)
}

#[cfg(test)]
mod tests {
    use crate::{
        filesystem::files::{hash_file_data, read_file},
        utils::compression::lzvn::{
            decom_operation, decompress_lzvn, extract, lzvn_opcodes, op_byte,
        },
    };
    use common::files::Hashes;
    use std::path::PathBuf;

    #[test]
    fn test_decompress_lzvn() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/lzvn/small.out");
        let bytes = read_file(&test_location.display().to_string()).unwrap();
        let decom = decompress_lzvn(&bytes).unwrap();
        assert_eq!(
            decom,
            [
                72, 101, 108, 108, 111, 32, 114, 117, 115, 116, 33, 32, 73, 32, 97, 100, 100, 101,
                100, 32, 100, 101, 99, 111, 109, 112, 114, 101, 115, 115, 105, 111, 110, 32, 115,
                117, 112, 112, 111, 114, 116, 32, 102, 111, 114, 32, 108, 122, 118, 110, 33, 32,
                87, 105, 116, 104, 32, 104, 101, 108, 112, 32, 102, 114, 111, 109, 32, 107, 101,
                114, 97, 109, 105, 99, 115, 32, 40, 104, 116, 116, 112, 115, 58, 47, 47, 103, 105,
                116, 104, 117, 98, 46, 99, 111, 109, 47, 107, 101, 114, 97, 109, 105, 99, 115, 47,
                107, 101, 114, 97, 109, 105, 99, 115, 41, 32, 97, 110, 100, 32, 100, 105, 115, 115,
                101, 99, 116, 32, 40, 104, 116, 116, 112, 115, 58, 47, 47, 103, 105, 116, 104, 117,
                98, 46, 99, 111, 109, 47, 102, 111, 120, 45, 105, 116, 47, 100, 105, 115, 115, 101,
                99, 116, 46, 117, 116, 105, 108, 41, 10, 10, 67, 111, 109, 112, 114, 101, 115, 115,
                105, 111, 110, 32, 97, 108, 103, 111, 114, 105, 116, 104, 109, 115, 32, 97, 114,
                101, 32, 100, 105, 102, 102, 105, 99, 117, 108, 116, 33, 32, 88, 68, 32, 79, 46,
                111
            ]
        );
    }

    #[test]
    fn test_decompress_lzvn_duplicate() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/lzvn/test.out");
        let bytes = read_file(&test_location.display().to_string()).unwrap();
        let decom = decompress_lzvn(&bytes).unwrap();
        assert_eq!(decom.len(), 13421);
    }

    #[test]
    fn test_decompress_lzvn_large() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/lzvn/rust.out");
        let bytes = read_file(&test_location.display().to_string()).unwrap();
        let decom = decompress_lzvn(&bytes).unwrap();
        assert_eq!(decom.len(), 24191);

        let (md5, _, _) = hash_file_data(
            &Hashes {
                md5: true,
                sha1: false,
                sha256: false,
            },
            &decom,
        );

        assert_eq!(md5, "54fa00d7a6fc158f00292a27d0c5baa0");
    }

    #[test]
    fn test_lzvn_opcodes() {
        assert_eq!(lzvn_opcodes().len(), 256);
    }

    #[test]
    #[should_panic(expected = "LzvnUndefined")]
    fn test_undefined_decom_operation() {
        let mut offset = 0;
        let mut distance = 10;
        let _ = decom_operation(114, &mut offset, &[0], &lzvn_opcodes(), &mut distance).unwrap();
    }

    #[test]
    #[should_panic(expected = "LzvnBadOffset")]
    fn test_bad_decom_operation() {
        let mut offset = 10;
        let mut distance = 10;
        let _ = decom_operation(87, &mut offset, &[0], &lzvn_opcodes(), &mut distance).unwrap();
    }

    #[test]
    fn test_op_byte() {
        let mut offset = 0;
        assert_eq!(op_byte(&[0], &mut offset).unwrap(), 0)
    }

    #[test]
    fn test_extract() {
        assert_eq!(extract(33, 8, 2, 2), 0);
    }
}
