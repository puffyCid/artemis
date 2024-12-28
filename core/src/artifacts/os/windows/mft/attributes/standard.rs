use crate::utils::nom_helper::{
    nom_signed_eight_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian,
};

pub(crate) struct Standard {
    created: u64,
    modified: u64,
    changed: u64,
    accessed: u64,
    file_attributes: Vec<FileAttributes>,
    owner_id: u32,
    sid_id: u32,
    quota: u64,
    usn: u64,
}

pub(crate) enum FileAttributes {
    ReadOnly,
    Hidden,
    System,
    Volume,
    Directory,
    Archive,
    Device,
    NOrmal,
    Temporary,
    Sparse,
    Reparse,
    Compressed,
    Offline,
    NotIndexed,
    Encrypted,
    Virtual,
    Unknown,
}

impl Standard {
    pub(crate) fn parse_standard_info(data: &[u8]) -> nom::IResult<&[u8], Standard> {
        let (input, created) = nom_unsigned_eight_bytes(data, Endian::Le)?;
        let (input, modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, changed) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, accessed) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let (input, flag_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, owner_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, sid_id) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, quota) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, usn) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let standard = Standard {
            created,
            modified,
            changed,
            accessed,
            file_attributes: todo!(),
            owner_id,
            sid_id,
            quota,
            usn,
        };
    }
}
