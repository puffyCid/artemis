use super::{error::RegistryError, header::RegHeader, keys::sk::SecurityKey};
use crate::{
    artifacts::os::{
        systeminfo::info::get_platform,
        windows::{
            artifacts::output_data,
            registry::{
                parser::ParamsReader,
                reader::{setup_registry_reader, setup_registry_reader_windows},
            },
        },
    },
    filesystem::{
        files::get_filename,
        ntfs::{
            raw_files::raw_reader_by_file_ref, reader::read_bytes, sector_reader::SectorReader,
            setup::setup_ntfs_parser,
        },
    },
    structs::toml::Output,
    utils::{
        regex_options::regex_check,
        time::{filetime_to_unixepoch, time_now, unixepoch_to_iso},
    },
};
use common::windows::RegistryData;
use log::error;
use ntfs::{Ntfs, NtfsFile, NtfsFileReference};
use regex::Regex;
use std::{collections::HashSet, fs::File, io::BufReader};

/// Parse provided `Registry` file at starting Key path and apply any optional Key path regex filtering
/// Use `get_registry_keys_by_ref` if you want to provide a `Registry` file reference
pub(crate) fn get_registry_keys(
    start_path: &str,
    regex: &Regex,
    path: &str,
) -> Result<Vec<RegistryData>, RegistryError> {
    let start_time = time_now();
    let plat = get_platform();

    if plat != "Windows" {
        let reader = setup_registry_reader(path)?;
        let mut buf_reader = BufReader::new(reader);
        let header = RegHeader::read_header(&mut buf_reader, None)?;
        let mut params = ParamsReader {
            start_path: start_path.to_string(),
            path_regex: Some(regex.clone()),
            filter: false,
            registry_path: path.to_string(),
            reader: buf_reader,
            offset: 0,
            size: 0,
            minor_version: header.minor_version,
            key_tracker: Vec::new(),
            offset_tracker: HashSet::new(),
        };

        let mut reg_data = get_root(&mut params, None)?;
        recurse_registry(
            &mut params,
            &mut None,
            false,
            &mut reg_data,
            None,
            start_time,
        )?;
        return Ok(reg_data);
    }

    // On Windows we default to parsing the NTFS in order to bypass locked Registry files
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not setup NTFS parser: {err:?}");
            return Err(RegistryError::SystemDrive);
        }
    };
    let ntfs_file = setup_registry_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;
    let header = RegHeader::read_header(&mut ntfs_parser.fs, Some(&ntfs_file))?;

    let mut params = ParamsReader {
        start_path: start_path.to_string(),
        path_regex: Some(regex.clone()),
        filter: false,
        registry_path: path.to_string(),
        reader: ntfs_parser.fs,
        offset: 0,
        size: 0,
        minor_version: header.minor_version,
        key_tracker: Vec::new(),
        offset_tracker: HashSet::new(),
    };

    let mut reg_data = get_root(&mut params, Some(&ntfs_file))?;
    recurse_registry(
        &mut params,
        &mut None,
        false,
        &mut reg_data,
        Some(&ntfs_file),
        start_time,
    )?;

    Ok(reg_data)
}

/// Parse provided `Registry` file reference at starting Key path and apply any optional Key path regex filtering
/// Use `get_registry_keys` if you want to provide a `Registry` file
pub(crate) fn get_registry_keys_by_ref(
    start_path: &str,
    regex: &Regex,
    file_ref: NtfsFileReference,
    path: &str,
) -> Result<Vec<RegistryData>, RegistryError> {
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not setup NTFS parser: {err:?}");
            return Err(RegistryError::SystemDrive);
        }
    };

    let use_ntfs = registry_ref_reader(file_ref, &ntfs_parser.ntfs, &mut ntfs_parser.fs)?;
    let mut params = ParamsReader {
        start_path: start_path.to_string(),
        path_regex: Some(regex.clone()),
        key_tracker: Vec::new(),
        offset_tracker: HashSet::new(),
        filter: false,
        registry_path: String::new(),
        reader: ntfs_parser.fs,
        offset: 0,
        size: 0,
        minor_version: 4,
    };
    let mut reg_data = get_root(&mut params, Some(&use_ntfs))?;
    recurse_registry(
        &mut params,
        &mut None,
        false,
        &mut reg_data,
        Some(&use_ntfs),
        time_now(),
    )?;

    Ok(reg_data)
}

/// Return a reader to the `Registry` file provided at file reference
pub(crate) fn registry_ref_reader<'a>(
    ntfs_ref: NtfsFileReference,
    ntfs: &'a Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<NtfsFile<'a>, RegistryError> {
    let result = raw_reader_by_file_ref(ntfs_ref, ntfs, fs);
    match result {
        Ok(buffer) => Ok(buffer),
        Err(err) => {
            error!("[registry] Failed to read registry file reference: {err:?}");
            Err(RegistryError::ReadRegistry)
        }
    }
}

/// Lookup Security Key info based on SK offset.
pub(crate) fn lookup_sk_info(path: &str, sk_offset: i32) -> Result<SecurityKey, RegistryError> {
    let plat = get_platform();
    let empty = 0;
    if sk_offset < empty {
        error!("[registry] Provided unallocated offset. Refusing to parse SK data.");
        return Err(RegistryError::ReadRegistry);
    }
    let adjust_offset = 4096;
    // Since we are jumping straight to the SK offset we need to add 4096 to skip the HBIN header
    let offset = sk_offset + adjust_offset;
    if plat != "Windows" {
        let reader = setup_registry_reader(path)?;
        let mut buf_reader = BufReader::new(reader);
        let sk_bytes = match read_bytes(offset as u64, 4096, None, &mut buf_reader) {
            Ok(result) => result,
            Err(err) => {
                error!("[registry] Could not read Security info at offset {offset}: {err:?}");
                return Err(RegistryError::Parser);
            }
        };

        let sk_result = SecurityKey::parse_security_key(&sk_bytes);
        let sk = if let Ok((_, result)) = sk_result {
            result
        } else {
            error!("[registry] Could not parse Security info at offset {sk_offset}");
            return Err(RegistryError::Parser);
        };
        return Ok(sk);
    }

    // On Windows we default to parsing the NTFS in order to bypass locked Registry files
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not setup NTFS parser: {err:?}");
            return Err(RegistryError::SystemDrive);
        }
    };
    let ntfs_file = setup_registry_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;
    let sk_bytes = match read_bytes(offset as u64, 4096, Some(&ntfs_file), &mut ntfs_parser.fs) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not read Security info at offset {offset}: {err:?}");
            return Err(RegistryError::Parser);
        }
    };

    let sk_result = SecurityKey::parse_security_key(&sk_bytes);
    let sk = if let Ok((_, result)) = sk_result {
        result
    } else {
        error!("[registry] Could not parse Security info at offset {sk_offset}");
        return Err(RegistryError::Parser);
    };
    Ok(sk)
}

/// Read and stream the Registry data. Will output Registry data every 200 entries
pub(crate) fn stream_registry(
    path: &str,
    start_path: &str,
    regex: Option<&Regex>,
    output: &mut Output,
    filter: bool,
) -> Result<(), RegistryError> {
    let start_time = time_now();
    let plat = get_platform();
    let no_lists = -1;
    if plat != "Windows" {
        let reader = setup_registry_reader(path)?;
        let mut buf_reader = BufReader::new(reader);
        let header = RegHeader::read_header(&mut buf_reader, None)?;
        let mut params = ParamsReader {
            start_path: start_path.to_string(),
            path_regex: regex.cloned(),
            filter,
            registry_path: path.to_string(),
            reader: buf_reader,
            offset: 0,
            size: 0,
            minor_version: header.minor_version,
            key_tracker: Vec::new(),
            offset_tracker: HashSet::new(),
        };

        let mut reg_data = get_root(&mut params, None)?;
        if params.offset as i32 != no_lists {
            recurse_registry(
                &mut params,
                &mut Some(output),
                filter,
                &mut reg_data,
                None,
                start_time,
            )?;
        }

        if !reg_data.is_empty() {
            let mut serde_data = match serde_json::to_value(&reg_data) {
                Ok(results) => results,
                Err(err) => {
                    error!(
                        "[registry] Failed to serialize Registry file {}: {err:?}",
                        params.registry_path
                    );
                    return Err(RegistryError::Serialize);
                }
            };

            if let Err(err) = output_data(
                &mut serde_data,
                "registry",
                output,
                start_time,
                params.filter,
            ) {
                error!(
                    "[registry] Failed to output data for {}, error: {err:?}",
                    params.registry_path
                );
                return Err(RegistryError::Output);
            }
        }
        return Ok(());
    }

    // On Windows we default to parsing the NTFS in order to bypass locked Registry files
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Could not setup NTFS parser: {err:?}");
            return Err(RegistryError::SystemDrive);
        }
    };
    let ntfs_file = setup_registry_reader_windows(&ntfs_parser.ntfs, &mut ntfs_parser.fs, path)?;
    let header = RegHeader::read_header(&mut ntfs_parser.fs, Some(&ntfs_file))?;

    let mut params = ParamsReader {
        start_path: start_path.to_string(),
        path_regex: regex.cloned(),
        filter,
        registry_path: path.to_string(),
        reader: ntfs_parser.fs,
        offset: 0,
        size: 0,
        minor_version: header.minor_version,
        key_tracker: Vec::new(),
        offset_tracker: HashSet::new(),
    };

    let mut reg_data = get_root(&mut params, Some(&ntfs_file))?;
    if params.offset as i32 != no_lists {
        recurse_registry(
            &mut params,
            &mut Some(output),
            filter,
            &mut reg_data,
            Some(&ntfs_file),
            start_time,
        )?;
    }

    if !reg_data.is_empty() {
        let mut serde_data = match serde_json::to_value(&reg_data) {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "[registry] Failed to serialize Registry file {}: {err:?}",
                    params.registry_path
                );
                return Err(RegistryError::Serialize);
            }
        };

        if let Err(err) = output_data(
            &mut serde_data,
            "registry",
            output,
            start_time,
            params.filter,
        ) {
            error!(
                "[registry] Failed to output data for {}, error: {err:?}",
                params.registry_path
            );
            return Err(RegistryError::Output);
        }
    }

    Ok(())
}

/// Walk the Registry and parse the data
fn recurse_registry<'a, T: std::io::Seek + std::io::Read>(
    params: &mut ParamsReader<T>,
    output: &mut Option<&mut Output>,
    filter: bool,
    reg_data: &mut Vec<RegistryData>,
    use_ntfs: Option<&NtfsFile<'a>>,
    start_time: u64,
) -> Result<(), RegistryError> {
    let root = 0;
    // We never recurse from the root of the Registry
    // `get_root()` should always be called prior to recursion
    if params.offset == root {
        return Ok(());
    }
    // Max Registry keys to store in our array. Before outputting
    // Smaller limit should mean less memory
    let key_limit = 200;
    if params.offset_tracker.contains(&params.offset) || reg_data.len() > key_limit {
        if let Some(out) = output {
            let mut serde_data = match serde_json::to_value(&reg_data) {
                Ok(results) => results,
                Err(err) => {
                    error!(
                        "[registry] Failed to serialize Registry file {}: {err:?}",
                        params.registry_path
                    );
                    return Err(RegistryError::Serialize);
                }
            };
            if let Err(err) =
                output_data(&mut serde_data, "registry", out, start_time, params.filter)
            {
                error!(
                    "[registry] Failed to output data for {}, error: {err:?}",
                    params.registry_path
                );
                return Err(RegistryError::Output);
            }
            *reg_data = Vec::new();
        }

        if params.offset_tracker.contains(&params.offset) {
            return Ok(());
        }
    }
    params.offset_tracker.insert(params.offset);

    let names = params.list_keys(use_ntfs)?;

    for name in names {
        if params
            .offset_tracker
            .contains(&(name.subkeys_list_offset as u32))
        {
            continue;
        }

        let mut registry_entry = RegistryData {
            path: String::new(),
            key: params.key_tracker.join("\\"),
            name: name.key_name.clone(),
            values: Vec::new(),
            last_modified: unixepoch_to_iso(filetime_to_unixepoch(name.last_modified)),
            depth: params.key_tracker.len(),
            security_offset: name.key_security_offset,
            registry_path: params.registry_path.clone(),
            registry_file: get_filename(&params.registry_path),
        };
        params.key_tracker.push(name.key_name);
        registry_entry.path = params.key_tracker.join("\\");

        let no_lists = -1;
        if name.key_values_offset != no_lists {
            params.offset = name.key_values_offset as u32;
            let values = params.list_values(use_ntfs, name.number_key_values)?;
            registry_entry.values = values;
        }

        // Case sensitivity does not matter for Registry keys
        if registry_entry
            .path
            .to_lowercase()
            .starts_with(&params.start_path.to_lowercase())
            && (params.path_regex.as_ref().is_some_and(|regex_match| {
                regex_check(&regex_match, &registry_entry.path.to_lowercase())
            }) || params.path_regex.is_none())
        {
            reg_data.push(registry_entry);
        }

        if name.subkeys_list_offset != no_lists {
            params.offset = name.subkeys_list_offset as u32;
            recurse_registry(params, output, filter, reg_data, use_ntfs, start_time)?;
        }

        // pop the params.key_tracker if we finished parsing a name key
        params.key_tracker.pop();
    }
    Ok(())
}

/// Get the Root Registry Key
fn get_root<'a, T: std::io::Seek + std::io::Read>(
    params: &mut ParamsReader<T>,
    use_ntfs: Option<&NtfsFile<'a>>,
) -> Result<Vec<RegistryData>, RegistryError> {
    let root = params.root_key(use_ntfs)?;
    let hbin = params.get_header(use_ntfs)?;
    params.minor_version = hbin.minor_version;
    let no_lists = -1;
    let mut reg_data = Vec::new();
    let mut registry_entry = RegistryData {
        path: String::new(),
        key: params.key_tracker.join("\\"),
        name: root.key_name.clone(),
        values: Vec::new(),
        last_modified: unixepoch_to_iso(filetime_to_unixepoch(root.last_modified)),
        depth: params.key_tracker.len(),
        security_offset: root.key_security_offset,
        registry_path: params.registry_path.clone(),
        registry_file: get_filename(&params.registry_path),
    };
    params.key_tracker.push(root.key_name);
    registry_entry.path = params.key_tracker.join("\\");

    if root.key_values_offset != no_lists {
        params.offset = root.key_values_offset as u32;
        let values = params.list_values(use_ntfs, root.number_key_values)?;
        registry_entry.values = values;
    }

    if root.subkeys_list_offset != no_lists {
        params.offset = root.subkeys_list_offset as u32;
    }

    // Case sensitivity does not matter for Registry keys
    if registry_entry
        .path
        .to_lowercase()
        .starts_with(&params.start_path.to_lowercase())
        && params.path_regex.as_ref().is_some_and(|regex_match| {
            regex_check(&regex_match, &registry_entry.path.to_lowercase())
        })
    {
        reg_data.push(registry_entry);
    }

    Ok(reg_data)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_registry_keys, get_registry_keys_by_ref};
    use crate::{
        artifacts::os::windows::registry::helper::{
            lookup_sk_info, registry_ref_reader, stream_registry,
        },
        filesystem::ntfs::{raw_files::get_user_registry_files, setup::setup_ntfs_parser},
        structs::toml::Output,
    };
    use regex::Regex;
    use std::path::PathBuf;

    #[test]
    fn test_get_registry_keys() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let start_path = "ROOT\\SOFTWARE\\Microsoft\\";
        let regex = Regex::new(r".*\\typedurls").unwrap();
        let result =
            get_registry_keys(start_path, &regex, &test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].name, "TypedURLs");
        assert_eq!(
            result[0].path,
            "ROOT\\SOFTWARE\\Microsoft\\Internet Explorer\\TypedURLs"
        );
        assert_eq!(
            result[0].key,
            "ROOT\\SOFTWARE\\Microsoft\\Internet Explorer"
        );
        assert_eq!(result[0].values.len(), 1);

        assert_eq!(result[0].values[0].value, "url1");
        assert_eq!(result[0].values[0].data_type, "REG_SZ");
        assert_eq!(
            result[0].values[0].data,
            "http://go.microsoft.com/fwlink/p/?LinkId=255141"
        );
        assert_eq!(result[0].last_modified, "2019-12-07T09:16:14.000Z");
        assert_eq!(result[0].depth, 4);
    }

    #[test]
    fn test_get_all_registry_keys() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let start_path = "";
        let regex = Regex::new("").unwrap();
        let result =
            get_registry_keys(start_path, &regex, &test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 666);
    }

    #[test]
    fn test_lookup_sk_info() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let result = lookup_sk_info(&test_location.display().to_string(), 368).unwrap();
        assert_eq!(result.reference_count, 1);
        assert_eq!(result.info.owner_sid, "S-1-5-32-544");
    }

    #[test]
    fn test_parse_infinite_loop_registry_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // This Registry contains an infinte loop. An offset points to parent offset
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER_Bad.DAT");
        let start_path = "";
        let regex = Regex::new("").unwrap();
        let result =
            get_registry_keys(start_path, &regex, &test_location.display().to_string()).unwrap();
        // The infinte loop causes the parser to skip two values
        assert_eq!(result.len(), 664);
    }

    #[test]
    fn test_get_registry_keys_by_ref() {
        let user_hives = get_user_registry_files('C').unwrap();
        for hive in user_hives {
            if hive.filename != "NTUSER.DAT" {
                continue;
            }
            let result = get_registry_keys_by_ref(
                "",
                &Regex::new("").unwrap(),
                hive.reg_reference,
                &hive.full_path,
            )
            .unwrap();
            assert!(result.len() > 10);
            break;
        }
    }

    #[test]
    fn test_read_registry_ref() {
        let user_hives = get_user_registry_files('C').unwrap();
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        for hive in user_hives {
            if hive.filename != "NTUSER.DAT" {
                continue;
            }
            let result =
                registry_ref_reader(hive.reg_reference, &ntfs_parser.ntfs, &mut ntfs_parser.fs)
                    .unwrap();
            assert!(result.data_size() > 10);
        }
    }

    #[test]
    fn test_stream_registry() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let mut output = Output {
            name: String::from("stream_registry"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: None,
            filter_script: None,
            logging: None,
        };

        stream_registry(
            test_location.to_str().unwrap(),
            "",
            None,
            &mut output,
            false,
        )
        .unwrap();
    }
}
