use super::{raw_files::raw_read_data, sector_reader::SectorReader};
use crate::filesystem::error::FileSystemError;
use common::windows::AttributeFlags;
use log::{error, warn};
use ntfs::{
    Ntfs, NtfsAttribute, NtfsAttributeType, NtfsError, NtfsFile, NtfsReadSeek,
    attribute_value::NtfsAttributeValue,
    structured_values::{NtfsAttributeList, NtfsFileName},
};
use std::{
    fs::File,
    io::{BufReader, Error, ErrorKind},
};

/// Return FILENAME attribute data
pub(crate) fn get_filename_attribute(
    filename_result: &Result<NtfsFileName, NtfsError>,
) -> Result<NtfsFileName, FileSystemError> {
    match filename_result {
        Ok(result) => Ok(result.clone()),
        Err(err) => {
            error!("[forensics] Failed to get filename info, error: {err:?}");
            Err(FileSystemError::NoFilenameAttr)
        }
    }
}

/// Get attribute data by walking the attribute list until we find our attribute or reading the attribute directly. Returns a vec data from the data runs
pub(crate) fn get_attribute_data(
    //ntfs_ref: NtfsFileReference,
    ntfs_file: &NtfsFile<'_>,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
    attribute: &str,
) -> Result<Vec<u8>, NtfsError> {
    //let ntfs_file = ntfs_ref.to_file(ntfs, fs)?;
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

/// Get the size of a file by parsing the NTFS filesystem
pub(crate) fn get_raw_file_size(
    ntfs: &NtfsFile<'_>,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<u64, NtfsError> {
    let attrib = match ntfs.data(fs, "") {
        Some(result) => result,
        None => {
            return Err(NtfsError::Io(Error::new(
                ErrorKind::InvalidData,
                "Could determine file size",
            )));
        }
    };
    Ok(attrib?.to_attribute()?.value_length())
}

/// Read the attribute data. Handles both resident and non-resident data.
pub(crate) fn read_attribute_data(
    value: &mut NtfsAttributeValue<'_, '_>,
    fs: &mut BufReader<SectorReader<File>>,
    entry_attr: &NtfsAttribute<'_, '_>,
) -> Result<Vec<u8>, NtfsError> {
    // If attribute data is resident, just read the all the data. Resident data is very small
    if entry_attr.is_resident() {
        let resident_data = raw_read_data(value, fs)?;
        return Ok(resident_data);
    }

    let mut all_data = Vec::new();
    // Grab non-resident attribute data
    if let NtfsAttributeValue::NonResident(non_resident) = value {
        let attribute_data_len = entry_attr.value_length() as usize;

        // Set a max size of 2GBs. Currently will not read more than 2GBs of attribute data
        let max_size = 2147483648;
        // Walkthrough the data runs
        for data_run in non_resident.data_runs() {
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
                warn!("[forensics] Currently 2GBs or more data. Exiting early");
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
pub(crate) fn file_attribute_flags(data: u32) -> Vec<AttributeFlags> {
    let mut attrs = Vec::new();

    if (data & 0x1) == 0x1 {
        attrs.push(AttributeFlags::ReadOnly);
    }
    if (data & 0x2) == 0x2 {
        attrs.push(AttributeFlags::Hidden);
    }
    if (data & 0x4) == 0x4 {
        attrs.push(AttributeFlags::System);
    }
    if (data & 0x8) == 0x8 {
        attrs.push(AttributeFlags::Volume);
    }
    if (data & 0x10) == 0x10 {
        attrs.push(AttributeFlags::Directory);
    }
    if (data & 0x20) == 0x20 {
        attrs.push(AttributeFlags::Archive);
    }
    if (data & 0x40) == 0x40 {
        attrs.push(AttributeFlags::Device);
    }
    if (data & 0x80) == 0x80 {
        attrs.push(AttributeFlags::Normal);
    }
    if (data & 0x100) == 0x100 {
        attrs.push(AttributeFlags::Temporary);
    }
    if (data & 0x200) == 0x200 {
        attrs.push(AttributeFlags::Sparse);
    }
    if (data & 0x400) == 0x400 {
        attrs.push(AttributeFlags::Reparse);
    }
    if (data & 0x800) == 0x800 {
        attrs.push(AttributeFlags::Compressed);
    }
    if (data & 0x1000) == 0x1000 {
        attrs.push(AttributeFlags::Offline);
    }
    if (data & 0x2000) == 0x2000 {
        attrs.push(AttributeFlags::NotIndexed);
    }
    if (data & 0x4000) == 0x4000 {
        attrs.push(AttributeFlags::Encrypted);
    }
    if (data & 0x8000) == 0x8000 {
        attrs.push(AttributeFlags::Unknown);
    }
    if (data & 0x10000) == 0x10000 {
        attrs.push(AttributeFlags::Virtual);
    }
    if (data & 0x10000000) == 0x10000000 {
        attrs.push(AttributeFlags::Directory);
    }
    if (data & 0x20000000) == 0x20000000 {
        attrs.push(AttributeFlags::IndexView);
    }

    attrs
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::{
        filesystem::ntfs::{
            attributes::{
                AttributeFlags, file_attribute_flags, get_filename_attribute, get_raw_file_size,
                read_attribute_data,
            },
            raw_files::{NtfsOptions, iterate_ntfs, raw_reader},
            sector_reader::SectorReader,
            setup::setup_ntfs_parser,
        },
        utils::regex_options::create_regex,
    };
    use ntfs::{Ntfs, NtfsAttributeType, structured_values::NtfsAttributeList};
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
        let flag = file_attribute_flags(test);
        assert_eq!(flag.len(), 1);
        assert_eq!(flag[0], AttributeFlags::ReadOnly)
    }

    #[test]
    fn test_get_raw_file_size() {
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        let ntfs_file = raw_reader("C:\\$MFT", &ntfs_parser.ntfs, &mut ntfs_parser.fs).unwrap();

        let size = get_raw_file_size(&ntfs_file, &mut ntfs_parser.fs).unwrap();
        assert!(size > 1);
    }

    #[test]
    fn test_get_attribute_data() {
        let drive = 'C';
        let mut ntfs_parser = setup_ntfs_parser(drive).unwrap();
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
                &filelist
                    .file
                    .to_file(&ntfs_parser.ntfs, &mut ntfs_parser.fs)
                    .unwrap(),
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
        let mut ntfs_parser = setup_ntfs_parser(drive).unwrap();
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
