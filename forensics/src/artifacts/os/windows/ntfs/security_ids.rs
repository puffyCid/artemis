use super::{
    attributes::{get_attribute_name, get_attribute_type},
    error::NTFSError,
};
use crate::{
    artifacts::os::windows::securitydescriptor::sid::grab_sid,
    filesystem::ntfs::{
        attributes::get_filename_attribute, raw_files::raw_read_data, sector_reader::SectorReader,
    },
    utils::nom_helper::{
        Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes,
    },
};
use log::error;
use nom::bytes::complete::{take, take_until};
use ntfs::{Ntfs, NtfsAttributes, NtfsFile, structured_values::NtfsIndexRoot};
use std::{collections::HashMap, fs::File, io::BufReader};

#[derive(Debug)]
pub(crate) struct SecurityIDs {
    pub(crate) sid: u32,
    pub(crate) sds_offset: u64,
    pub(crate) user_sid: String,
    pub(crate) group_sid: String,
}

impl SecurityIDs {
    /// Get Windows SID info from $SII and $SDS attributes
    pub(crate) fn get_security_ids(
        root_dir: &NtfsFile<'_>,
        fs: &mut BufReader<SectorReader<File>>,
        ntfs: &Ntfs,
    ) -> Result<HashMap<u32, SecurityIDs>, NTFSError> {
        // $Secure file exists in root directory
        let index_result = root_dir.directory_index(fs);
        let index = match index_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[ntfs] Failed to get NTFS index directory for Security IDs, error: {err:?}"
                );
                return Err(NTFSError::IndexDir);
            }
        };

        let mut iter = index.entries();
        let mut security_ids: HashMap<u32, SecurityIDs> = HashMap::new();
        // Looping through files at Root directory until we get to $Secure file, then we will need to parse $SII and $SDS attributes
        while let Some(entry) = iter.next(fs) {
            let entry_result = entry;
            let entry_index = match entry_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS entry index, error: {err:?}");
                    continue;
                }
            };

            let filename_result = entry_index.key();
            let filename_result = match filename_result {
                Some(result) => get_filename_attribute(&result),
                None => continue,
            };
            let filename = match filename_result {
                Ok(result) => result,
                Err(_err) => return Err(NTFSError::FilenameInfo),
            };

            if filename.name() != "$Secure" {
                continue;
            }

            let ntfs_file_result = entry_index.file_reference().to_file(ntfs, fs);
            let ntfs_file = match ntfs_file_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS $Secure file, error: {err:?}");
                    break;
                }
            };

            let mut ntfs_attr = ntfs_file.attributes();
            while let Some(attribute) = ntfs_attr.next(fs) {
                let attr_result = attribute;
                let attr = match attr_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[ntfs] Failed to get $SDS or $SII attributes: {err:?}");
                        continue;
                    }
                };

                let attr_data_result = attr.to_attribute();
                let attr_data = match attr_data_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[ntfs] Failed to get NTFS attribute error: {err:?}");
                        continue;
                    }
                };
                let attr_name = get_attribute_name(&attr_data);
                let attr_type = get_attribute_type(&attr_data);

                if attr_name == "$SII" && attr_type == "IndexRoot" {
                    let indx_root_result = attr_data.structured_value(fs);
                    let indx_root = match indx_root_result {
                        Ok(result) => result,
                        Err(err) => {
                            error!("[ntfs] Failed to get NTFS INDX root: {err:?}");
                            break;
                        }
                    };
                    // We need to parse $SII first
                    let sids_vec =
                        SecurityIDs::get_sii(&indx_root, fs, &mut ntfs_file.attributes())?;
                    security_ids = SecurityIDs::get_sds(fs, &mut ntfs_file.attributes(), &sids_vec);
                }
            }
            break;
        }
        Ok(security_ids)
    }

    /// Get the $SII attribute data
    fn get_sii(
        indx_root: &NtfsIndexRoot<'_>,
        fs: &mut BufReader<SectorReader<File>>,
        attributes: &mut NtfsAttributes<'_, '_>,
    ) -> Result<Vec<SecurityIDs>, NTFSError> {
        let mut sids: Vec<SecurityIDs> = Vec::new();
        while let Some(attribute) = attributes.next(fs) {
            let attr_result = attribute;
            let attr = match attr_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get $SII attribute item: {err:?}");
                    continue;
                }
            };

            let attr_data_result = attr.to_attribute();
            let attr_data = match attr_data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS attribute error: {err:?}");
                    continue;
                }
            };
            let attr_name = get_attribute_name(&attr_data);
            let attr_type = get_attribute_type(&attr_data);

            if attr_name != "$SII" || attr_type != "IndexAllocation" {
                continue;
            }

            let data_result = attr_data.value(fs);
            let mut data_attr_value = match data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get $SII INDX data error: {err:?}");
                    continue;
                }
            };

            // Read the whole INDX record (they are very small)
            let buff_results = raw_read_data(&mut data_attr_value, fs);
            let buff_data = match buff_results {
                Ok(results) => results,
                Err(err) => {
                    error!("[ntfs] Failed to read $SII INDX: {err:?}");
                    return Ok(sids);
                }
            };
            // Now parse the INDX record
            let result = SecurityIDs::parse_sii(&buff_data, indx_root.index_record_size());
            match result {
                Ok((_, sid_data)) => sids.extend(sid_data.into_iter()),
                Err(err) => {
                    error!(
                        "[ntfs] Failed to parse $SII will not be able to lookup SID information, error: {err:?}"
                    );
                }
            }
            break;
        }
        Ok(sids)
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

            loop {
                // Search for the default $SII values. Note this will include values in slack space
                let search_result = SecurityIDs::search_data(record_data);
                let sid_data = match search_result {
                    Ok((result, _)) => result,
                    Err(_) => break,
                };

                let (sid_data, offset) = nom_unsigned_two_bytes(sid_data, Endian::Le)?;
                let (sid_data, size) = nom_unsigned_two_bytes(sid_data, Endian::Le)?;

                let offset_size_value = 20;
                if offset != offset_size_value || size != offset_size_value {
                    break;
                }

                let (sid_data, _padding) = nom_unsigned_four_bytes(sid_data, Endian::Le)?;

                let (sid_data, _index_entry_size) = nom_unsigned_two_bytes(sid_data, Endian::Le)?;
                let (sid_data, _index_entry_key) = nom_unsigned_two_bytes(sid_data, Endian::Le)?;
                let (sid_data, _flags) = nom_unsigned_two_bytes(sid_data, Endian::Le)?;
                let (sid_data, _padding2) = nom_unsigned_two_bytes(sid_data, Endian::Le)?;

                let (sid_data, sid) = nom_unsigned_four_bytes(sid_data, Endian::Le)?;
                let (sid_data, _security_descript_hash) =
                    nom_unsigned_four_bytes(sid_data, Endian::Le)?;
                let (sid_data, _security_id2) = nom_unsigned_four_bytes(sid_data, Endian::Le)?;
                let (sid_data, sds_offset) = nom_unsigned_eight_bytes(sid_data, Endian::Le)?;
                let (sid_data, _sds_data_size) = nom_unsigned_four_bytes(sid_data, Endian::Le)?;

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

    /// Get the $SDS attribute data
    fn get_sds(
        fs: &mut BufReader<SectorReader<File>>,
        attributes: &mut NtfsAttributes<'_, '_>,
        security_ids: &[SecurityIDs],
    ) -> HashMap<u32, SecurityIDs> {
        let mut sids: HashMap<u32, SecurityIDs> = HashMap::new();
        while let Some(attribute) = attributes.next(fs) {
            let attr_result = attribute;
            let attr = match attr_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get $SDS attribute item: {err:?}");
                    continue;
                }
            };

            let attr_data_result = attr.to_attribute();
            let attr_data = match attr_data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS attribute error: {err:?}");
                    continue;
                }
            };
            let attr_name = get_attribute_name(&attr_data);
            let attr_type = get_attribute_type(&attr_data);

            if attr_name != "$SDS" || attr_type != "Data" {
                continue;
            }

            let data_result = attr_data.value(fs);
            let mut data_attr_value = match data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[ntfs] Failed to get NTFS $SDS data error: {:?}", err);
                    continue;
                }
            };
            // $SDS data is under Data attribute type, read the whole data
            let buff_results = raw_read_data(&mut data_attr_value, fs);
            let buff_data = match buff_results {
                Ok(results) => results,
                Err(err) => {
                    error!("[ntfs] Failed to read $SDS INDX: {err:?}");
                    return sids;
                }
            };
            // Using the $SII data parse the $SDS data
            let result = SecurityIDs::parse_sds(&buff_data, security_ids);
            match result {
                Ok((_, result)) => sids = result,
                Err(err) => {
                    error!(
                        "[ntfs] Failed to parse $SDS will not be able to lookup SID information, error: {err:?}"
                    );
                    break;
                }
            }
        }
        sids
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

            let (sds_data, _revision_number) = nom_unsigned_one_byte(data_sid, Endian::Le)?;
            let (sds_data, _padding) = nom_unsigned_one_byte(sds_data, Endian::Le)?;

            let (sds_data, _control_flags) = nom_unsigned_two_bytes(sds_data, Endian::Le)?;
            let (sds_data, offset_sid) = nom_unsigned_four_bytes(sds_data, Endian::Le)?;
            let (sds_data, offset_group) = nom_unsigned_four_bytes(sds_data, Endian::Le)?;
            let (sds_data, _sacl_offset) = nom_unsigned_four_bytes(sds_data, Endian::Le)?;
            let (_sds_data, _dacl_offset) = nom_unsigned_four_bytes(sds_data, Endian::Le)?;

            if offset_sid as usize > data_sid.len() || offset_group as usize > data_sid.len() {
                continue;
            }

            let (_, sid_user) = SecurityIDs::parse_sid(offset_sid, data_sid)?;
            let (_, sid_group) = SecurityIDs::parse_sid(offset_group, data_sid).unwrap();

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

    /// Search for default values for $SII entries
    fn search_data(indx_data: &[u8]) -> nom::IResult<&[u8], &[u8]> {
        let sid_start: &[u8; 8] = &[20, 0, 20, 0, 0, 0, 0, 0];
        take_until(sid_start.as_slice())(indx_data)
    }

    /// Lookup the NTFS SID value and get the Windows User and Group SIDs (if any)
    pub(crate) fn lookup_sids(
        sid: &u32,
        security_ids: &HashMap<u32, SecurityIDs>,
    ) -> (String, String) {
        let sid_option = security_ids.get(sid);
        match sid_option {
            Some(result) => (result.user_sid.clone(), result.group_sid.clone()),
            None => (String::new(), String::new()),
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        filesystem::ntfs::sector_reader::SectorReader,
        structs::artifacts::os::windows::RawFilesOptions,
    };
    use ntfs::Ntfs;
    use std::{
        collections::HashMap,
        fs::{self, File},
        io::BufReader,
        path::PathBuf,
    };

    use super::SecurityIDs;

    #[test]
    fn test_get_security_ids() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 2,
            recover_indx: false,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let drive_path = format!("\\\\.\\{}:", test_path.drive_letter);
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let _result = SecurityIDs::get_security_ids(&root_dir, &mut fs, &ntfs).unwrap();
    }

    #[test]
    fn test_get_sii() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 2,
            recover_indx: false,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let drive_path = format!("\\\\.\\{}:", test_path.drive_letter);
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let index = root_dir.directory_index(&mut fs).unwrap();

        let mut iter = index.entries();
        let mut secuity_ids: Vec<SecurityIDs> = Vec::new();

        // Looping through files at Root directory until we get to $Secure file
        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();

            let filename = entry_index.key().unwrap().unwrap();

            if filename.name() != "$Secure" {
                continue;
            }

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();

            let mut test = ntfs_file.attributes();
            while let Some(attribute) = test.next(&mut fs) {
                let attr = attribute.unwrap();
                let attr_data = attr.to_attribute().unwrap();
                let attr_name = attr_data.name().unwrap();
                let attr_type = attr_data.ty().unwrap();

                if attr_name == "$SII" && attr_type.to_string() == "IndexRoot" {
                    let indx_root = attr_data.structured_value(&mut fs).unwrap();

                    secuity_ids =
                        SecurityIDs::get_sii(&indx_root, &mut fs, &mut ntfs_file.attributes())
                            .unwrap();
                }
            }
        }
        assert!(secuity_ids.len() > 10)
    }

    #[test]
    fn test_get_sds() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\"),
            depth: 2,
            recover_indx: false,
            md5: Some(true),
            sha1: Some(false),
            sha256: Some(false),
            metadata: Some(false),
            path_regex: Some(String::new()),
            filename_regex: Some(String::new()),
        };
        let drive_path = format!("\\\\.\\{}:", test_path.drive_letter);
        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let index = root_dir.directory_index(&mut fs).unwrap();

        let mut iter = index.entries();
        let mut secuity_ids: Vec<SecurityIDs> = Vec::new();

        // Looping through files at Root directory until we get to $Secure file
        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();

            let filename = entry_index.key().unwrap().unwrap();

            if filename.name() != "$Secure" {
                continue;
            }

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();

            let mut test = ntfs_file.attributes();
            while let Some(attribute) = test.next(&mut fs) {
                let attr = attribute.unwrap();
                let attr_data = attr.to_attribute().unwrap();
                let attr_name = attr_data.name().unwrap();
                let attr_type = attr_data.ty().unwrap();

                if attr_name == "$SII" && attr_type.to_string() == "IndexRoot" {
                    let indx_root = attr_data.structured_value(&mut fs).unwrap();

                    secuity_ids =
                        SecurityIDs::get_sii(&indx_root, &mut fs, &mut ntfs_file.attributes())
                            .unwrap();
                    SecurityIDs::get_sds(&mut fs, &mut ntfs_file.attributes(), &mut secuity_ids);
                }
            }
        }
        assert!(secuity_ids.len() > 10)
    }

    #[test]
    fn test_parse_sds() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$SDS");
        let buffer = fs::read(test_location).unwrap();

        let mut secuity_ids: Vec<SecurityIDs> = Vec::new();
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

        let (_, result) = SecurityIDs::parse_sds(&buffer, &secuity_ids).unwrap();
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

    #[test]
    fn test_parse_sid() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$SDS");
        let buffer = fs::read(test_location).unwrap();
        let (_, result) = SecurityIDs::parse_sid(4845576, &buffer).unwrap();
        assert_eq!(result, "S-1-5-18");
    }

    #[test]
    fn test_parse_sii() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/ntfs/$SII");
        let buffer = fs::read(test_location).unwrap();

        let (_, result) = SecurityIDs::parse_sii(&buffer, 4096).unwrap();
        assert_eq!(result.len(), 11113);

        assert_eq!(result[0].sid, 256);
        assert_eq!(result[0].sds_offset, 0);
        assert_eq!(result[0].user_sid, "");
        assert_eq!(result[0].group_sid, "");
    }

    #[test]
    fn test_search_data() {
        let test_data = vec![20, 0, 20, 0, 0, 0, 0, 0];

        let (result, result2) = SecurityIDs::search_data(&test_data).unwrap();
        assert_eq!(result, [20, 0, 20, 0, 0, 0, 0, 0]);
        assert_eq!(result2.is_empty(), true);
    }

    #[test]
    fn test_lookup_sids() {
        let sid = 0;
        let mut test_data = HashMap::new();
        test_data.insert(
            0,
            SecurityIDs {
                sid: 0,
                sds_offset: 0,
                user_sid: String::new(),
                group_sid: String::new(),
            },
        );

        let result = SecurityIDs::lookup_sids(&sid, &test_data);
        assert_eq!(result.0, "");
        assert_eq!(result.1, "");
    }
}
