use super::{
    attributes::attribute::grab_attributes,
    error::MftError,
    header::MftHeader,
    reader::{setup_mft_reader, setup_mft_reader_windows},
};
use crate::{
    artifacts::os::windows::mft::{attributes::attribute::Namespace, fixup::Fixup},
    filesystem::ntfs::reader::read_bytes,
};
use crate::{
    artifacts::os::{
        systeminfo::info::get_platform, windows::mft::attributes::attribute::FileAttributes,
    },
    filesystem::ntfs::setup::setup_ntfs_parser,
    structs::toml::Output,
};
use log::error;
use ntfs::NtfsFile;
use std::{
    collections::HashMap,
    io::{BufRead, BufReader},
};

pub(crate) fn parse_mft(
    path: &str,
    output: &mut Output,
    filter: &bool,
    start_time: &u64,
) -> Result<(), MftError> {
    let plat = get_platform();
    if plat != "Windows" {
        let reader = setup_mft_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        return read_mft(&mut buf_reader, None, output);
    }

    // Windows we default to parsing the NTFS in order to bypass locked $MFT
    let ntfs_parser_result = setup_ntfs_parser(&path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not setup NTFS parser: {err:?}");
            return Err(MftError::Systemdrive);
        }
    };
    let ntfs_file = setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;

    read_mft(&mut ntfs_parser.fs, Some(&ntfs_file), output)
}

fn read_mft<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'a>>,
    output: &mut Output,
) -> Result<(), MftError> {
    let mut tracker = HashMap::new();

    let header_size = 48;
    let mut offset = 0;
    let mut entry_size = 1024;
    while reader.fill_buf().is_ok_and(|x| !x.is_empty()) {
        println!("offset: {offset}");
        let header_bytes_results = read_bytes(&offset, header_size, ntfs_file, reader);
        let header_bytes = match header_bytes_results {
            Ok(result) => result,
            Err(err) => {
                error!("[mft] Could not read header bytes: {err:?}");
                break;
            }
        };

        let header_results = MftHeader::parse_header(&header_bytes);
        let (_, header) = match header_results {
            Ok(result) => result,
            Err(err) => {
                error!("[mft] Could not parse header: {err:?}");
                break;
            }
        };

        if header.sig == 0 {
            offset += entry_size;
            continue;
        }
        entry_size = header.total_size as u64;

        let remaining_size = header.total_size - header_size as u32;

        let entry_bytes = match read_bytes(
            &(&header_size + &offset),
            remaining_size as u64,
            ntfs_file,
            reader,
        ) {
            Ok(result) => result,
            Err(err) => {
                error!("[mft] Could not read entry bytes: {err:?}");
                break;
            }
        };
        offset += header.total_size as u64;

        let entry_bytes = match Fixup::get_fixup(&entry_bytes, header.fix_up_count) {
            Ok((input, _fixup)) => input,
            Err(err) => {
                error!("[mft] Could not parse mft fixup values: {err:?}");
                break;
            }
        };

        let entry = match grab_attributes(&entry_bytes) {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[mft] Could not parse mft attributes: {err:?}");
                break;
            }
        };

        let mut parent = 0;
        let mut name = String::new();
        for value in &entry.filename {
            if value.name == "Windowŧ" {
                println!("{header:?}");
                panic!("{:?}", entry.filename);
            }
            if !value.file_attributes.contains(&FileAttributes::Directory)
                || value.namespace == Namespace::Dos
            {
                continue;
            }
            parent = value.parent_mft;
            name = value.name.clone();
            if let Some(cache) = tracker.get(&value.parent_mft) {
                let path = format!("{cache}\\{}", value.name);
                println!("cache: {path}");
                tracker.insert(header.index, path);
            } else {
                tracker.insert(header.index, value.name.clone());
            }
        }

        for value in &entry.standard {
            if !value.file_attributes.contains(&FileAttributes::Directory) {
                continue;
            }
            if let Some(cache) = tracker.get(&parent) {
                let path = format!("{cache}\\{name}");
                println!("cache: {path}");
                tracker.insert(header.index, path);
            } else {
                tracker.insert(header.index, name.clone());
            }
        }

        //println!("{:?}", entry);
    }
    // println!("{tracker:?}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_mft;
    use crate::structs::toml::Output;
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_parse_mft() {
        //let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");
        let mut output = output_options("mft_test", "local", "./tmp", false);

        parse_mft(
            // &test_location.display().to_string(),
            "/home/puffycid/Downloads/$MFT",
            &mut output,
            &false,
            &0,
        )
        .unwrap();
    }

    #[test]
    fn test_nonresident_large_record_length() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/mft/win11/nonresident.raw");

        let mut output = output_options("mft_test", "local", "./tmp", false);

        parse_mft(
            &test_location.display().to_string(),
            &mut output,
            &false,
            &0,
        )
        .unwrap();
    }
}
