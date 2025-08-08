use super::error::NTFSError;
use crate::{
    filesystem::{
        files::hash_file_data,
        ntfs::{
            attributes::{get_filename_attribute, read_attribute_data},
            compression::check_wofcompressed,
            raw_files::raw_hash_data,
            sector_reader::SectorReader,
        },
    },
    utils::{
        nom_helper::{Endian, nom_unsigned_four_bytes},
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use common::files::Hashes;
use common::windows::{ADSInfo, CompressionType, RawFilelist};
use log::error;
use ntfs::{
    Ntfs, NtfsAttribute, NtfsAttributeType, NtfsError, NtfsFile, NtfsFileReference,
    structured_values::{
        NtfsAttributeList, NtfsFileName, NtfsFileNamespace, NtfsStandardInformation,
    },
};
use serde::Serialize;
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
    file_info.filename_created = unixepoch_to_iso(filetime_to_unixepoch(
        filename.creation_time().nt_timestamp(),
    ));
    file_info.filename_modified = unixepoch_to_iso(filetime_to_unixepoch(
        filename.modification_time().nt_timestamp(),
    ));
    file_info.filename_changed = unixepoch_to_iso(filetime_to_unixepoch(
        filename.mft_record_modification_time().nt_timestamp(),
    ));
    file_info.filename_accessed =
        unixepoch_to_iso(filetime_to_unixepoch(filename.access_time().nt_timestamp()));
    Ok(())
}

/// Get Standard attributes data
pub(crate) fn standard_info(standard: &NtfsStandardInformation, file_info: &mut RawFilelist) {
    file_info.created = unixepoch_to_iso(filetime_to_unixepoch(
        standard.creation_time().nt_timestamp(),
    ));
    file_info.modified = unixepoch_to_iso(filetime_to_unixepoch(
        standard.modification_time().nt_timestamp(),
    ));
    file_info.changed = unixepoch_to_iso(filetime_to_unixepoch(
        standard.mft_record_modification_time().nt_timestamp(),
    ));
    file_info.accessed =
        unixepoch_to_iso(filetime_to_unixepoch(standard.access_time().nt_timestamp()));

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
    ntfs_ref: NtfsFileReference,
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
            error!("[forensics] Failed to get NTFS attribute error: {err:?}");
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

#[derive(Serialize, Debug, PartialEq)]
pub(crate) enum ReparseType {
    Reserved,
    ReservedOne,
    ReservedTwo,
    MountPoint,
    HierarchicalStorageManagement,
    DriveExtender,
    HierarchicalStorageManagement2,
    SingleInstanceStorage,
    Wim,
    ClusteredSharedVolume,
    DistributedFileSystem,
    FilterManager,
    SymbolicLink,
    IisCache,
    DistributedFileSystemReplication,
    Dedup,
    Appxstrm,
    NetworkFileSystem,
    FilePlaceholder,
    DynamicFilter,
    Wof,
    WindowsContainerIsolation,
    WindowsContainerIsolation1,
    GlobalReparse,
    Cloud,
    Cloud1,
    Cloud2,
    Cloud3,
    Cloud4,
    Cloud5,
    Cloud6,
    Cloud7,
    Cloud8,
    Cloud9,
    CloudA,
    CloudB,
    CloudC,
    CloudD,
    CloudE,
    CloudF,
    AppExecLink,
    ProjectedFileSystem,
    LinuxSymbolicLink,
    StorageSync,
    StorageSyncFolder,
    WindowsContainerTombstone,
    Unhandled,
    Onedrive,
    ProjectFileSystemTombstone,
    AfUnix,
    LinuxFifo,
    LinuxChar,
    LinuxBlock,
    LinuxLink,
    LinuxLink1,
    Unknown,
}

/// Get the Reparse Point type
pub(crate) fn get_reparse_type(
    ntfs_ref: NtfsFileReference,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<ReparseType, NtfsError> {
    let ntfs_file = ntfs_ref.to_file(ntfs, fs)?;
    let attr_raw = ntfs_file.attributes_raw();

    let mut reparse_data = Vec::new();
    // Loop through the raw attributes looking for REPARSE_POINT
    for attrs in attr_raw {
        let attr = attrs?;
        if attr.ty()? != NtfsAttributeType::AttributeList
            && attr.ty()? != NtfsAttributeType::ReparsePoint
        {
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
                if entry.ty()? != NtfsAttributeType::ReparsePoint {
                    continue;
                }

                let temp_file = entry.to_file(ntfs, fs)?;
                let entry_attr = entry.to_attribute(&temp_file)?;
                let mut value = entry_attr.value(fs)?;

                reparse_data = read_attribute_data(&mut value, fs, &entry_attr)?;
            }
            break;
        }
        let mut value = attr.value(fs)?;
        reparse_data = read_attribute_data(&mut value, fs, &attr)?;
        break;
    }

    let min_size = 4;
    if reparse_data.len() < min_size {
        return Ok(ReparseType::Unknown);
    }
    // First four (4) bytes contain the Reparse Tag
    // https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/c8e77b37-3909-4fe6-a4ea-2b9d423b1ee4
    let tag = match nom_unsigned_four_bytes(&reparse_data, Endian::Le) {
        Ok((_, result)) => result,
        Err(_err) => return Ok(ReparseType::Unknown),
    };

    Ok(reparse_type(tag))
}

/// Determine Reparse Type
fn reparse_type(tag: u32) -> ReparseType {
    match tag {
        0x00000000 => ReparseType::Reserved,
        0x00000001 => ReparseType::ReservedOne,
        0x00000002 => ReparseType::ReservedTwo,
        0xA0000003 => ReparseType::MountPoint,
        0xC0000004 => ReparseType::HierarchicalStorageManagement,
        0x80000005 => ReparseType::DriveExtender,
        0x80000006 => ReparseType::HierarchicalStorageManagement2,
        0x80000007 => ReparseType::SingleInstanceStorage,
        0x80000008 => ReparseType::Wim,
        0x80000009 => ReparseType::ClusteredSharedVolume,
        0x8000000A => ReparseType::DistributedFileSystem,
        0x8000000B => ReparseType::FilterManager,
        0xA000000C => ReparseType::SymbolicLink,
        0xA0000010 => ReparseType::IisCache,
        0x80000012 => ReparseType::DistributedFileSystemReplication,
        0x80000013 => ReparseType::Dedup,
        0xC0000014 => ReparseType::Appxstrm,
        0x80000014 => ReparseType::NetworkFileSystem,
        0x80000015 => ReparseType::FilePlaceholder,
        0x80000016 => ReparseType::DynamicFilter,
        0x80000017 => ReparseType::Wof,
        0x80000018 => ReparseType::WindowsContainerIsolation,
        0x90001018 => ReparseType::WindowsContainerIsolation1,
        0xA0000019 => ReparseType::GlobalReparse,
        0x9000001A => ReparseType::Cloud,
        0x9000101A => ReparseType::Cloud1,
        0x9000201A => ReparseType::Cloud2,
        0x9000301A => ReparseType::Cloud3,
        0x9000401A => ReparseType::Cloud4,
        0x9000501A => ReparseType::Cloud5,
        0x9000601A => ReparseType::Cloud6,
        0x9000701A => ReparseType::Cloud7,
        0x9000801A => ReparseType::Cloud8,
        0x9000901A => ReparseType::Cloud9,
        0x9000A01A => ReparseType::CloudA,
        0x9000B01A => ReparseType::CloudB,
        0x9000C01A => ReparseType::CloudC,
        0x9000D01A => ReparseType::CloudD,
        0x9000E01A => ReparseType::CloudE,
        0x9000F01A => ReparseType::CloudF,
        0x8000001B => ReparseType::AppExecLink,
        0x9000001C => ReparseType::ProjectedFileSystem,
        0xA000001D => ReparseType::LinuxSymbolicLink,
        0x8000001E => ReparseType::StorageSync,
        0x90000027 => ReparseType::StorageSyncFolder,
        0xA000001F => ReparseType::WindowsContainerTombstone,
        0x80000020 => ReparseType::Unhandled,
        0x80000021 => ReparseType::Onedrive,
        0xA0000022 => ReparseType::ProjectFileSystemTombstone,
        0x80000023 => ReparseType::AfUnix,
        0x80000024 => ReparseType::LinuxFifo,
        0x80000025 => ReparseType::LinuxChar,
        0x80000026 => ReparseType::LinuxBlock,
        0xA0000027 => ReparseType::LinuxLink,
        0xA0001027 => ReparseType::LinuxLink1,
        _ => ReparseType::Unknown,
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
    ntfs_ref: NtfsFileReference,
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
        artifacts::os::windows::ntfs::{
            attributes::{
                ReparseType, file_data, filename_info, get_ads_names, get_attribute_name,
                get_attribute_type, reparse_type, standard_info,
            },
            parser::ntfs_filelist,
        },
        filesystem::ntfs::{sector_reader::SectorReader, setup::setup_ntfs_parser},
        structs::{artifacts::os::windows::RawFilesOptions, toml::Output},
    };
    use common::files::Hashes;
    use common::windows::CompressionType;
    use ntfs::Ntfs;
    use std::{fs::File, io::BufReader, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

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
        let mut ntfs_parser = setup_ntfs_parser(test_path.drive_letter).unwrap();

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
                    entry_index.file_reference(),
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
        let mut ntfs_parser = setup_ntfs_parser(test_path.drive_letter).unwrap();

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
                    entry_index.file_reference(),
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
    fn test_get_reparse_type() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files");
        let test_path = RawFilesOptions {
            drive_letter: 'C',
            start_path: String::from("C:\\Document and Settings"),
            depth: 2,
            recover_indx: false,
            md5: None,
            sha1: None,
            sha256: None,
            metadata: None,
            path_regex: None,
            filename_regex: None,
        };

        let mut output = output_options("rawfiles_temp", "local", "./tmp", false);

        let result = ntfs_filelist(&test_path, &mut output, false).unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_reparse_type() {
        let test = [
            0x00000000, 0x00000001, 0x00000002, 0xA0000003, 0xC0000004, 0x80000005, 0x80000006,
            0x80000007, 0x80000008, 0x80000009, 0x8000000A, 0x8000000B, 0xA000000C, 0xA0000010,
            0x80000012, 0x80000013, 0xC0000014, 0x80000014, 0x80000015, 0x80000016, 0x80000017,
            0x80000018, 0x90001018, 0xA0000019, 0x9000001A, 0x9000101A, 0x9000201A, 0x9000301A,
            0x9000401A, 0x9000501A, 0x9000601A, 0x9000701A, 0x9000801A, 0x9000901A, 0x9000A01A,
            0x9000B01A, 0x9000C01A, 0x9000D01A, 0x9000E01A, 0x9000F01A, 0x8000001B, 0x9000001C,
            0xA000001D, 0x8000001E, 0x90000027, 0xA000001F, 0x80000020, 0x80000021, 0xA0000022,
            0x80000023, 0x80000024, 0x80000025, 0x80000026, 0xA0000027, 0xA0001027,
        ];
        for entry in test {
            assert_ne!(reparse_type(entry), ReparseType::Unknown);
        }
        assert_eq!(reparse_type(0xff), ReparseType::Unknown);
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
