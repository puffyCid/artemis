use super::{
    attributes::attribute::{EntryAttributes, grab_attributes},
    error::MftError,
    header::MftHeader,
    reader::{setup_mft_reader, setup_mft_reader_windows},
};
use crate::{
    artifacts::os::windows::mft::{fixup::Fixup, header::EntryFlags},
    filesystem::{
        files::get_file_size,
        ntfs::{attributes::get_raw_file_size, reader::read_bytes},
    },
    utils::nom_helper::nom_data,
};
use crate::{
    artifacts::os::{systeminfo::info::get_platform, windows::artifacts::output_data},
    filesystem::ntfs::setup::setup_ntfs_parser,
    structs::toml::Output,
    utils::time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::windows::{AttributeFlags, MftEntry, Namespace};
use log::{error, warn};
use ntfs::NtfsFile;
use std::{
    collections::{HashMap, HashSet},
    io::BufReader,
};

/// Parse the provided $MFT file and try to re-create filelisting
pub(crate) fn parse_mft(
    path: &str,
    output: &mut Output,
    filter: bool,
    start_time: u64,
) -> Result<(), MftError> {
    let plat = get_platform();
    let size;
    if plat != "Windows" {
        size = get_file_size(path);
        let reader = setup_mft_reader(path)?;
        let mut buf_reader = BufReader::new(reader);

        return read_mft(&mut buf_reader, None, output, start_time, filter, size);
    }

    // Windows we default to parsing the NTFS in order to bypass locked $MFT
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not setup NTFS parser: {err:?}");
            return Err(MftError::Systemdrive);
        }
    };
    let ntfs_file = setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;
    // We use NTFS crate to parse NTFS filesystem to briefly parse part of the MFT to get the size of the MFT so we can later parse the full MFT ourselves...
    let size = match get_raw_file_size(&ntfs_file, &mut ntfs_parser.fs) {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Failed to determine size of $MFT file: {err:?}");
            return Err(MftError::RawSize);
        }
    };

    read_mft(
        &mut ntfs_parser.fs,
        Some(&ntfs_file),
        output,
        start_time,
        filter,
        size,
    )
}

/// Read the MFT in small chunks
fn read_mft<T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    output: &mut Output,
    start_time: u64,
    filter: bool,
    size: u64,
) -> Result<(), MftError> {
    let mut cache: HashMap<String, String> = HashMap::new();
    // Keep a directory cache limit of 1000 entries
    let cache_limit = 1000;

    let mut offset = 0;
    let mut entries = Vec::new();

    let mut extended_attribs = HashMap::new();
    let mut first_pass = 0;

    // We parse 1000 FILE entries at a time
    let file_entries = 1000;

    // We parse the $MFT twice :(
    // First to only get $MFT entries that are extension entries. These entries contain attributes that are part of a real FILE entry but are too large to fit in a single FILE
    // The second pass we parse FILE entries and check our cached extension entries and combine both if there is a match
    // https://harelsegev.github.io/posts/resolving-file-paths-using-the-mft/#pitfall-3-extension-records-missing-attributes-and-orphaned-attributes
    while first_pass < 2 {
        // Read through the MFT. We read 1000 entries at time
        while let Ok(header) = determine_header_info(offset, reader, ntfs_file) {
            // If our offset is larger than the MFT size. Then we are done
            if offset > size {
                break;
            }

            // MFT entry size is 0 bytes. Add 1024 to our offset and move on
            if header.total_size == 0 {
                let default_entry_size = 1024;
                offset += default_entry_size;
                continue;
            }
            // Cache 1000 directories. We use this for quick lookups for FILE entries
            while cache.len() > cache_limit {
                if let Some(key) = cache.keys().next() {
                    let key = key.clone();
                    cache.remove(&key);
                    break;
                }
            }

            // Read 1000 MFT FILE entries
            let mut mft_bytes = match read_bytes(
                offset,
                file_entries * header.total_size as u64,
                ntfs_file,
                reader,
            ) {
                Ok(result) => result,
                Err(err) => {
                    error!("[mft] Could not read entry bytes: {err:?}");
                    break;
                }
            };

            // Parse 1000 entries
            while mft_bytes.len() >= header.total_size as usize {
                let temp_bytes = mft_bytes.clone();
                // Nom first FILE entry
                let (remaining, entry_bytes) = match nom_data(&temp_bytes, header.total_size as u64)
                {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[mft] Could not parse entry bytes: {err:?}");
                        break;
                    }
                };
                if entry_bytes.is_empty() {
                    break;
                }
                // Remaining entries. 1000->999->998->etc
                mft_bytes = remaining.to_vec();
                let header_results = MftHeader::parse_header(entry_bytes);
                let (entry_bytes, mft_header) = match header_results {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[mft] Could not parse header: {err:?}");
                        break;
                    }
                };
                let file0 = 1162627398;
                if mft_header.sig != file0 {
                    continue;
                }

                let fixed_mft_bytes = apply_fixup(entry_bytes, mft_header.fix_up_count)?;
                let mut entry = match grab_attributes(
                    &fixed_mft_bytes,
                    reader,
                    ntfs_file,
                    mft_header.total_size,
                    mft_header.index,
                ) {
                    Ok((_, result)) => result,
                    Err(err) => {
                        error!("[mft] Could not parse mft attributes: {err:?}");
                        break;
                    }
                };

                // On first pass we only get extension entries. On second pass we skip them
                if mft_header.mft_base_seq != 0 && mft_header.mft_base_index != 0 || first_pass == 0
                {
                    if first_pass == 0 {
                        extended_attribs.insert(
                            format!("{}_{}", mft_header.mft_base_index, mft_header.mft_base_seq),
                            entry,
                        );
                    }

                    continue;
                }

                let check_extended_attrib = format!("{}_{}", mft_header.index, mft_header.sequence);
                // On second pass we check if FILE entry has an extended MFT entry. If it does we combine them
                // Commonly seen with Hard links
                if let Some(value) = extended_attribs.get_mut(&check_extended_attrib) {
                    entry.standard.append(&mut value.standard);
                    entry.filename.append(&mut value.filename);
                    entry.attributes.append(&mut value.attributes);
                    extended_attribs.remove(&check_extended_attrib);
                }

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
                        deleted: !mft_header.entry_flags.contains(&EntryFlags::InUse),
                    };

                    if let Some(standard) = entry.standard.first() {
                        mft_entry.created =
                            unixepoch_to_iso(filetime_to_unixepoch(standard.created));
                        mft_entry.modified =
                            unixepoch_to_iso(filetime_to_unixepoch(standard.modified));
                        mft_entry.changed =
                            unixepoch_to_iso(filetime_to_unixepoch(standard.changed));
                        mft_entry.accessed =
                            unixepoch_to_iso(filetime_to_unixepoch(standard.accessed));
                        mft_entry.attributes = standard.file_attributes.clone();
                        mft_entry.usn = standard.usn;
                    }

                    if mft_entry.attributes.is_empty() {
                        mft_entry.attributes = value.file_attributes.clone();
                    }

                    let created = unixepoch_to_iso(filetime_to_unixepoch(value.created));
                    let modified = unixepoch_to_iso(filetime_to_unixepoch(value.modified));
                    let accessed = unixepoch_to_iso(filetime_to_unixepoch(value.accessed));
                    let changed = unixepoch_to_iso(filetime_to_unixepoch(value.changed));

                    mft_entry.filename = value.name.clone();
                    mft_entry.parent_inode = value.parent_mft;
                    mft_entry.inode = mft_header.index;
                    mft_entry.namespace = value.namespace.clone();
                    mft_entry.filename_created = created;
                    mft_entry.filename_modified = modified;
                    mft_entry.filename_accessed = accessed;
                    mft_entry.filename_changed = changed;
                    mft_entry.attribute_list = entry.attributes.clone();

                    if value.file_attributes.contains(&AttributeFlags::Directory) {
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
                    if value.parent_mft == root && mft_header.index != root {
                        mft_entry.full_path = format!(".\\{}", value.name);
                        mft_entry.directory = String::from(".");
                        entries.push(mft_entry);

                        if value.file_attributes.contains(&AttributeFlags::Directory)
                            && value.namespace != Namespace::Dos
                        {
                            cache.insert(
                                format!("{}_{}", mft_header.index, mft_header.sequence),
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

                        if value.file_attributes.contains(&AttributeFlags::Directory)
                            && value.namespace != Namespace::Dos
                        {
                            cache.insert(
                                format!("{}_{}", mft_header.index, mft_header.sequence),
                                mft_entry.full_path.clone(),
                            );
                        }
                        entries.push(mft_entry);

                        continue;
                    }

                    let mut tracker = Lookups {
                        parent_index: value.parent_mft,
                        parent_sequence: value.parent_sequence,
                        size: mft_header.total_size,
                        tracker: HashSet::new(),
                    };

                    let path = lookup_parent(
                        reader,
                        ntfs_file,
                        &mut cache,
                        &extended_attribs,
                        &mut tracker,
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

            offset += file_entries * header.total_size as u64;
        }

        offset = 0;
        first_pass += 1;
    }

    if !entries.is_empty() {
        let _ = output_mft(&entries, output, filter, start_time);
    }

    Ok(())
}

pub(crate) struct Lookups {
    pub(crate) parent_index: u32,
    pub(crate) parent_sequence: u16,
    pub(crate) size: u32,
    pub(crate) tracker: HashSet<String>,
}

/// Try to find parents of a MFT entry. We maintain a small cache to speed up lookup
pub(crate) fn lookup_parent<T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    cache: &mut HashMap<String, String>,
    extended_attribs: &HashMap<String, EntryAttributes>,
    tracker: &mut Lookups,
) -> Result<String, MftError> {
    if tracker.tracker.contains(&format!(
        "{}_{}",
        tracker.parent_index, tracker.parent_sequence
    )) {
        warn!("[mft] Got recursive parent. This is wrong. Stopping lookups now");
        return Ok(String::new());
    }

    // If size is zero get FILE entry size of first MFT entry
    let empty = 0;
    if tracker.size == empty {
        let header = determine_header_info(0, reader, ntfs_file)?;
        tracker.size = header.total_size;
    }

    let offset = (tracker.parent_index * tracker.size) as u64;
    let header = determine_header_info(offset, reader, ntfs_file)?;

    if (tracker.parent_sequence != header.sequence
        || !header.entry_flags.contains(&EntryFlags::InUse))
        || header.sequence == 0 && tracker.parent_sequence != header.sequence - 1
    {
        // Before we give up on finding the parent folder, check if extended attributes contain our deleted parent
        // See: https://harelsegev.github.io/posts/resolving-file-paths-using-the-mft/#orphaned-attributes
        let check_extended_attrib = format!("{}_{}", tracker.parent_index, tracker.parent_sequence);
        if let Some(value) = extended_attribs.get(&check_extended_attrib) {
            // We are only here for the filename
            if let Some(parent_filename) = value.filename.first() {
                let parent_cache = format!(
                    "{}_{}",
                    parent_filename.parent_mft, parent_filename.parent_sequence
                );
                let root = 5;
                if parent_filename.parent_mft == root
                    && parent_filename
                        .file_attributes
                        .contains(&AttributeFlags::Directory)
                {
                    return Ok(format!("$OrphanFiles\\.\\{}", parent_filename.name));
                }
                // Now check if the extended attrib parent is cached. If not continue lookups
                if let Some(cache_hit) = cache.get(&parent_cache) {
                    let path = format!("{cache_hit}\\{}", parent_filename.name);

                    if parent_filename
                        .file_attributes
                        .contains(&AttributeFlags::Directory)
                        && parent_filename.namespace != Namespace::Dos
                    {
                        cache.insert(
                            format!("$OrphanFiles\\{}_{}", header.index, header.sequence),
                            path.clone(),
                        );
                    }

                    return Ok(path);
                }

                // Before we continue lookups. Add current parent to tracker
                // Should help us avoid recursive lookups
                let tracked = format!("{}_{}", tracker.parent_sequence, tracker.parent_sequence);
                tracker.tracker.insert(tracked);
                tracker.parent_index = parent_filename.parent_mft;
                tracker.parent_sequence = parent_filename.parent_sequence;

                // Not found in cache. Go look for it in the MFT
                let parents = lookup_parent(reader, ntfs_file, cache, extended_attribs, tracker)?;
                let path = format!("$OrphanFiles\\{parents}\\{}", parent_filename.name);
                if parent_filename
                    .file_attributes
                    .contains(&AttributeFlags::Directory)
                    && parent_filename.namespace != Namespace::Dos
                {
                    cache.insert(
                        format!("{}_{}", header.index, header.sequence),
                        path.clone(),
                    );
                }
                return Ok(path);
            }
        }

        // Parent is gone
        return Ok(String::from("$OrphanFiles"));
    }

    let header_size = 48;
    let remaining_size = header.total_size - header_size as u32;

    let entry_bytes = match read_bytes(
        header_size + offset,
        remaining_size as u64,
        ntfs_file,
        reader,
    ) {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not read entry bytes: {err:?}");
            return Ok(String::from("$OrphanFiles"));
        }
    };

    let mft_bytes = apply_fixup(&entry_bytes, header.fix_up_count)?;

    let entry = match grab_attributes(
        &mft_bytes,
        reader,
        ntfs_file,
        header.total_size,
        header.index,
    ) {
        Ok((_, result)) => result,
        Err(err) => {
            error!("[mft] Could not parse mft attributes: {err:?}");
            return Ok(String::from("$OrphanFiles"));
        }
    };

    for value in &entry.filename {
        if !value.file_attributes.contains(&AttributeFlags::Directory) {
            return Ok(String::from("$OrphanFiles"));
        }
        if value.namespace == Namespace::Dos && entry.filename.len() != 1 {
            continue;
        }
        let root = 5;
        if value.parent_mft == root && value.file_attributes.contains(&AttributeFlags::Directory) {
            return Ok(format!(".\\{}", value.name));
        }

        let parent_cache = format!("{}_{}", value.parent_mft, value.parent_sequence);
        if let Some(cache_hit) = cache.get(&parent_cache) {
            let path = format!("{cache_hit}\\{}", value.name);

            if value.file_attributes.contains(&AttributeFlags::Directory)
                && value.namespace != Namespace::Dos
            {
                cache.insert(
                    format!("{}_{}", header.index, header.sequence),
                    path.clone(),
                );
            }

            return Ok(path);
        }

        // Before we continue lookups. Add current parent to tracker
        // Should help us avoid recursive lookups
        let tracked = format!("{}_{}", tracker.parent_sequence, tracker.parent_sequence);
        tracker.tracker.insert(tracked);
        tracker.parent_index = value.parent_mft;
        tracker.parent_sequence = value.parent_sequence;

        let parents = lookup_parent(reader, ntfs_file, cache, extended_attribs, tracker)?;
        let path = format!("{parents}\\{}", value.name);
        if value.file_attributes.contains(&AttributeFlags::Directory)
            && value.namespace != Namespace::Dos
        {
            cache.insert(
                format!("{}_{}", header.index, header.sequence),
                path.clone(),
            );
        }
        tracker.tracker.remove(&format!(
            "{}_{}",
            tracker.parent_sequence, tracker.parent_sequence
        ));
        return Ok(path);
    }

    if entry.filename.is_empty() {
        return Ok(String::new());
    }

    Ok(String::from("$OrphanFiles"))
}

/// Try to determine FILE entry size by parsing first 48 bytes of the header
fn determine_header_info<T: std::io::Seek + std::io::Read>(
    offset: u64,
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
) -> Result<MftHeader, MftError> {
    let header_size = 48;
    let header_bytes_results = read_bytes(offset, header_size, ntfs_file, reader);
    let header_bytes = match header_bytes_results {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not read header bytes: {err:?}");
            return Err(MftError::EntrySize);
        }
    };

    let header_results = MftHeader::parse_header(&header_bytes);
    let (_, header) = match header_results {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not parse header: {err:?}");
            return Err(MftError::EntrySize);
        }
    };

    Ok(header)
}

/// Apply fixup values to the FILE entry to ensure accurate data
fn apply_fixup(data: &[u8], count: u16) -> Result<Vec<u8>, MftError> {
    let (entry_bytes, fixup) = match Fixup::get_fixup(data, count) {
        Ok(result) => result,
        Err(err) => {
            error!("[mft] Could not parse mft fixup values: {err:?}");
            return Err(MftError::EntrySize);
        }
    };

    let mut mft_bytes = entry_bytes.to_vec();
    Fixup::apply_fixup(&mut mft_bytes, &fixup);

    Ok(mft_bytes)
}

/// Output MFT data. Due to size of $MFT we will output every 10k entries we parse
fn output_mft(
    entries: &[MftEntry],
    output: &mut Output,
    filter: bool,
    start_time: u64,
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
    use crate::structs::toml::Output;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("csv"),
            timeline: false,
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_parse_mft() {
        use super::parse_mft;
        use std::path::PathBuf;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/mft/win11/MFT");
        let mut output = output_options("mft_test", "local", "./tmp", false);

        parse_mft(&test_location.to_str().unwrap(), &mut output, false, 0).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_mft() {
        use super::setup_ntfs_parser;
        use crate::{
            artifacts::os::windows::mft::master::{read_mft, setup_mft_reader_windows},
            filesystem::ntfs::attributes::get_raw_file_size,
        };

        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();

        let ntfs_file =
            setup_mft_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, "C:\\$MFT").unwrap();

        let mut output = output_options("mft_test", "local", "./tmp", false);
        let size = get_raw_file_size(&ntfs_file, &mut ntfs_parser.fs).unwrap();
        read_mft(
            &mut ntfs_parser.fs,
            Some(&ntfs_file),
            &mut output,
            0,
            false,
            size,
        )
        .unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_nonresident_large_record_length() {
        use super::parse_mft;
        use std::path::PathBuf;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/mft/win11/nonresident.raw");

        let mut output = output_options("mft_test", "local", "./tmp", false);

        parse_mft(&test_location.display().to_string(), &mut output, false, 0).unwrap();
    }
}
