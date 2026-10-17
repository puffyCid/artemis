use tracing::error;

/// Apply fixup values to the entry
pub(crate) fn apply_fixup(record: &mut [u8], offset: u16, count: u16) -> bool {
    let fixup_size = 2;
    let fix_offset = offset as usize;
    let fix_count = count as usize;

    if fix_count < 2 || fix_offset + fix_count * fixup_size > record.len() {
        error!("MFT fixup array does not fit in the record. Offset {offset}, count: {count}");
        return false;
    }

    let sectors = fix_count - 1;
    if !record.len().is_multiple_of(sectors) {
        error!(
            "MFT record size {} not divisible by {sectors} sectors",
            record.len()
        );
        return false;
    }

    let sector_size = record.len() / sectors;
    let placeholder = [record[fix_offset], record[fix_offset + 1]];
    for sector in 1..=sectors {
        let end = sector * sector_size;
        if record[end - fixup_size..end] != placeholder {
            continue;
        }

        let original = fix_offset + sector * fixup_size;
        record.copy_within(original..original + fixup_size, end - fixup_size);
    }

    true
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::mft::fixup::apply_fixup;

    #[test]
    fn test_apply_fixup() {
        let mut record = vec![0u8; 1024];
        record[48..54].copy_from_slice(&[129, 2, 53, 49, 7, 8]);
        record[510..512].copy_from_slice(&[129, 2]);
        record[1022..1024].copy_from_slice(&[129, 2]);

        assert!(apply_fixup(&mut record, 48, 3));
        assert_eq!(record[510..512], [53, 49]);
        assert_eq!(record[1022..1024], [7, 8]);
    }

    #[test]
    fn test_apply_fixup_offset_42() {
        let mut record = vec![0u8; 1024];
        record[42..48].copy_from_slice(&[182, 3, 76, 105, 0, 0]);
        record[510..512].copy_from_slice(&[182, 3]);
        record[1022..1024].copy_from_slice(&[182, 3]);

        assert!(apply_fixup(&mut record, 42, 3));
        assert_eq!(record[510..512], [76, 105]);
        assert_eq!(record[1022..1024], [0, 0]);
    }

    #[test]
    fn test_apply_fixup_bad_array() {
        let mut record = vec![0u8; 1024];
        assert!(!apply_fixup(&mut record, 1020, 3));
    }
}
