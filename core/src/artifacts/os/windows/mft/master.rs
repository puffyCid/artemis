use super::{
    attributes::attribute::grab_attributes,
    error::MftError,
    header::MftHeader,
    reader::{setup_mft_reader, setup_mft_reader_windows},
};
use crate::{
    artifacts::os::windows::mft::{fixup::Fixup, header::EntryFlags},
    filesystem::ntfs::reader::read_bytes,
};
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

/// Parse the provided $MFT file and try to re-create filelisting
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

/// Read the MFT in small chunks
fn read_mft<T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), MftError> {
    let mut cache: HashMap<String, String> = HashMap::new();
    // Keep a directory cache limit of 1000 entries
    let cache_limit = 1000;

    let header_size = 48;
    let mut offset = 0;
    let mut entry_size = 1024;

    let mut entries = Vec::new();

    while reader.fill_buf().is_ok_and(|x| !x.is_empty()) {
        while cache.len() > cache_limit {
            if let Some(key) = cache.keys().next() {
                let key = key.clone();
                cache.remove(&key);
                break;
            }
        }
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
            &(header_size + offset),
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

        // Skip Extension MFT records
        if header.mft_base_seq != 0 && header.mft_base_index != 0 {
            //continue;
        }

        let (entry_bytes, fixup) = match Fixup::get_fixup(&entry_bytes, header.fix_up_count) {
            Ok(result) => result,
            Err(err) => {
                panic!("[mft] Could not parse mft fixup values: {err:?}");
                break;
            }
        };

        let mut mft_bytes = entry_bytes.to_vec();
        Fixup::apply_fixup(&mut mft_bytes, &fixup);

        let entry = match grab_attributes(
            &mft_bytes,
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
                deleted: !header.entry_flags.contains(&EntryFlags::InUse),
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
                mft_entry.size = entry.size;
                mft_entry.extension = value
                    .name
                    .split_terminator(".")
                    .last()
                    .unwrap_or_default()
                    .to_string();
            }

            let root = 5;
            if value.parent_mft == root && header.index != root {
                mft_entry.full_path = format!(".\\{}", value.name);
                mft_entry.directory = String::from(".");
                entries.push(mft_entry);

                if value.file_attributes.contains(&FileAttributes::Directory)
                    && value.namespace != Namespace::Dos
                {
                    cache.insert(
                        format!("{}_{}", header.index, header.sequence),
                        format!(".\\{}", value.name),
                    );
                    continue;
                }
                continue;
            }

            if let Some(cache_hit) =
                cache.get(&format!("{}_{}", value.parent_mft, value.parent_sequence))
            {
                let path = format!("{cache_hit}\\{}", value.name);
                mft_entry.full_path = path;

                let path_components: Vec<&str> = mft_entry.full_path.split("\\").collect();
                if let Some((_, components)) = path_components.split_last() {
                    mft_entry.directory = components.join("\\");
                }

                if value.file_attributes.contains(&FileAttributes::Directory)
                    && value.namespace != Namespace::Dos
                {
                    cache.insert(
                        format!("{}_{}", header.index, header.sequence),
                        mft_entry.full_path.clone(),
                    );
                }
                entries.push(mft_entry);

                continue;
            }

            let path = lookup_parent(
                reader,
                ntfs_file,
                &value.parent_mft,
                &value.parent_sequence,
                &header.total_size,
                &mut cache,
            )?;

            mft_entry.full_path = format!("{path}\\{}", value.name);
            mft_entry.directory = path;

            entries.push(mft_entry);
        }

        let limit = 1000;
        if entries.len() >= limit {
            let _ = output_mft(&entries, output, filter, start_time);
            entries = Vec::new();
        }
    }
    if !entries.is_empty() {
        let _ = output_mft(&entries, output, filter, start_time);
        entries = Vec::new();
    }

    Ok(())
}

/// Try to find parents of a MFT entry. We maintain a small cache to speed up lookup
fn lookup_parent<T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    parent_index: &u32,
    parent_sequence: &u16,
    size: &u32,
    cache: &mut HashMap<String, String>,
) -> Result<String, MftError> {
    let header_size = 48;
    let offset = (parent_index * size) as u64;
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

    if (*parent_sequence != header.sequence || !header.entry_flags.contains(&EntryFlags::InUse))
        && *parent_sequence != header.sequence - 1
    {
        return Ok(String::from("$OrphanFiles"));
    }

    let remaining_size = header.total_size - header_size as u32;

    let entry_bytes = match read_bytes(
        &(header_size + offset),
        remaining_size as u64,
        ntfs_file,
        reader,
    ) {
        Ok(result) => result,
        Err(err) => {
            panic!("[mft] Could not read entry bytes: {err:?}");
        }
    };

    let (entry_bytes, fixup) = match Fixup::get_fixup(&entry_bytes, header.fix_up_count) {
        Ok(result) => result,
        Err(err) => {
            panic!("[mft] Could not parse mft fixup values: {err:?}");
        }
    };

    let mut mft_bytes = entry_bytes.to_vec();
    Fixup::apply_fixup(&mut mft_bytes, &fixup);

    let entry = match grab_attributes(
        &mft_bytes,
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
            return Ok(String::from("$OrphanFiles"));
        }
        if value.namespace == Namespace::Dos && entry.filename.len() != 1 {
            continue;
        }
        let root = 5;
        if value.parent_mft == root && value.file_attributes.contains(&FileAttributes::Directory) {
            return Ok(format!(".\\{}", value.name));
        }

        if let Some(cache_hit) =
            cache.get(&format!("{}_{}", value.parent_mft, value.parent_sequence))
        {
            let path = format!("{cache_hit}\\{}", value.name);

            if value.file_attributes.contains(&FileAttributes::Directory)
                && value.namespace != Namespace::Dos
            {
                cache.insert(
                    format!("{}_{}", header.index, header.sequence),
                    path.clone(),
                );
            }

            return Ok(path);
        }

        let parents = lookup_parent(
            reader,
            ntfs_file,
            &value.parent_mft,
            &value.parent_sequence,
            size,
            cache,
        )?;
        let path = format!("{parents}\\{}", value.name);
        if value.file_attributes.contains(&FileAttributes::Directory)
            && value.namespace != Namespace::Dos
        {
            cache.insert(
                format!("{}_{}", header.index, header.sequence),
                path.clone(),
            );
        }
        return Ok(path);
    }

    if entry.filename.is_empty() {
        return Ok(String::new());
    }

    println!("{entry:?}");
    panic!("umm wrong?")
}

/// Output MFT data. Due to size of $MFT we will output every 10k entries we parse
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
            format: String::from("csv"),
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
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/mft/win11/MFT");
        let mut output = output_options("mft_test", "local", "./tmp", false);

        parse_mft(&test_location.to_str().unwrap(), &mut output, &false, &0).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_mft() {
        use super::setup_ntfs_parser;
        use crate::artifacts::os::windows::mft::master::{read_mft, setup_mft_reader_windows};

        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();

        let ntfs_file =
            setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, "C:\\$MFT").unwrap();

        let mut output = output_options("mft_test", "local", "./tmp", false);
        read_mft(
            &mut ntfs_parser.fs,
            Some(&ntfs_file),
            &mut output,
            &0,
            &false,
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
