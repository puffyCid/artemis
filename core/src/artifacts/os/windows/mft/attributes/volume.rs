use crate::utils::nom_helper::{
    nom_unsigned_eight_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
};

#[derive(Debug)]
pub(crate) struct VolumeInfo {
    major: u8,
    minor: u8,
    flags: Vec<VolumeFlags>,
}

#[derive(Debug)]
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
    pub(crate) fn parse_volume_info(data: &[u8]) -> nom::IResult<&[u8], VolumeInfo> {
        let (input, _unknown) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, major) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, minor) = nom_unsigned_one_byte(input, Endian::Le)?;

        let (input, flag_data) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let info = VolumeInfo {
            major,
            minor,
            flags: VolumeInfo::get_flags(&flag_data),
        };

        Ok((input, info))
    }

    fn get_flags(data: &u16) -> Vec<VolumeFlags> {
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
