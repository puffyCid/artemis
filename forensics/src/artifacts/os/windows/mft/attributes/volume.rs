use crate::utils::nom_helper::{
    Endian, nom_unsigned_eight_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct VolumeInfo {
    major: u8,
    minor: u8,
    flags: Vec<VolumeFlags>,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum VolumeFlags {
    Dirty,
    ResizeLogFIle,
    UpgradeOnMount,
    MountedOnNt4,
    DeleteUsnUnderway,
    RepairObjectId,
    ChkdskUnderway,
    ModifiedByChkdsk,
    Unknown,
}

impl VolumeInfo {
    /// Parse volume attribute
    pub(crate) fn parse_volume_info(data: &[u8]) -> nom::IResult<&[u8], VolumeInfo> {
        let (input, _unknown) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, major) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, minor) = nom_unsigned_one_byte(input, Endian::Le)?;

        let (input, flag_data) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let info = VolumeInfo {
            major,
            minor,
            flags: VolumeInfo::get_flags(flag_data),
        };

        Ok((input, info))
    }

    /// Determine volume flags
    fn get_flags(data: u16) -> Vec<VolumeFlags> {
        let mut flags = Vec::new();

        if (data & 0x1) == 0x1 {
            flags.push(VolumeFlags::Dirty);
        } else if (data & 0x2) == 0x2 {
            flags.push(VolumeFlags::ResizeLogFIle);
        } else if (data & 0x4) == 0x4 {
            flags.push(VolumeFlags::UpgradeOnMount);
        } else if (data & 0x8) == 0x8 {
            flags.push(VolumeFlags::MountedOnNt4);
        } else if (data & 0x10) == 0x10 {
            flags.push(VolumeFlags::DeleteUsnUnderway);
        } else if (data & 0x20) == 0x20 {
            flags.push(VolumeFlags::RepairObjectId);
        } else if (data & 0x80) == 0x80 {
            flags.push(VolumeFlags::Unknown);
        } else if (data & 0x4000) == 0x4000 {
            flags.push(VolumeFlags::ChkdskUnderway);
        } else if (data & 0x8000) == 0x8000 {
            flags.push(VolumeFlags::ModifiedByChkdsk);
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use super::VolumeInfo;

    #[test]
    fn test_parse_volume_info() {
        let test = [0, 0, 0, 0, 0, 0, 0, 0, 3, 1, 0, 0, 0, 0, 0, 0];
        let (_, volume) = VolumeInfo::parse_volume_info(&test).unwrap();

        assert_eq!(volume.flags, vec![]);
        assert_eq!(volume.major, 3);
        assert_eq!(volume.minor, 1)
    }

    #[test]
    fn test_get_flags() {
        let test = [0x1, 0x2, 0x4, 0x8, 0x10, 0x20, 0x80, 0x4000, 0x8000];
        for entry in test {
            assert!(!VolumeInfo::get_flags(entry).is_empty());
        }
    }
}
