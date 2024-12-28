use crate::utils::nom_helper::{nom_unsigned_two_bytes, Endian};

pub(crate) struct Fixup {
    placeholder: u16,
    original: Vec<u16>,
}

impl Fixup {
    pub(crate) fn get_fixup(data: &[u8], count: u16) -> nom::IResult<&[u8], Fixup> {
        let (mut input, placeholder) = nom_unsigned_two_bytes(data, Endian::Le)?;
        let mut fixup_count = 0;

        let mut original = Vec::new();
        while fixup_count < count {
            let (remaining, value) = nom_unsigned_two_bytes(input, Endian::Le)?;
            original.push(value);
            input = remaining;
            fixup_count += 1;
        }

        let fix = Fixup {
            placeholder,
            original,
        };

        Ok((input, fix))
    }
}

#[cfg(test)]
mod tests {
    use super::Fixup;

    #[test]
    fn test_get_fixup() {
        let test = [1, 0, 13, 0, 233, 12];
        let (_, fixup) = Fixup::get_fixup(&test, 2).unwrap();
        assert_eq!(fixup.placeholder, 1);
        assert_eq!(fixup.original.len(), 2);
    }
}
