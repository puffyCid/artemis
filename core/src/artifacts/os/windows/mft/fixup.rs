use log::error;
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct Fixup {
    pub(crate) placeholder: Vec<u8>,
    pub(crate) original: Vec<Vec<u8>>,
}

impl Fixup {
    /// Grab fixup values that need to be applied to entries
    pub(crate) fn get_fixup(data: &[u8], count: u16) -> nom::IResult<&[u8], Fixup> {
        let fixup_size: u8 = 2;

        let (mut input, placeholder) = take(fixup_size)(data)?;
        let mut fixup_count = 0;

        let mut original = Vec::new();
        while fixup_count < count {
            let (remaining, value) = take(fixup_size)(input)?;
            original.push(value.to_vec());
            input = remaining;
            fixup_count += 1;
        }

        let fix = Fixup {
            placeholder: placeholder.to_vec(),
            original,
        };

        Ok((input, fix))
    }

    /// Apply the provided fixup values to the entry
    pub(crate) fn apply_fixup(entry: &mut [u8], fixup: &Fixup) {
        let cluster_size = 512;
        let header_size = 48;
        let fixup_size = 2;
        // We nom'd part of the bytes away, so we need to adjust to make sure fixup values are applied correctly
        let previous_bytes = header_size + (fixup.original.len() * fixup_size) + fixup_size;
        if (entry.len() + previous_bytes) % cluster_size != 0 {
            error!("[mft] MFT bytes not divisble by 512");
            return;
        }

        let sections = (entry.len() + previous_bytes) / cluster_size;
        let mut count = 0;

        while count < sections {
            let start = (count * (cluster_size - previous_bytes)) + (cluster_size - previous_bytes)
                - fixup_size
                + (count * previous_bytes);
            let end = (count * (cluster_size - previous_bytes)) + cluster_size - previous_bytes
                + (count * previous_bytes);
            if entry[start..end].to_vec() == fixup.placeholder {
                if let Some(fix) = fixup.original.get(count) {
                    // Fixup values are always two bytes. We used nom to ensure we always have two bytes
                    entry[start] = fix[0];
                    entry[end - 1] = fix[1];
                }
            }
            count += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Fixup;

    #[test]
    fn test_get_fixup() {
        let mut test = vec![1, 0, 13, 0, 233, 12];
        let (_, fixup) = Fixup::get_fixup(&mut test, 2).unwrap();
        assert_eq!(fixup.placeholder, [1, 0]);
        assert_eq!(fixup.original.len(), 2);
    }

    #[test]
    fn test_apply_fixup() {
        let mut test = vec![
            16, 0, 0, 0, 96, 0, 0, 0, 0, 0, 24, 0, 0, 0, 0, 0, 72, 0, 0, 0, 24, 0, 0, 0, 172, 119,
            65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194,
            223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 48,
            0, 0, 0, 104, 0, 0, 0, 0, 0, 24, 0, 0, 0, 3, 0, 74, 0, 0, 0, 24, 0, 1, 0, 5, 0, 0, 0,
            0, 0, 5, 0, 172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1,
            172, 119, 65, 126, 194, 223, 218, 1, 172, 119, 65, 126, 194, 223, 218, 1, 0, 0, 76, 59,
            0, 0, 0, 0, 0, 0, 76, 59, 0, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 4, 3, 36, 0, 77, 0, 70,
            0, 84, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 112, 0, 0, 0, 1, 0, 64, 0, 0, 0, 22, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 191, 194, 3, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 44, 60, 0,
            0, 0, 0, 0, 0, 44, 60, 0, 0, 0, 0, 0, 0, 44, 60, 0, 0, 0, 0, 51, 128, 191, 0, 0, 0, 12,
            51, 0, 200, 0, 64, 194, 55, 51, 79, 209, 0, 17, 185, 114, 51, 49, 171, 0, 195, 98, 129,
            66, 131, 55, 246, 221, 163, 0, 50, 253, 41, 234, 223, 24, 50, 64, 93, 108, 208, 105, 0,
            176, 0, 0, 0, 216, 0, 0, 0, 1, 0, 64, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 0, 0, 0, 30, 0, 0,
            0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 240, 1, 0, 0, 0, 0, 0, 96, 234, 1, 0, 0, 0,
            0, 0, 96, 234, 1, 0, 0, 0, 0, 0, 49, 1, 255, 255, 11, 49, 1, 38, 0, 244, 49, 1, 223,
            63, 5, 49, 1, 43, 96, 26, 49, 1, 60, 166, 5, 49, 1, 50, 87, 26, 49, 1, 200, 76, 12, 33,
            1, 204, 238, 49, 1, 246, 109, 66, 49, 1, 238, 226, 17, 49, 1, 32, 31, 1, 49, 1, 225,
            203, 12, 49, 1, 137, 240, 236, 49, 1, 131, 61, 27, 49, 1, 76, 159, 18, 49, 1, 98, 110,
            129, 2, 1, 124, 171, 0, 49, 1, 92, 175, 0, 49, 1, 213, 240, 1, 49, 1, 95, 114, 1, 33,
            1, 89, 19, 49, 1, 118, 203, 207, 49, 1, 28, 7, 57, 49, 1, 204, 82, 2, 49, 1, 9, 173,
            13, 49, 1, 247, 170, 11, 49, 1, 223, 200, 176, 33, 1, 233, 21, 65, 2, 80, 113, 183, 0,
            33, 1, 135, 146, 0, 0, 0, 0, 0, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 129, 2,
        ];
        let fix = Fixup {
            placeholder: vec![129, 2],
            original: vec![vec![53, 49], vec![0, 0], vec![0, 0]],
        };
        Fixup::apply_fixup(&mut test, &fix);
        assert!(test.ends_with(&[0, 0]));
    }
}
