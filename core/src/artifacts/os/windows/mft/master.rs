use super::{
    attributes::attribute::grab_attributes,
    error::MftError,
    header::MftHeader,
    reader::{setup_mft_reader, setup_mft_reader_windows},
};
use crate::{artifacts::os::windows::mft::fixup::Fixup, filesystem::ntfs::reader::read_bytes};
use crate::{
    artifacts::os::{systeminfo::info::get_platform, windows::artifacts::output_data},
    filesystem::ntfs::setup::setup_ntfs_parser,
    structs::toml::Output,
    utils::time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::windows::{FileAttributes, MftEntry, Namespace};
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

        return read_mft(&mut buf_reader, None, output, start_time, filter);
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

    read_mft(
        &mut ntfs_parser.fs,
        Some(&ntfs_file),
        output,
        start_time,
        filter,
    )
}

fn read_mft<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'a>>,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), MftError> {
    let mut cache = HashMap::new();

    let header_size = 48;
    let mut offset = 0;
    let mut entry_size = 1024;

    let mut entries = Vec::new();
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
                panic!("[mft] Could not read entry bytes: {err:?}");
                break;
            }
        };
        offset += header.total_size as u64;

        let entry_bytes = match Fixup::get_fixup(&entry_bytes, header.fix_up_count) {
            Ok((input, _fixup)) => input,
            Err(err) => {
                panic!("[mft] Could not parse mft fixup values: {err:?}");
                break;
            }
        };

        let entry = match grab_attributes(
            &entry_bytes,
            reader,
            ntfs_file,
            &header.total_size,
            &header.index,
        ) {
            Ok((_, result)) => result,
            Err(err) => {
                panic!("[mft] Could not parse mft attributes: {err:?}");
                break;
            }
        };

        for value in &entry.filename {
            let mut mft_entry = MftEntry {
                filename: String::new(),
                directory: String::new(),
                full_path: String::new(),
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
                is_file: false,
                is_directory: false,
                attributes: Vec::new(),
                namespace: Namespace::Unknown,
                usn: 0,
                parent_inode: 0,
                attribute_list: Vec::new(),
            };

            if let Some(standard) = entry.standard.first() {
                mft_entry.created = unixepoch_to_iso(&filetime_to_unixepoch(&standard.created));
                mft_entry.modified = unixepoch_to_iso(&filetime_to_unixepoch(&standard.modified));
                mft_entry.changed = unixepoch_to_iso(&filetime_to_unixepoch(&standard.changed));
                mft_entry.accessed = unixepoch_to_iso(&filetime_to_unixepoch(&standard.accessed));
                mft_entry.attributes = standard.file_attributes.clone();
                mft_entry.usn = standard.usn;
            }

            if mft_entry.attributes.is_empty() {
                mft_entry.attributes = value.file_attributes.clone();
            }

            let created = unixepoch_to_iso(&filetime_to_unixepoch(&value.created));
            let modified = unixepoch_to_iso(&filetime_to_unixepoch(&value.modified));
            let accessed = unixepoch_to_iso(&filetime_to_unixepoch(&value.accessed));
            let changed = unixepoch_to_iso(&filetime_to_unixepoch(&value.changed));

            mft_entry.filename = value.name.clone();
            mft_entry.parent_inode = value.parent_mft;
            mft_entry.inode = header.index;
            mft_entry.size = value.size;
            mft_entry.namespace = value.namespace.clone();
            mft_entry.filename_created = created;
            mft_entry.filename_modified = modified;
            mft_entry.filename_accessed = accessed;
            mft_entry.filename_changed = changed;
            mft_entry.attribute_list = entry.attributes.clone();

            if value.file_attributes.contains(&FileAttributes::Directory) {
                mft_entry.is_directory = true;
            } else {
                mft_entry.is_file = true;
            }

            let root = 5;
            if value.parent_mft == root && header.index != root {
                mft_entry.full_path = format!(".\\{}", value.name);
                mft_entry.directory = String::from(".");
                entries.push(mft_entry);

                if value.file_attributes.contains(&FileAttributes::Directory)
                    && value.namespace != Namespace::Dos
                {
                    cache.insert(header.index, format!(".\\{}", value.name));
                    continue;
                }
                continue;
            }

            if let Some(cache_hit) = cache.get(&value.parent_mft) {
                let path = format!("{cache_hit}\\{}", value.name);
                mft_entry.full_path = path;

                let path_components: Vec<&str> = mft_entry.full_path.split("\\").collect();
                if let Some((_, components)) = path_components.split_last() {
                    mft_entry.directory = components.join("\\");
                }

                if value.file_attributes.contains(&FileAttributes::Directory)
                    && value.namespace != Namespace::Dos
                {
                    cache.insert(header.index, mft_entry.full_path.clone());
                }
                entries.push(mft_entry);

                continue;
            }

            let path = lookup_parent(
                reader,
                ntfs_file,
                &value.parent_mft,
                &header.total_size,
                &mut cache,
            )?;

            mft_entry.full_path = format!("{path}\\{}", value.name);
            mft_entry.directory = path;

            entries.push(mft_entry);
        }

        let limit = 10000;
        if entries.len() >= limit {
            output_mft(&entries, output, filter, start_time);
            entries = Vec::new();
        }
    }
    if !entries.is_empty() {
        output_mft(&entries, output, filter, start_time);
        entries = Vec::new();
    }
    Ok(())
}

fn lookup_parent<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'a>>,
    parent_index: &u32,
    size: &u32,
    cache: &mut HashMap<u32, String>,
) -> Result<String, MftError> {
    let header_size = 48;
    let offset = (parent_index * size) as u64;
    println!("offset: {offset}");
    let header_bytes_results = read_bytes(&offset, header_size, ntfs_file, reader);
    let header_bytes = match header_bytes_results {
        Ok(result) => result,
        Err(err) => {
            panic!("[mft] Could not read header bytes: {err:?}");
        }
    };

    let header_results = MftHeader::parse_header(&header_bytes);
    let (_, header) = match header_results {
        Ok(result) => result,
        Err(err) => {
            panic!("[mft] Could not parse header: {err:?}");
        }
    };
    let remaining_size = header.total_size - header_size as u32;

    let entry_bytes = match read_bytes(
        &(&header_size + &offset),
        remaining_size as u64,
        ntfs_file,
        reader,
    ) {
        Ok(result) => result,
        Err(err) => {
            panic!("[mft] Could not read entry bytes: {err:?}");
        }
    };

    let entry_bytes = match Fixup::get_fixup(&entry_bytes, header.fix_up_count) {
        Ok((input, _fixup)) => input,
        Err(err) => {
            panic!("[mft] Could not parse mft fixup values: {err:?}");
        }
    };
    let entry = match grab_attributes(
        &entry_bytes,
        reader,
        ntfs_file,
        &header.total_size,
        &header.index,
    ) {
        Ok((_, result)) => result,
        Err(err) => {
            panic!("[mft] Could not parse mft attributes: {err:?}");
        }
    };

    for value in &entry.filename {
        if !value.file_attributes.contains(&FileAttributes::Directory) {
            return Ok(format!("Path Unknown"));
        }
        if value.namespace == Namespace::Dos && entry.filename.len() != 1 {
            continue;
        }
        let root = 5;
        if value.parent_mft == root {
            if value.file_attributes.contains(&FileAttributes::Directory) {
                return Ok(format!(".\\{}", value.name));
            }
        }

        if let Some(cache_hit) = cache.get(&value.parent_mft) {
            let path = format!("{cache_hit}\\{}", value.name);
            println!("cache: {path}");

            if value.file_attributes.contains(&FileAttributes::Directory)
                && value.namespace != Namespace::Dos
            {
                cache.insert(header.index, path.clone());
            }

            return Ok(path);
        }

        let parents = lookup_parent(reader, ntfs_file, &value.parent_mft, size, cache)?;
        let path = format!("{parents}\\{}", value.name);
        if value.file_attributes.contains(&FileAttributes::Directory)
            && value.namespace != Namespace::Dos
        {
            cache.insert(header.index, path.clone());
        }
        return Ok(path);
    }

    if entry.filename.is_empty() {
        return Ok(String::new());
    }

    println!("{entry:?}");
    panic!("umm wrong?")
}

fn output_mft(
    entries: &[MftEntry],
    output: &mut Output,
    filter: &bool,
    start_time: &u64,
) -> Result<(), MftError> {
    if entries.is_empty() {
        return Ok(());
    }

    let serde_data_result = serde_json::to_value(entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[mft] Failed to serialize MFT entries: {err:?}");
            return Err(MftError::Serialize);
        }
    };
    let result = output_data(&mut serde_data, "mft", output, start_time, filter);
    match result {
        Ok(_result) => {}
        Err(err) => {
            error!("[mft] Could not output MFT messages: {err:?}");
            return Err(MftError::OutputData);
        }
    }

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
