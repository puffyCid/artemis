use crate::{
    accessor::{
        error::AccessorResult,
        filesystem::ntfs::{
            attributes::{read_named_data, read_value_bytes},
            walk::ntfs_err,
        },
    },
    artifacts::os::windows::securitydescriptor::sid::grab_sid,
};
use nom::{
    bytes::complete::{take, take_until},
    number::complete::{le_u8, le_u16, le_u32, le_u64},
};
use ntfs::{
    Ntfs, NtfsAttributeType, NtfsFile, indexes::NtfsFileNameIndex, structured_values::NtfsIndexRoot,
};
use std::{
    collections::HashMap,
    io::{Read, Seek},
};
use tracing::warn;

/// Read and pares the $Secure to get SID values
///
/// We can use it to get SIDs associated with files
pub(crate) fn read_secure<R: Read + Seek>(
    ntfs: &Ntfs,
    reader: &mut R,
) -> AccessorResult<HashMap<u32, (String, String)>> {
    let Some(secure) = open_secure(ntfs, reader)? else {
        return Ok(HashMap::new());
    };

    let mut record_size = None;
    let mut attrs: ntfs::NtfsAttributes<'_, '_> = secure.attributes();

    let mut sii_bytes = None;

    while let Some(item) = attrs.next(reader) {
        let item = match item {
            Ok(item) => item,
            Err(err) => {
                warn!("Could not read $Secure attribute: {err:?}");
                continue;
            }
        };

        let attr = item.to_attribute().map_err(ntfs_err)?;
        let name = attr.name().map_err(ntfs_err)?.to_string_lossy();
        if name != "$SII" {
            continue;
        }

        match attr.ty().map_err(ntfs_err)? {
            NtfsAttributeType::IndexRoot => {
                let root = attr
                    .structured_value::<_, NtfsIndexRoot<'_>>(reader)
                    .map_err(ntfs_err)?;
                record_size = Some(root.index_record_size());
            }
            NtfsAttributeType::IndexAllocation => {
                let mut value = attr.value(reader).map_err(ntfs_err)?;
                sii_bytes = Some(read_value_bytes(&mut value, reader, attr.value_length())?);
            }
            _ => {}
        }
    }

    let (Some(record_size), Some(sii_bytes)) = (record_size, sii_bytes) else {
        warn!("$Secure file missing $SII");
        return Ok(HashMap::new());
    };

    let sds_bytes = read_named_data(reader, &secure, "$SDS")?;
    let sii = match parse_sii(&sii_bytes, record_size) {
        Ok((_, sii)) => sii,
        Err(err) => {
            warn!("Could not parse $SII: {err:?}");
            return Ok(HashMap::new());
        }
    };

    let sds = match parse_sds(&sds_bytes, &sii) {
        Ok((_, sds)) => sds,
        Err(err) => {
            warn!("Could not parse $SDS: {err:?}");
            return Ok(HashMap::new());
        }
    };

    Ok(sds
        .into_iter()
        .map(|(id, entry)| (id, (entry.user_sid, entry.group_sid)))
        .collect())
}

/// Get the $Secure NTFS File
///
/// We need to map SIDs to files
fn open_secure<'a, R: Read + Seek>(
    ntfs: &'a Ntfs,
    reader: &mut R,
) -> AccessorResult<Option<NtfsFile<'a>>> {
    let root = ntfs.root_directory(reader).map_err(ntfs_err)?;
    let index = root.directory_index(reader).map_err(ntfs_err)?;
    let mut finder = index.finder();

    let entry = match NtfsFileNameIndex::find(&mut finder, ntfs, reader, "$Secure") {
        Some(Ok(entry)) => entry,
        Some(Err(err)) => return Err(ntfs_err(err)),
        None => return Ok(None),
    };

    Ok(Some(entry.to_file(ntfs, reader).map_err(ntfs_err)?))
}

#[derive(Debug)]
pub(crate) struct SecurityIDs {
    pub(crate) sid: u32,
    pub(crate) sds_offset: u64,
    pub(crate) user_sid: String,
    pub(crate) group_sid: String,
}

/// Parse $SII INDX records
fn parse_sii(data: &[u8], record_size: u32) -> nom::IResult<&[u8], Vec<SecurityIDs>> {
    let mut sii_data = data;
    let mut sids: Vec<SecurityIDs> = Vec::new();

    while !sii_data.is_empty() && sii_data.len() > record_size as usize {
        // Get size of record
        let (remaining_data, sid_record_data) = take(record_size)(sii_data)?;
        sii_data = remaining_data;

        let indx_headers: usize = 24;
        let (mut record_data, _header) = take(indx_headers)(sid_record_data)?;

        // Search for the default $SII values. Note this will include values in slack space
        while let Ok((sid_data, _)) = search_data(record_data) {
            let (sid_data, offset) = le_u16(sid_data)?;
            let (sid_data, size) = le_u16(sid_data)?;

            let offset_size_value = 20;
            if offset != offset_size_value || size != offset_size_value {
                break;
            }

            let (sid_data, _padding) = le_u32(sid_data)?;

            let (sid_data, _index_entry_size) = le_u16(sid_data)?;
            let (sid_data, _index_entry_key) = le_u16(sid_data)?;
            let (sid_data, _flags) = le_u16(sid_data)?;
            let (sid_data, _padding2) = le_u16(sid_data)?;

            let (sid_data, sid) = le_u32(sid_data)?;
            let (sid_data, _security_descript_hash) = le_u32(sid_data)?;
            let (sid_data, _security_id2) = le_u32(sid_data)?;
            let (sid_data, sds_offset) = le_u64(sid_data)?;
            let (sid_data, _sds_data_size) = le_u32(sid_data)?;

            let security_ids = SecurityIDs {
                sid,
                sds_offset,
                user_sid: String::new(),
                group_sid: String::new(),
            };

            record_data = sid_data;
            sids.push(security_ids);
        }
    }

    Ok((sii_data, sids))
}

/// Search for default values for $SII entries
fn search_data(indx_data: &[u8]) -> nom::IResult<&[u8], &[u8]> {
    let sid_start: &[u8; 8] = &[20, 0, 20, 0, 0, 0, 0, 0];
    take_until(&sid_start[..])(indx_data)
}

/// Parse the $SDS attribute data
fn parse_sds<'a>(
    data: &'a [u8],
    security_ids: &[SecurityIDs],
) -> nom::IResult<&'a [u8], HashMap<u32, SecurityIDs>> {
    let mut sids: HashMap<u32, SecurityIDs> = HashMap::new();
    // Go through the sid and offsets found in $SII
    for sid_data in security_ids {
        // Skip any offsets larger than $SDS data. Sometimes offsets found in slack space are too large
        if sid_data.sds_offset as usize > data.len() {
            continue;
        }

        let (sds_data, _) = take(sid_data.sds_offset)(data)?;
        let sds_header: usize = 20;
        let (data_sid, _) = take(sds_header)(sds_data)?;

        let (sds_data, _revision_number) = le_u8(data_sid)?;
        let (sds_data, _padding) = le_u8(sds_data)?;

        let (sds_data, _control_flags) = le_u16(sds_data)?;
        let (sds_data, offset_sid) = le_u32(sds_data)?;
        let (sds_data, offset_group) = le_u32(sds_data)?;
        let (sds_data, _sacl_offset) = le_u32(sds_data)?;
        let (_sds_data, _dacl_offset) = le_u32(sds_data)?;

        if offset_sid as usize > data_sid.len() || offset_group as usize > data_sid.len() {
            continue;
        }

        let (_, sid_user) = parse_sid(offset_sid, data_sid)?;
        let (_, sid_group) = parse_sid(offset_group, data_sid).unwrap();

        // Skip not found SIDs
        if !sid_user.contains("S-1-") || !sid_group.contains("S-1-") {
            continue;
        }

        sids.insert(
            sid_data.sid,
            SecurityIDs {
                sid: sid_data.sid,
                sds_offset: sid_data.sds_offset,
                user_sid: sid_user,
                group_sid: sid_group,
            },
        );
    }
    Ok((data, sids))
}

/// Parse the Windows SIDs found in $SDS
fn parse_sid(offset: u32, data: &[u8]) -> nom::IResult<&[u8], String> {
    let (sid_data, _) = take(offset)(data)?;
    grab_sid(sid_data)
}

#[cfg(test)]

mod tests {
    use crate::accessor::filesystem::ntfs::security::{
        SecurityIDs, parse_sds, parse_sid, parse_sii, search_data,
    };
    use std::{
        fs::{self},
        path::PathBuf,
    };

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_secure() {
        use crate::accessor::filesystem::ntfs::{security::read_secure, volume::SectorReader};
        use ntfs::Ntfs;
        use std::{fs::File, io::BufReader};

        let device_path = format!("\\\\.\\C:");
        let file = File::open(&device_path).unwrap();
        let sector_reader = SectorReader::new(file, 4096).unwrap();
        let mut fs = BufReader::new(sector_reader);

        let mut ntfs = Ntfs::new(&mut fs).unwrap();
        ntfs.read_upcase_table(&mut fs).unwrap();

        let secure = read_secure(&ntfs, &mut fs).unwrap();
        assert!(!secure.is_empty());
    }

    #[test]
    fn test_parse_sii() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$SII");
        let buffer = fs::read(test_location).unwrap();

        let (_, result) = parse_sii(&buffer, 4096).unwrap();
        assert_eq!(result.len(), 11113);

        assert_eq!(result[0].sid, 256);
        assert_eq!(result[0].sds_offset, 0);
        assert_eq!(result[0].user_sid, "");
        assert_eq!(result[0].group_sid, "");
    }

    #[test]
    fn test_search_data() {
        let test_data = vec![20, 0, 20, 0, 0, 0, 0, 0];

        let (result, result2) = search_data(&test_data).unwrap();
        assert_eq!(result, [20, 0, 20, 0, 0, 0, 0, 0]);
        assert!(result2.is_empty());
    }

    #[test]
    fn test_parse_sid() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$SDS");
        let buffer = fs::read(test_location).unwrap();
        let (_, result) = parse_sid(4845576, &buffer).unwrap();
        assert_eq!(result, "S-1-5-18");
    }

    #[test]
    fn test_parse_sds() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$SDS");
        let buffer = fs::read(test_location).unwrap();

        let mut secuity_ids = Vec::new();
        secuity_ids.push(SecurityIDs {
            sid: 7340,
            sds_offset: 4844736,
            user_sid: String::new(),
            group_sid: String::new(),
        });

        secuity_ids.push(SecurityIDs {
            sid: 256,
            sds_offset: 0,
            user_sid: String::new(),
            group_sid: String::new(),
        });

        let (_, result) = parse_sds(&buffer, &secuity_ids).unwrap();
        assert_eq!(result.len(), 2);

        assert_eq!(result.get(&7340).unwrap().sid, 7340);
        assert_eq!(result.get(&7340).unwrap().sds_offset, 4844736);
        assert_eq!(result.get(&7340).unwrap().user_sid, "S-1-5-18");
        assert_eq!(result.get(&7340).unwrap().group_sid, "S-1-5-18");

        assert_eq!(result.get(&256).unwrap().sid, 256);
        assert_eq!(result.get(&256).unwrap().sds_offset, 0);
        assert_eq!(result.get(&256).unwrap().user_sid, "S-1-5-18");
        assert_eq!(result.get(&256).unwrap().group_sid, "S-1-5-32-544");
    }
}
