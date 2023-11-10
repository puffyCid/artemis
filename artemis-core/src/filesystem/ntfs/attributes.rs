use super::{raw_files::raw_read_data, sector_reader::SectorReader};
use crate::filesystem::error::FileSystemError;
use common::windows::AttributeFlags;
use log::{error, warn};
use ntfs::{
    attribute_value::NtfsAttributeValue,
    structured_values::{NtfsAttributeList, NtfsFileName},
    Ntfs, NtfsAttribute, NtfsAttributeType, NtfsError, NtfsFileReference, NtfsReadSeek,
};
use std::{fs::File, io::BufReader};

/// Return FILENAME attribute data
pub(crate) fn get_filename_attribute(
    filename_result: &Result<NtfsFileName, NtfsError>,
) -> Result<NtfsFileName, FileSystemError> {
    match filename_result {
        Ok(result) => Ok(result.clone()),
        Err(err) => {
            error!("[artemis-core] Failed to get filename info, error: {err:?}");
            Err(FileSystemError::NoFilenameAttr)
        }
    }
}

/// Get attribute data by walking the attribute list until we find our attribute or reading the attribute directly. Returns a vec data from the data runs
pub(crate) fn get_attribute_data(
    ntfs_ref: &NtfsFileReference,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
    attribute: &str,
) -> Result<Vec<u8>, NtfsError> {
    let ntfs_file = ntfs_ref.to_file(ntfs, fs)?;
    let attr_raw = ntfs_file.attributes_raw();

    let mut attr_data = Vec::new();
    // Loop through the raw attributes looking for user provided attribute name
    for attrs in attr_raw {
        let attr = attrs?;
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
                if attr_name.to_string_lossy() != attribute {
                    continue;
                }

                let mut value = entry_attr.value(fs)?;
                let all_data = read_attribute_data(&mut value, fs, &entry_attr)?;
                if all_data.is_empty() {
                    continue;
                }
                attr_data = all_data;
            }
        } else if attr.ty()? == NtfsAttributeType::Data {
            let attr_name = attr.name()?;
            if attr_name.to_string_lossy() != attribute {
                continue;
            }
            let mut value = attr.value(fs)?;

            let all_data = read_attribute_data(&mut value, fs, &attr)?;
            if all_data.is_empty() {
                continue;
            }
            attr_data = all_data;
        }
    }
    Ok(attr_data)
}

/// Read the attribute data. Handles both resident and non-resident data.
fn read_attribute_data(
    value: &mut NtfsAttributeValue<'_, '_>,
    fs: &mut BufReader<SectorReader<File>>,
    entry_attr: &NtfsAttribute<'_, '_>,
) -> Result<Vec<u8>, NtfsError> {
    let mut all_data = Vec::new();
    // If attribute data is resident, just read the all the data. Resident data is very small
    if entry_attr.is_resident() {
        let mut resident_data = raw_read_data(value, fs)?;
        all_data.append(&mut resident_data);
        return Ok(all_data);
    }

    // Grab non-resident attribute data
    if let NtfsAttributeValue::NonResident(non_resident) = value {
        let attribute_data_len = entry_attr.value_length() as usize;

        // Set a max size of 2GBs. Currently will not read more than 2GBs of attribute data
        let max_size = 2147483648;
        // Walkthrough the data runs
        for (_, data_run) in non_resident.data_runs().enumerate() {
            let mut temp_run = data_run?;

            // We only want the non-sparse data runs
            if temp_run.data_position() == None.into() {
                continue;
            }

            let mut buff_data: Vec<u8> = Vec::new();
            loop {
                let temp_buff_size = 65536;
                let mut temp_buff: Vec<u8> = vec![0u8; temp_buff_size];
                let bytes = temp_run.read(fs, &mut temp_buff)?;

                let finished = 0;
                if bytes == finished {
                    break;
                }

                // Make sure our temp buff does not any have extra zeros from the intialization
                if bytes < temp_buff_size {
                    buff_data.append(&mut temp_buff[0..bytes].to_vec());
                } else {
                    buff_data.append(&mut temp_buff);
                }
            }
            all_data.append(&mut buff_data);
            if all_data.len() >= max_size {
                warn!("[artemis-core] Currently 2GBs or more data. Exiting early");
                break;
            }
        }

        let empty_data = 0;
        // Verfiy our final data equals the expected attribute length (if not zero (0))
        if all_data.len() > attribute_data_len && attribute_data_len != empty_data {
            all_data = all_data[0..attribute_data_len].to_vec();
        }
    }
    Ok(all_data)
}

/// Determine attribute flags of a file
pub(crate) fn file_attribute_flags(attributes: &u32) -> Vec<AttributeFlags> {
    let mut attributes_vec: Vec<AttributeFlags> = Vec::new();
    let readonly = 0x1;
    let hidden = 0x2;
    let system = 0x4;
    let directory = 0x10;
    let archive = 0x20;
    let device = 0x40;
    let normal = 0x80;
    let temp = 0x100;
    let sparse = 0x200;
    let reparse = 0x400;
    let compressed = 0x800;
    let offline = 0x1000;
    let indexed = 0x2000;
    let encrypted = 0x4000;
    let virtual_attr = 0x10000;

    if (attributes & readonly) == readonly {
        attributes_vec.push(AttributeFlags::ReadOnly);
    }
    if (attributes & hidden) == hidden {
        attributes_vec.push(AttributeFlags::Hidden);
    }
    if (attributes & system) == system {
        attributes_vec.push(AttributeFlags::System);
    }
    if (attributes & directory) == directory {
        attributes_vec.push(AttributeFlags::Directory);
    }
    if (attributes & archive) == archive {
        attributes_vec.push(AttributeFlags::Archive);
    }
    if (attributes & device) == device {
        attributes_vec.push(AttributeFlags::Device);
    }
    if (attributes & normal) == normal {
        attributes_vec.push(AttributeFlags::Normal);
    }
    if (attributes & temp) == temp {
        attributes_vec.push(AttributeFlags::Temporary);
    }
    if (attributes & sparse) == sparse {
        attributes_vec.push(AttributeFlags::SparseFile);
    }
    if (attributes & reparse) == reparse {
        attributes_vec.push(AttributeFlags::ReparsePoint);
    }
    if (attributes & compressed) == compressed {
        attributes_vec.push(AttributeFlags::Compressed);
    }
    if (attributes & offline) == offline {
        attributes_vec.push(AttributeFlags::Offline);
    }
    if (attributes & indexed) == indexed {
        attributes_vec.push(AttributeFlags::NotConentIndexed);
    }
    if (attributes & encrypted) == encrypted {
        attributes_vec.push(AttributeFlags::Encrypted);
    }
    if (attributes & virtual_attr) == virtual_attr {
        attributes_vec.push(AttributeFlags::Virtual);
    }

    attributes_vec
}

#[cfg(test)]
mod tests {
    use crate::{
        filesystem::ntfs::{
            attributes::{
                file_attribute_flags, get_filename_attribute, read_attribute_data, AttributeFlags,
            },
            raw_files::{iterate_ntfs, NtfsOptions},
            sector_reader::SectorReader,
            setup::setup_ntfs_parser,
        },
        utils::regex_options::create_regex,
    };
    use ntfs::{structured_values::NtfsAttributeList, Ntfs, NtfsAttributeType};
    use std::{fs::File, io::BufReader};

    use super::get_attribute_data;

    #[test]
    fn test_get_filename_attribute() {
        let drive_path = "\\\\.\\C:";
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
            let filename_result = entry_index.key().unwrap();

            let result = get_filename_attribute(&filename_result).unwrap();
            assert_eq!(result.name().is_empty(), false);

            break;
        }
    }

    #[test]
    fn test_file_attribute_flags() {
        let test = 1;
        let flag = file_attribute_flags(&test);
        assert_eq!(flag.len(), 1);
        assert_eq!(flag[0], AttributeFlags::ReadOnly)
    }

    #[test]
    fn test_get_attribute_data() {
        let drive = 'C';
        let mut ntfs_parser = setup_ntfs_parser(&drive).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let mut ntfs_options = NtfsOptions {
            start_path: String::from("C:\\$Extend\\$UsnJrnl"),
            start_path_depth: 0,
            depth: String::from("C:\\$Extend\\$UsnJrnl").split('\\').count(),
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            directory_tracker: vec![format!("{drive}:")],
        };

        // Search and iterate through the NTFS system for the file
        let _ = iterate_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut ntfs_options,
        );

        for filelist in ntfs_options.filelist {
            if filelist.full_path != "C:\\$Extend\\$UsnJrnl" {
                continue;
            }

            // $MAX attribute is 32 bytes should be resident data
            let data = get_attribute_data(
                &filelist.file,
                &ntfs_parser.ntfs,
                &mut ntfs_parser.fs,
                "$Max",
            )
            .unwrap();
            assert_eq!(data.len(), 32);
            break;
        }
    }

    #[test]
    fn test_read_attribute_data() {
        let drive = 'C';
        let mut ntfs_parser = setup_ntfs_parser(&drive).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let mut ntfs_options = NtfsOptions {
            start_path: String::from("C:\\$Extend\\$UsnJrnl"),
            start_path_depth: 0,
            depth: String::from("C:\\$Extend\\$UsnJrnl").split('\\').count(),
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            directory_tracker: vec![format!("{drive}:")],
        };

        let _ = iterate_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut ntfs_options,
        );

        for filelist in ntfs_options.filelist {
            if filelist.full_path != "C:\\$Extend\\$UsnJrnl" {
                continue;
            }

            // $MAX attribute is 32 bytes should be resident data

            let ntfs_file = filelist
                .file
                .to_file(&ntfs_parser.ntfs, &mut ntfs_parser.fs)
                .unwrap();

            let attr_raw = ntfs_file.attributes_raw();

            for attrs in attr_raw {
                let attr = attrs.unwrap();
                if attr.ty().unwrap() == NtfsAttributeType::AttributeList {
                    let list = attr
                        .structured_value::<_, NtfsAttributeList<'_, '_>>(&mut ntfs_parser.fs)
                        .unwrap();
                    let mut list_iter = list.entries();
                    while let Some(entry) = list_iter.next(&mut ntfs_parser.fs) {
                        let entry = entry.unwrap();

                        let temp_file = entry
                            .to_file(&ntfs_parser.ntfs, &mut ntfs_parser.fs)
                            .unwrap();
                        let entry_attr = entry.to_attribute(&temp_file).unwrap();

                        let attr_name = entry_attr.name().unwrap();
                        if attr_name.to_string_lossy() != "$Max" {
                            continue;
                        }

                        let mut value = entry_attr.value(&mut ntfs_parser.fs).unwrap();

                        let data =
                            read_attribute_data(&mut value, &mut ntfs_parser.fs, &attr).unwrap();
                        assert_eq!(data.len(), 32);
                        break;
                    }
                } else if attr.ty().unwrap() == NtfsAttributeType::Data {
                    let attr_name = attr.name().unwrap();
                    if attr_name.to_string_lossy() != "$Max" {
                        continue;
                    }
                    let mut value = attr.value(&mut ntfs_parser.fs).unwrap();

                    let data = read_attribute_data(&mut value, &mut ntfs_parser.fs, &attr).unwrap();
                    assert_eq!(data.len(), 32);
                    break;
                }
            }
        }
    }
}
