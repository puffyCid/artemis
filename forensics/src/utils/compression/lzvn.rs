use crate::utils::compression::error::CompressionError;

/**
 * Kind of working
 * TODO:
 * 1. Create simple test
 * 2. Create complex test
 * 3. Review
 */

/// Decompress LZVN data. Primarily seen on Apple platforms
pub(crate) fn decompress_lzvn(
    data: &[u8],
    //decompress_size: u32,
) -> Result<Vec<u8>, CompressionError> {
    let codes = lzvn_opcodes();
    let mut offset = 0;
    let mut decom_offset = 0;
    let size = data.len();
    //let mut decom_buf: Vec<u8> = Vec::with_capacity(decompress_size as usize);
    let mut decom_buf = Vec::new();
    let mut distance: u32 = 0;

    while offset <= size {
        let op = data[offset];
        offset += 1;
        // Safe because we have array of u8. And our codes are length 255
        if codes[op as usize] == LzvnOpcodes::EndOfStream {
            break;
        } else if codes[op as usize] == LzvnOpcodes::Nop {
            continue;
        }

        let (literal, match_size) =
            decom_operation(op, &mut offset, size, data, &codes, &mut distance)?;
        if literal > 0 {
            let end_offset = offset + literal as usize;
            let decom_end_offset = decom_offset + literal as usize;

            decom_buf.append(&mut data[offset..end_offset].to_vec());
            offset = end_offset;
            decom_offset = decom_end_offset;
        }

        if match_size > 0 {
            let mut match_offset = decom_offset - distance as usize;
            println!(
                "match offset: {match_offset}- match size: {match_size}. distance: {distance}. decom offset: {decom_offset}"
            );

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

        /*
        let mut literal = 0;
        let mut match_size = 0;

        match &codes[op as usize] {
            LzvnOpcodes::SmallDistance => {
                literal = (op & 0xc0) >> 6;
                match_size = ((op & 0x38) >> 3) + 3;
                distance = ((op & 0x7) << 8) | op;

                offset += 1;
            }
            LzvnOpcodes::LargeDistance => {
                let min_size = 2;
                if size - offset < min_size {
                    panic!("what LargeDistance!!!!");
                }
                offset += 1;

                literal = (op & 0xc0) >> 6;
                match_size = ((op & 0x38) >> 3) + 3;
                distance = (data[offset] << 8) | op;

                offset += 1;
            }
            LzvnOpcodes::EndOfStream => break,
            LzvnOpcodes::Nop => {}
            LzvnOpcodes::Undefined => panic!("undefined!"),
            LzvnOpcodes::PreviousDistance => {
                literal = (op & 0xc0) >> 6;
                match_size = ((op & 0x38) >> 3) + 3;
            }
            LzvnOpcodes::MediumDistance => {
                let min_size = 2;
                if size - offset < min_size {
                    panic!("what MediumDistance!!!!");
                }
                offset += 1;

                literal = (op & 0x18) >> 3;
                match_size = (((op & 0x7) << 2) | (op & 0x3)) + 3;
                distance = ((data[offset] << 6) | (op & 0xfc) >> 2);

                offset += 1;
            }
            LzvnOpcodes::SmallLiteral => {
                let small = 0xf;
                literal = op & small;
            }
            LzvnOpcodes::LargeLiteral => {
                let large = 16;
                literal = op + large;
                offset += 1;
            }
            LzvnOpcodes::SmallMatch => {
                let small = 0xf;
                match_size = op & small;
            }
            LzvnOpcodes::LargeMatch => {
                let large = 16;
                match_size = op + large;
            }
        }
        */
    }
    Ok(decom_buf)
}

fn decom_operation(
    op: u8,
    offset: &mut usize,
    size: usize,
    data: &[u8],
    codes: &[LzvnOpcodes],
    distance: &mut u32,
) -> Result<(u8, u32), CompressionError> {
    let mut literal = 0;
    let mut match_size: u32 = 0;
    println!("{:?}", codes[op as usize]);
    match &codes[op as usize] {
        LzvnOpcodes::SmallDistance => {
            literal = (op & 0xc0) >> 6;
            match_size = (((op & 0x38) >> 3) + 3) as u32;
            *distance = ((op as u32 & 0x7) << 8) | data[*offset] as u32;

            *offset += 1;
        }
        LzvnOpcodes::LargeDistance => {
            let min_size = 2;
            if size - *offset < min_size {
                panic!("what LargeDistance!!!!");
            }
            *offset += 1;

            literal = (op & 0xc0) >> 6;
            match_size = (((op & 0x38) >> 3) + 3) as u32;
            *distance = ((data[*offset] as u32) << 8) | op as u32;

            *offset += 1;
        }
        LzvnOpcodes::EndOfStream => {}
        LzvnOpcodes::Nop => {}
        LzvnOpcodes::Undefined => panic!("undefined!"),
        LzvnOpcodes::PreviousDistance => {
            literal = (op & 0xc0) >> 6;
            match_size = (((op & 0x38) >> 3) + 3) as u32;
        }
        LzvnOpcodes::MediumDistance => {
            let min_size = 2;
            if size - *offset < min_size {
                panic!("what MediumDistance!!!!");
            }
            let op_value = data[*offset];
            *offset += 1;

            literal = (op & 0x18) >> 3;
            match_size = ((((op & 0x7) << 2) | (op_value & 0x3)) + 3) as u32;
            *distance = (((data[*offset] as u32) << 6) | (op as u32 & 0xfc) >> 2);

            *offset += 1;
        }
        LzvnOpcodes::SmallLiteral => {
            let small = 0xf;
            literal = op & small;
        }
        LzvnOpcodes::LargeLiteral => {
            let large = 16;
            literal = data[*offset] + large;
            *offset += 1;
        }
        LzvnOpcodes::SmallMatch => {
            let small = 0xf;
            match_size = (op & small) as u32;
        }
        LzvnOpcodes::LargeMatch => {
            let large = 16;
            let value = data[*offset];
            match_size = value as u32 + large;
            *offset += 1
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

#[cfg(test)]
mod tests {
    use crate::{filesystem::files::read_file, utils::compression::lzvn::decompress_lzvn};
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
        //println!("{decom:?}");
        assert_eq!(decom.len(), 24191);
    }
}
