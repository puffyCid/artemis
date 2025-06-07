use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    strings::extract_utf8_string,
    uuid::format_guid_le_bytes,
};
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

#[derive(Debug)]
pub(crate) struct Tracker {
    _size: u32,            // 96 bytes
    _sig: u32,             // 0xa0000003
    _tracker_size: u32,    // 88 bytes
    _tracker_version: u32, // 0 version
    pub(crate) machine_id: String,
    pub(crate) droid_volume_id: String,
    pub(crate) droid_file_id: String,
    pub(crate) birth_droid_volume_id: String,
    pub(crate) birth_droid_file_id: String,
}

/// Determine if extra Tracker data exists in `Shortcut` data
pub(crate) fn has_tracker(data: &[u8]) -> (bool, Tracker) {
    let result = parse_tracker(data);
    match result {
        Ok((_, tracker)) => (true, tracker),
        Err(_err) => {
            let tracker = Tracker {
                _size: 0,
                _sig: 0,
                _tracker_size: 0,
                _tracker_version: 0,
                machine_id: String::new(),
                droid_volume_id: String::new(),
                droid_file_id: String::new(),
                birth_droid_volume_id: String::new(),
                birth_droid_file_id: String::new(),
            };
            (false, tracker)
        }
    }
}

/// Scan for Tracker data and parse if exists
fn parse_tracker(data: &[u8]) -> nom::IResult<&[u8], Tracker> {
    let sig = [3, 0, 0, 160];
    let (_, sig_start) = take_until(sig.as_slice())(data)?;

    let adjust_start = 4;
    let (tracker_start, _) = take(sig_start.len() - adjust_start)(data)?;
    let (input, size) = nom_unsigned_four_bytes(tracker_start, Endian::Le)?;
    let (input, sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, tracker_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, tracker_version) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, machine_data) = take(size_of::<u128>())(input)?;
    let (input, droid_volume) = take(size_of::<u128>())(input)?;
    let (input, droid_file) = take(size_of::<u128>())(input)?;
    let (input, birth_volume) = take(size_of::<u128>())(input)?;
    let (input, birth_file) = take(size_of::<u128>())(input)?;

    let tracker = Tracker {
        _size: size,
        _sig: sig,
        _tracker_size: tracker_size,
        _tracker_version: tracker_version,
        machine_id: extract_utf8_string(machine_data),
        droid_volume_id: format_guid_le_bytes(droid_volume),
        droid_file_id: format_guid_le_bytes(droid_file),
        birth_droid_volume_id: format_guid_le_bytes(birth_volume),
        birth_droid_file_id: format_guid_le_bytes(birth_file),
    };

    Ok((input, tracker))
}

#[cfg(test)]
mod tests {
    use super::parse_tracker;
    use crate::artifacts::os::windows::shortcuts::extras::tracker::has_tracker;

    #[test]
    fn test_has_tracker() {
        let test = [
            96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0, 0, 100, 101, 115, 107, 116, 111, 112,
            45, 101, 105, 115, 57, 51, 56, 110, 0, 104, 69, 141, 62, 17, 228, 24, 73, 143, 120,
            151, 205, 108, 179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39, 110,
            180, 94, 104, 69, 141, 62, 17, 228, 24, 73, 143, 120, 151, 205, 108, 179, 64, 197, 192,
            88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39, 110, 180, 94,
        ];
        let (has_track, result) = has_tracker(&test);
        assert_eq!(has_track, true);
        assert_eq!(result._size, 96);
        assert_eq!(result._sig, 2684354563);
        assert_eq!(result._tracker_size, 88);
        assert_eq!(result._tracker_version, 0);
        assert_eq!(result.machine_id, "desktop-eis938n");
        assert_eq!(
            result.droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
        assert_eq!(result.droid_file_id, "09f158c0-5a6a-11ed-a10d-0800276eb45e");
        assert_eq!(
            result.birth_droid_file_id,
            "09f158c0-5a6a-11ed-a10d-0800276eb45e"
        );
        assert_eq!(
            result.birth_droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
    }

    #[test]
    fn test_parse_tracker() {
        let test = [
            96, 0, 0, 0, 3, 0, 0, 160, 88, 0, 0, 0, 0, 0, 0, 0, 100, 101, 115, 107, 116, 111, 112,
            45, 101, 105, 115, 57, 51, 56, 110, 0, 104, 69, 141, 62, 17, 228, 24, 73, 143, 120,
            151, 205, 108, 179, 64, 197, 192, 88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39, 110,
            180, 94, 104, 69, 141, 62, 17, 228, 24, 73, 143, 120, 151, 205, 108, 179, 64, 197, 192,
            88, 241, 9, 106, 90, 237, 17, 161, 13, 8, 0, 39, 110, 180, 94,
        ];
        let (_, result) = parse_tracker(&test).unwrap();
        assert_eq!(result._size, 96);
        assert_eq!(result._sig, 2684354563);
        assert_eq!(result._tracker_size, 88);
        assert_eq!(result._tracker_version, 0);
        assert_eq!(result.machine_id, "desktop-eis938n");
        assert_eq!(
            result.droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
        assert_eq!(result.droid_file_id, "09f158c0-5a6a-11ed-a10d-0800276eb45e");
        assert_eq!(
            result.birth_droid_file_id,
            "09f158c0-5a6a-11ed-a10d-0800276eb45e"
        );
        assert_eq!(
            result.birth_droid_volume_id,
            "3e8d4568-e411-4918-8f78-97cd6cb340c5"
        );
    }
}
