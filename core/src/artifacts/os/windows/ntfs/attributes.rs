use super::error::NTFSError;
use crate::{
    filesystem::{
        files::hash_file_data,
        ntfs::{
            attributes::get_filename_attribute, compression::check_wofcompressed,
            raw_files::raw_hash_data, sector_reader::SectorReader,
        },
    },
    utils::time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::files::Hashes;
use common::windows::{ADSInfo, CompressionType, RawFilelist};
use log::error;
use ntfs::{
    structured_values::{
        NtfsAttributeList, NtfsFileName, NtfsFileNamespace, NtfsStandardInformation,
    },
    Ntfs, NtfsAttribute, NtfsAttributeType, NtfsError, NtfsFile, NtfsFileReference,
};
use std::{fs::File, io::BufReader};

/// Get filename and Filename timestamps
pub(crate) fn filename_info(
    filename_result: &Result<NtfsFileName, NtfsError>,
    file_info: &mut RawFilelist,
) -> Result<(), NTFSError> {
    let filename_result = get_filename_attribute(filename_result);
    let filename = match filename_result {
        Ok(result) => result,
        Err(_err) => return Err(NTFSError::FilenameInfo),
    };

    if filename.namespace() == NtfsFileNamespace::Dos {
        return Err(NTFSError::Dos);
    }

    filename.parent_directory_reference().file_record_number();

    file_info.filename = filename.name().to_string().unwrap_or_default();
    file_info.filename_created = unixepoch_to_iso(&filetime_to_unixepoch(
        &filename.creation_time().nt_timestamp(),
    ));
    file_info.filename_modified = unixepoch_to_iso(&filetime_to_unixepoch(
        &filename.modification_time().nt_timestamp(),
    ));
    file_info.filename_changed = unixepoch_to_iso(&filetime_to_unixepoch(
        &filename.mft_record_modification_time().nt_timestamp(),
    ));
    file_info.filename_accessed = unixepoch_to_iso(&filetime_to_unixepoch(
        &filename.access_time().nt_timestamp(),
    ));
    Ok(())
}

/// Get Standard attributes data
pub(crate) fn standard_info(standard: &NtfsStandardInformation, file_info: &mut RawFilelist) {
    file_info.created = unixepoch_to_iso(&filetime_to_unixepoch(
        &standard.creation_time().nt_timestamp(),
    ));
    file_info.modified = unixepoch_to_iso(&filetime_to_unixepoch(
        &standard.modification_time().nt_timestamp(),
    ));
    file_info.changed = unixepoch_to_iso(&filetime_to_unixepoch(
        &standard.mft_record_modification_time().nt_timestamp(),
    ));
    file_info.accessed = unixepoch_to_iso(&filetime_to_unixepoch(
        &standard.access_time().nt_timestamp(),
    ));

    file_info.usn = standard.usn().unwrap_or(0);
    file_info.sid = standard.security_id().unwrap_or(0);
    file_info.owner = standard.owner_id().unwrap_or(0);

    file_info.sid = standard.security_id().unwrap_or(0);

    let attributes: Vec<String> = standard
        .file_attributes()
        .iter_names()
        .map(|(s, _)| s.to_string())
        .collect();
    file_info.attributes = attributes;

    if file_info.attributes.contains(&String::from("COMPRESSED")) {
        file_info.compression_type = CompressionType::NTFSCompressed;
    }
}

/// Get $DATA attribute data size and hash the data (if enabled)
pub(crate) fn file_data(
    ntfs_file: &NtfsFile<'_>,
    ntfs_ref: &NtfsFileReference,
    file_info: &mut RawFilelist,
    fs: &mut BufReader<SectorReader<File>>,
    ntfs: &Ntfs,
    hashes: &Hashes,
) -> Result<(), NTFSError> {
    let check_results = check_wofcompressed(ntfs_ref, ntfs, fs);
    let (is_compressed, uncompressed_data, compressed_size) = match check_results {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[ntfs] Could not check for wofcompression. Returning default data. Error: {err:?}"
            );
            (false, Vec::new(), 0)
        }
    };

    if is_compressed {
        file_info.size = uncompressed_data.len() as u64;
        file_info.compressed_size = compressed_size;
        file_info.compression_type = CompressionType::WofCompressed;

        // If not doing any hashing, just return now
        if !hashes.md5 && !hashes.sha1 && !hashes.sha256 {
            return Ok(());
        }
        (file_info.md5, file_info.sha1, file_info.sha256) =
            hash_file_data(hashes, &uncompressed_data);
        return Ok(());
    }

    let data_name = "";
    let ntfs_data_option = ntfs_file.data(fs, data_name);
    let ntfs_data_result = match ntfs_data_option {
        Some(result) => result,
        None => return Err(NTFSError::FileData), // Some files do not have data stored under "". Ex: $UsnJrnl data is stored under "$J"
    };

    let ntfs_data = match ntfs_data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ntfs] Failed to get NTFS data error: {err:?}");
            return Err(NTFSError::FileData);
        }
    };

    let ntfs_attribute_result = ntfs_data.to_attribute();
    let ntfs_attribute = match ntfs_attribute_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Failed to get NTFS attribute error: {err:?}");
            return Err(NTFSError::NoAttribute);
        }
    };

    file_info.size = ntfs_attribute.value_length();

    // If not doing any hashing, just return now
    if !hashes.md5 && !hashes.sha1 && !hashes.sha256 {
        return Ok(());
    }

    let data_result = ntfs_attribute.value(fs);
    let mut data_attr_value = match data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[ntfs] Failed to get NTFS attribute data error: {err:?}");
            return Err(NTFSError::AttributeValue);
        }
    };

    (file_info.md5, file_info.sha1, file_info.sha256) =
        raw_hash_data(&mut data_attr_value, fs, hashes);

    Ok(())
}

/// Get the name of an Attribute
pub(crate) fn get_attribute_name(attribute: &NtfsAttribute<'_, '_>) -> String {
    let attr_name_result = attribute.name();

    match attr_name_result {
        Ok(result) => result.to_string().unwrap_or_default(),
        Err(err) => {
            error!("[ntfs] Failed to get INDX attribute name: {err:?}");
            String::new()
        }
    }
}

/// Get the type of an Attribute
pub(crate) fn get_attribute_type(attribute: &NtfsAttribute<'_, '_>) -> String {
    let attr_type_result = attribute.ty();

    match attr_type_result {
        Ok(result) => result.to_string(),
        Err(err) => {
            error!("[ntfs] Failed to get INDX attribute type: {err:?}");
            String::new()
        }
    }
}

/// Get all alternative data streams (ADS) for a file
pub(crate) fn get_ads_names(
    ntfs_ref: &NtfsFileReference,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<Vec<ADSInfo>, NtfsError> {
    let ntfs_file = ntfs_ref.to_file(ntfs, fs)?;
    let attr_raw = ntfs_file.attributes_raw();

    let mut ads = Vec::new();
    // Loop through the raw attributes looking for ADS names
    for attrs in attr_raw {
        let attr = attrs?;
        if attr.ty()? != NtfsAttributeType::AttributeList && attr.ty()? != NtfsAttributeType::Data {
            continue;
        }

        /*
         * If there are a lot of attributes or attributes take up alot of space
         * `NTFS` will create a new MFT record and create an `AttributeList` to track all the attributes for the file
         */
        if attr.ty()? == NtfsAttributeType::AttributeList {
            let list = attr.structured_value::<_, NtfsAttributeList<'_, '_>>(fs)?;
            let mut list_iter = list.entries();
            // Walk the attributelist
            while let Some(entry) = list_iter.next(fs) {
                let entry = entry?;

                let temp_file = entry.to_file(ntfs, fs)?;
                let entry_attr = entry.to_attribute(&temp_file)?;

                let attr_name = entry_attr.name()?;
                let mut ads_info = ADSInfo {
                    name: String::new(),
                    size: 0,
                };
                ads_info.name = attr_name.to_string_lossy();
                ads_info.size = entry_attr.value_length();

                if ads_info.name.is_empty() {
                    continue;
                }
                ads.push(ads_info);
            }
            continue;
        }

        let mut ads_info = ADSInfo {
            name: String::new(),
            size: 0,
        };
        let attr_name = attr.name()?;
        ads_info.name = attr_name.to_string_lossy();
        ads_info.size = attr.value_length();

        if ads_info.name.is_empty() {
            continue;
        }
        ads.push(ads_info);
    }
    Ok(ads)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::RawFilelist;
    use crate::{
        artifacts::os::windows::ntfs::attributes::{
            file_data, filename_info, get_ads_names, get_attribute_name, get_attribute_type,
            standard_info,
        },
        filesystem::ntfs::{sector_reader::SectorReader, setup::setup_ntfs_parser},
        structs::artifacts::os::windows::RawFilesOptions,
    };
    use common::files::Hashes;
    use common::windows::CompressionType;
    use ntfs::Ntfs;
    use std::{fs::File, io::BufReader, path::PathBuf};

    #[test]
    fn test_filename_info() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::new(),
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
        let directory_tracker: Vec<String> =
            vec![String::from(format!("{}:", test_path.drive_letter))];

        let index = root_dir.directory_index(&mut fs).unwrap();
        let mut iter = index.entries();
        let root_index = 1;
        while let Some(entry) = iter.next(&mut fs) {
            let mut file_info = RawFilelist {
                full_path: String::new(),
                directory: String::new(),
                filename: String::new(),
                extension: String::new(),
                created: String::new(),
                modified: String::new(),
                changed: String::new(),
                accessed: String::new(),
                filename_created: String::new(),
                filename_modified: String::new(),
                filename_changed: String::new(),
                filename_accessed: String::new(),
                size: 0,
                inode: 0,
                sequence_number: 0,
                parent_mft_reference: 0,
                is_indx: false,
                owner: 0,
                attributes: Vec::new(),
                md5: String::new(),
                sha1: String::new(),
                sha256: String::new(),
                is_file: false,
                is_directory: false,
                depth: directory_tracker.len() - root_index, // Subtract root index (C:\)
                usn: 0,
                sid: 0,
                user_sid: String::new(),
                group_sid: String::new(),
                drive: directory_tracker[0].to_owned(),
                compressed_size: 0,
                compression_type: CompressionType::None,
                ads_info: Vec::new(),
                pe_info: Vec::new(),
            };

            let entry_index = entry.unwrap();
            let filename_result = entry_index.key().unwrap();

            let result = filename_info(&filename_result, &mut file_info).unwrap();
            assert_eq!(result, ());

            assert!(file_info.filename.is_empty() == false);
            break;
        }
    }

    #[test]
    fn test_standard_info() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::new(),
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
        let directory_tracker: Vec<String> =
            vec![String::from(format!("{}:", test_path.drive_letter))];

        let index = root_dir.directory_index(&mut fs).unwrap();
        let mut iter = index.entries();
        let root_index = 1;
        while let Some(entry) = iter.next(&mut fs) {
            let mut file_info = RawFilelist {
                full_path: String::new(),
                directory: String::new(),
                filename: String::new(),
                extension: String::new(),
                created: String::new(),
                modified: String::new(),
                changed: String::new(),
                accessed: String::new(),
                filename_created: String::new(),
                filename_modified: String::new(),
                filename_changed: String::new(),
                filename_accessed: String::new(),
                size: 0,
                inode: 0,
                sequence_number: 0,
                owner: 0,
                parent_mft_reference: 0,
                is_indx: false,
                attributes: Vec::new(),
                md5: String::new(),
                sha1: String::new(),
                sha256: String::new(),
                is_file: false,
                is_directory: false,
                depth: directory_tracker.len() - root_index, // Subtract root index (C:\)
                usn: 0,
                sid: 0,
                user_sid: String::new(),
                group_sid: String::new(),
                drive: directory_tracker[0].to_owned(),
                compressed_size: 0,
                compression_type: CompressionType::None,
                ads_info: Vec::new(),
                pe_info: Vec::new(),
            };

            let entry_index = entry.unwrap();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();
            let result = standard_info(&ntfs_file.info().unwrap(), &mut file_info);

            assert_eq!(result, ());
            assert!(file_info.created != "");
            assert!(file_info.modified != "");
            assert!(file_info.accessed != "");
            assert!(file_info.changed != "");
            break;
        }
    }

    #[test]
    fn test_file_data() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::new(),
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
        let mut ntfs_parser = setup_ntfs_parser(&test_path.drive_letter).unwrap();

        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();
        let directory_tracker: Vec<String> =
            vec![String::from(format!("{}:", test_path.drive_letter))];

        let index = root_dir.directory_index(&mut fs).unwrap();
        let mut iter = index.entries();
        let root_index = 1;
        let hash_data = Hashes {
            md5: true,
            sha1: false,
            sha256: false,
        };
        while let Some(entry) = iter.next(&mut fs) {
            let mut file_info = RawFilelist {
                full_path: String::new(),
                directory: String::new(),
                filename: String::new(),
                extension: String::new(),
                created: String::new(),
                modified: String::new(),
                changed: String::new(),
                accessed: String::new(),
                filename_created: String::new(),
                filename_modified: String::new(),
                filename_changed: String::new(),
                filename_accessed: String::new(),
                size: 0,
                parent_mft_reference: 0,
                is_indx: false,
                inode: 0,
                sequence_number: 0,
                owner: 0,
                attributes: Vec::new(),
                md5: String::new(),
                sha1: String::new(),
                sha256: String::new(),
                is_file: false,
                is_directory: false,
                depth: directory_tracker.len() - root_index, // Substract root index (C:\)
                usn: 0,
                sid: 0,
                user_sid: String::new(),
                group_sid: String::new(),
                drive: directory_tracker[0].to_owned(),
                compressed_size: 0,
                compression_type: CompressionType::None,
                ads_info: Vec::new(),
                pe_info: Vec::new(),
            };

            let entry_index = entry.unwrap();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();
            if !ntfs_file.is_directory() {
                let result = file_data(
                    &ntfs_file,
                    &entry_index.file_reference(),
                    &mut file_info,
                    &mut ntfs_parser.fs,
                    &ntfs_parser.ntfs,
                    &hash_data,
                )
                .unwrap();
                assert_eq!(result, ());
                assert_eq!(file_info.md5.is_empty(), false);
                break;
            }
        }
    }

    #[test]
    fn test_get_ads_names() {
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::new(),
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
        let mut ntfs_parser = setup_ntfs_parser(&test_path.drive_letter).unwrap();

        let fs = File::open(drive_path).unwrap();

        let reader_sector_size = 4096;
        let sector_reader = SectorReader::new(fs, reader_sector_size).unwrap();
        let mut fs = BufReader::new(sector_reader);
        let ntfs = Ntfs::new(&mut fs).unwrap();
        let root_dir = ntfs.root_directory(&mut fs).unwrap();

        let index = root_dir.directory_index(&mut fs).unwrap();
        let mut iter = index.entries();

        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();
            let filename = entry_index.key().unwrap().unwrap().name().to_string_lossy();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();
            if !ntfs_file.is_directory() && filename == "$UsnJrnl" {
                let result = get_ads_names(
                    &entry_index.file_reference(),
                    &ntfs_parser.ntfs,
                    &mut ntfs_parser.fs,
                )
                .unwrap();
                assert_eq!(result.len(), 2);
                assert_eq!(result[0].name, "$J");
                assert_eq!(result[0].name, "$MAX");

                break;
            }
        }
    }

    #[test]
    fn test_get_attribute_name() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files");
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: test_location.display().to_string(),
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
        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();

            if !ntfs_file.is_directory() {
                let ntfs_data = ntfs_file.data(&mut fs, "").unwrap().unwrap();
                let ntfs_attribute = ntfs_data.to_attribute().unwrap();

                let name = get_attribute_name(&ntfs_attribute);
                assert_eq!(name, "");

                break;
            }
        }
    }

    #[test]
    fn test_get_attribute_type() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files");
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: test_location.display().to_string(),
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
        while let Some(entry) = iter.next(&mut fs) {
            let entry_index = entry.unwrap();

            let ntfs_file = entry_index
                .file_reference()
                .to_file(&ntfs, &mut fs)
                .unwrap();

            if !ntfs_file.is_directory() {
                let ntfs_data = ntfs_file.data(&mut fs, "").unwrap().unwrap();
                let ntfs_attribute = ntfs_data.to_attribute().unwrap();

                let attr_type = get_attribute_type(&ntfs_attribute);
                assert_eq!(attr_type, "Data");
                break;
            }
        }
    }
}
