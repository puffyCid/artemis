use log::error;
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct Fixup {
    pub(crate) placeholder: Vec<u8>,
    pub(crate) original: Vec<Vec<u8>>,
}

impl Fixup {
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
                - fixup_size;
            let end = (count * (cluster_size - previous_bytes)) + cluster_size - previous_bytes;
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
}
