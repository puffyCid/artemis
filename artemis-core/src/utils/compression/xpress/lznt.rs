// Full credit to: https://github.com/ForensicRS/frnsc-prefetch/blob/main/src/decompress/lznt.rs - MIT License - 2024-03-07
use crate::utils::compression::error::CompressionError;
use std::mem;

const LZNT1_COMPRESSED_FLAG: usize = 0x8000;

macro_rules! load16le {
    ($dst:expr,$src:expr,$idx:expr) => {{
        $dst = (u32::from($src[$idx + 1]) << 8 | u32::from($src[$idx])) as usize;
    }};
}

/// Decompress LZNT compressed data
pub fn decompress_lznt(in_buf: &[u8], out_buf: &mut Vec<u8>) -> Result<(), CompressionError> {
    let mut out_idx: usize = 0;
    let mut in_idx: usize = 0;

    let mut header: usize;
    let mut length: usize;
    let mut chunk_len: usize;
    let mut offset: usize;

    let mut _block_id = 0;

    // We don't want to compute those values at each round.
    let in_buf_max_size = in_buf.len();

    while in_idx < in_buf_max_size {
        let in_chunk_base = in_idx;
        // compressed chunk header (2 bytes)
        load16le!(header, in_buf, in_idx);
        in_idx += mem::size_of::<u16>();
        chunk_len = (header & 0xfff) + 1;

        if chunk_len > (in_buf_max_size - in_idx) {
            return Err(CompressionError::LzntBadFormat);
        }

        if header & LZNT1_COMPRESSED_FLAG != 0 {
            let in_base_idx = in_idx;
            let out_base_idx = out_idx;

            let mut flag_bit = 0;
            let mut flags = in_buf[in_idx];
            in_idx += mem::size_of::<u8>();

            while (in_idx - in_base_idx) < chunk_len {
                if in_idx >= in_buf_max_size {
                    break;
                }

                if (flags & (1 << flag_bit)) == 0 {
                    if in_idx >= in_buf_max_size || (in_idx - in_base_idx) >= chunk_len {
                        break;
                    }

                    out_buf.push(in_buf[in_idx]);
                    out_idx += mem::size_of::<u8>();
                    in_idx += mem::size_of::<u8>();
                } else {
                    let copy_token;

                    if in_idx >= in_buf_max_size || (in_idx - in_base_idx) >= chunk_len {
                        break;
                    }

                    load16le!(copy_token, in_buf, in_idx);
                    in_idx += mem::size_of::<u16>();

                    let mut pos = out_idx - out_base_idx - 1;
                    let mut l_mask = 0xFFF;
                    let mut o_shift = 12;

                    while pos >= 0x10 {
                        l_mask >>= 1;
                        o_shift -= 1;
                        pos >>= 1;
                    }

                    length = (copy_token & l_mask) + 3;
                    offset = (copy_token >> o_shift) + 1;

                    if offset > out_idx {
                        return Err(CompressionError::LzntBadFormat);
                    }

                    for _i in 0..length {
                        if offset > out_idx {
                            return Err(CompressionError::LzntBadFormat);
                        }

                        out_buf.push(out_buf[out_idx - offset]);
                        out_idx += mem::size_of::<u8>();
                    }
                }

                flag_bit = (flag_bit + 1) % 8;

                if flag_bit == 0 {
                    if (in_idx - in_base_idx) >= chunk_len {
                        break;
                    }
                    flags = in_buf[in_idx];
                    in_idx += mem::size_of::<u8>();
                }
            }
        } else {
            // Not compressed
            for _i in 0..chunk_len {
                out_buf.push(in_buf[in_idx]);
                out_idx += mem::size_of::<u8>();
                in_idx += mem::size_of::<u8>();
            }
        }

        in_idx = in_chunk_base + 2 + chunk_len;
        _block_id += 1;
    }

    Ok(())
}
