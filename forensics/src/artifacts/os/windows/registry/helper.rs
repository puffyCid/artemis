use super::{
    error::RegistryError, hbin::HiveBin, header::RegHeader, keys::sk::SecurityKey, parser::Params,
};
use crate::filesystem::ntfs::{
    raw_files::{raw_read_by_file_ref, raw_read_file},
    setup::NtfsParser,
};
use common::windows::RegistryData;
use log::error;
use nom::bytes::complete::take;
use ntfs::NtfsFileReference;
use regex::Regex;
use std::collections::HashMap;

/// Parse provided `Registry` file at starting Key path and apply any optional Key path regex filtering
/// Use `get_registry_keys_by_ref` if you want to provide a `Registry` file reference
pub(crate) fn get_registry_keys(
    start_path: &str,
    regex: &Regex,
    file_path: &str,
) -> Result<Vec<RegistryData>, RegistryError> {
    let mut params = Params {
        start_path: start_path.to_string(),
        path_regex: regex.clone(),
        registry_list: Vec::new(),
        key_tracker: Vec::new(),
        offset_tracker: HashMap::new(),
        filter: false,
        registry_path: file_path.to_string(),
    };
    let buffer = read_registry(file_path)?;
    let reg_entries_results = parse_raw_registry(&buffer, &mut params);
    match reg_entries_results {
        Ok((_, results)) => Ok(results),
        Err(_err) => {
            error!("[registry] Failed to parse registry file {file_path}");
            Err(RegistryError::Parser)
        }
    }
}

/// Parse provided `Registry` file reference at starting Key path and apply any optional Key path regex filtering
/// Use `get_registry_keys` if you want to provide a `Registry` file
pub(crate) fn get_registry_keys_by_ref(
    start_path: &str,
    regex: &Regex,
    file_ref: &NtfsFileReference,
    ntfs_parser: &mut NtfsParser,
) -> Result<Vec<RegistryData>, RegistryError> {
    let mut params = Params {
        start_path: start_path.to_string(),
        path_regex: regex.clone(),
        registry_list: Vec::new(),
        key_tracker: Vec::new(),
        offset_tracker: HashMap::new(),
        filter: false,
        registry_path: String::new(),
    };
    let buffer = read_registry_ref(file_ref, ntfs_parser)?;
    let reg_entries_results = parse_raw_registry(&buffer, &mut params);
    match reg_entries_results {
        Ok((_, results)) => Ok(results),
        Err(_err) => {
            error!("[registry] Failed to parse registry file reference: {file_ref:?}");
            Err(RegistryError::Parser)
        }
    }
}

/// Parse the provided `Registry` bytes with associated parsing parameters
pub(crate) fn parse_raw_registry<'a>(
    data: &'a [u8],
    params: &mut Params,
) -> nom::IResult<&'a [u8], Vec<RegistryData>> {
    let (input, header) = RegHeader::parse_header(data)?;

    let (_, reg_data) = take(header.hive_bins_size)(input)?;
    let (_, result) = HiveBin::parse_hive_bin_header(reg_data)?;
    let (input, hbin_data) = take(result.size)(reg_data)?;

    let (_, result) = HiveBin::parse_hive_cells(reg_data, hbin_data, params, header.minor_version)?;

    Ok((input, result))
}

/// Read the `Registry` file provided at path
pub(crate) fn read_registry(path: &str) -> Result<Vec<u8>, RegistryError> {
    let result = raw_read_file(path);
    match result {
        Ok(buffer) => Ok(buffer),
        Err(err) => {
            error!("[registry] Failed to read registry file {path}, error: {err:?}");
            Err(RegistryError::ReadRegistry)
        }
    }
}

/// Read the `Registry` file provided at file reference
pub(crate) fn read_registry_ref(
    ntfs_ref: &NtfsFileReference,
    ntfs_parser: &mut NtfsParser,
) -> Result<Vec<u8>, RegistryError> {
    let result = raw_read_by_file_ref(ntfs_ref, &ntfs_parser.ntfs, &mut ntfs_parser.fs);
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
    let empty = 0;
    if sk_offset < empty {
        error!("[registry] Provided unallocated offset. Refusing to parse SK data.");
        return Err(RegistryError::ReadRegistry);
    }
    let adjust_offset = 4096;
    // Since we are jumping straight to the SK offset we need to add 4096 to skip the HBIN header
    let offset = sk_offset + adjust_offset;
    let reg_data = read_registry(path)?;

    let sk_result = SecurityKey::parse_security_key(&reg_data, offset as u32);
    let sk = if let Ok((_, result)) = sk_result {
        result
    } else {
        error!("[registry] Could not parse Security info at offset {sk_offset}");
        return Err(RegistryError::Parser);
    };
    Ok(sk)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{
        get_registry_keys, get_registry_keys_by_ref, parse_raw_registry, read_registry,
        read_registry_ref,
    };
    use crate::{
        artifacts::os::windows::registry::{helper::lookup_sk_info, parser::Params},
        filesystem::ntfs::{raw_files::get_user_registry_files, setup::setup_ntfs_parser},
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_read_registry() {
        let buffer = read_registry("C:\\Windows\\appcompat\\Programs\\Amcache.hve").unwrap();
        assert!(buffer.len() > 10000)
    }

    #[test]
    fn test_parse_raw_registry() {
        let buffer = read_registry("C:\\Windows\\appcompat\\Programs\\Amcache.hve").unwrap();
        let mut params = Params {
            start_path: String::from("{"),
            path_regex: Regex::new("").unwrap(),
            registry_list: Vec::new(),
            key_tracker: Vec::new(),
            offset_tracker: HashMap::new(),
            filter: false,
            registry_path: String::new(),
        };
        let (_, result) = parse_raw_registry(&buffer, &mut params).unwrap();
        assert!(result.len() > 100)
    }

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
        let user_hives = get_user_registry_files(&'C').unwrap();
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();
        for hive in user_hives {
            if hive.filename != "NTUSER.DAT" {
                continue;
            }
            let result = get_registry_keys_by_ref(
                "",
                &Regex::new("").unwrap(),
                &hive.reg_reference,
                &mut ntfs_parser,
            )
            .unwrap();
            assert!(result.len() > 10);
            break;
        }
    }

    #[test]
    fn test_read_registry_ref() {
        let user_hives = get_user_registry_files(&'C').unwrap();
        let mut ntfs_parser = setup_ntfs_parser(&'C').unwrap();
        for hive in user_hives {
            if hive.filename != "NTUSER.DAT" {
                continue;
            }
            let result = read_registry_ref(&hive.reg_reference, &mut ntfs_parser).unwrap();
            assert!(result.len() > 10);
            break;
        }
    }
}
