/// Validate data with CRC32C checksum
///
/// <https://wiki.osdev.org/CRC32>
/// <https://commandlinefanatic.com/cgi-bin/showarticle.cgi?article=art008>
/// <https://fuchsia.googlesource.com/third_party/wuffs/+/HEAD/std/crc32/README.md>
pub(crate) fn crc32c_check(mut crc: u32, data: &[u8]) -> u32 {
    for byte in data {
        let index = ((crc ^ *byte as u32) & 0xff) as usize;
        crc = CRC32C_TABLE[index] ^ (crc >> 8);
    }

    crc
}

/// Validate data withc CRC16 checksum
pub(crate) fn crc16_check(mut crc: u16, data: &[u8]) -> u16 {
    for byte in data {
        let index = ((crc ^ *byte as u16) & 0xff) as usize;
        crc = CRC16_TABLE[index] ^ (crc >> 8);
    }

    crc
}

/// Table for the CRC32C checks
const CRC32C_TABLE: [u32; 256] = crc32c_table();

/// Table for the CRC16 checks
const CRC16_TABLE: [u16; 256] = crc16_table();

/// Generate the CRC32C table
const fn crc32c_table() -> [u32; 256] {
    let mut table = [0_u32; 256];
    let mut index = 0;

    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;

        while bit < 8 {
            if crc & 1 == 0 {
                crc >>= 1;
            } else {
                crc = (crc >> 1) ^ 0x82f63b78;
            }

            bit += 1;
        }

        table[index] = crc;
        index += 1;
    }

    table
}

/// Generate the CRC16 table
const fn crc16_table() -> [u16; 256] {
    let mut table = [0_u16; 256];
    let mut index = 0;

    while index < 256 {
        let mut crc = index as u16;
        let mut bit = 0;

        while bit < 8 {
            if crc & 1 == 0 {
                crc >>= 1;
            } else {
                crc = (crc >> 1) ^ 0xa001;
            }

            bit += 1;
        }

        table[index] = crc;
        index += 1;
    }

    table
}

#[cfg(test)]
mod tests {
    use crate::accessor::filesystem::ext4::format::checksum::{crc16_check, crc32c_check};

    #[test]
    fn test_crc32c_check() {
        assert_eq!(crc32c_check(0xffffffff, b"123456789"), 0x1cf96d7c);
    }

    #[test]
    fn test_crc16_check() {
        assert_eq!(crc16_check(0xffff, b"123456789"), 0x4b37);
    }
}
