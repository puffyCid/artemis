use super::{
    error::RegistryError, hbin::HiveBin, header::RegHeader, keys::sk::SecurityKey, parser::Params,
};
use crate::{
    accessor::{access::Accessor, entry::handle::FileHandle},
    output::manager::OutputManager,
    structs::artifacts::os::windows::RegistryOptions,
};
use common::windows::RegistryData;
use nom::bytes::complete::take;
use regex::Regex;
use std::collections::HashMap;
use tracing::error;

/// Parse provided `Registry` file at starting Key path and apply any optional Key path regex filtering
/// Use `get_registry_keys_handle` if you want to provide a `Registry` file handle
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
        registry_path: file_path.to_string(),
    };
    let buffer = read_registry(file_path)?;
    let reg_entries_results = parse_raw_registry(&buffer, &mut params, &mut None, None);
    match reg_entries_results {
        Ok((_, results)) => Ok(results),
        Err(_err) => {
            error!("Failed to parse registry file {file_path}");
            Err(RegistryError::Parser)
        }
    }
}

/// Parse the provided `Registry` bytes with associated parsing parameters.
/// Provide an optional `OutputManager` structure if you want artemis to stream the Registry output to disk.
/// Caller will need to handle any leftover `Params.registry_list` data remaining from the stream
pub(crate) fn parse_raw_registry<'a>(
    data: &'a [u8],
    params: &mut Params,
    manager: &mut Option<&mut OutputManager>,
    options: Option<&RegistryOptions>,
) -> nom::IResult<&'a [u8], Vec<RegistryData>> {
    let (input, header) = RegHeader::parse_header(data)?;

    let (_, reg_data) = take(header.hive_bins_size)(input)?;
    let (_, result) = HiveBin::parse_hive_bin_header(reg_data)?;
    let (input, hbin_data) = take(result.size)(reg_data)?;

    let (_, result) = HiveBin::parse_hive_cells(
        reg_data,
        hbin_data,
        params,
        header.minor_version,
        manager,
        options,
    )?;

    Ok((input, result))
}

/// Read the `Registry` file provided at path
pub(crate) fn read_registry(path: &str) -> Result<Vec<u8>, RegistryError> {
    // Use our accessor to read the provided Registry path
    let bytes = match Accessor::with_defaults().read_file(path) {
        Ok(buffer) => buffer,
        Err(err) => {
            error!("Failed to read registry file {path}, error: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };

    Ok(bytes)
}

/// Read the `Registry` file provided at file handle
pub(crate) fn read_registry_handle(handle: &FileHandle) -> Result<Vec<u8>, RegistryError> {
    // Use our accessor to read the provided Registry path
    let bytes = match Accessor::with_defaults().read_file_handle(handle) {
        Ok(buffer) => buffer,
        Err(err) => {
            error!(
                "Failed to read registry file handle {}, error: {err:?}",
                handle.display_path()
            );
            return Err(RegistryError::ReadRegistry);
        }
    };

    Ok(bytes)
}

/// Parse provided `Registry` `FileHandle` at starting Key path and apply any optional Key path regex filtering
/// Use `get_registry_keys` if you want to provide a `Registry` file path
pub(crate) fn get_registry_keys_handle(
    start_path: String,
    regex: Regex,
    file_handle: &FileHandle,
) -> Result<Vec<RegistryData>, RegistryError> {
    let mut params = Params {
        start_path: start_path,
        path_regex: regex,
        registry_list: Vec::new(),
        key_tracker: Vec::new(),
        offset_tracker: HashMap::new(),
        registry_path: file_handle.display_path(),
    };
    let buffer = read_registry_handle(file_handle)?;
    let reg_entries_results = parse_raw_registry(&buffer, &mut params, &mut None, None);
    match reg_entries_results {
        Ok((_, results)) => Ok(results),
        Err(_err) => {
            error!(
                "Failed to parse registry file {}",
                file_handle.display_path()
            );
            Err(RegistryError::Parser)
        }
    }
}

/// Lookup Security Key info based on SK offset.
pub(crate) fn lookup_sk_info(path: &str, sk_offset: i32) -> Result<SecurityKey, RegistryError> {
    let empty = 0;
    if sk_offset < empty {
        error!("Provided unallocated offset. Refusing to parse SK data.");
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
        error!("Could not parse Security info at offset {sk_offset}");
        return Err(RegistryError::Parser);
    };
    Ok(sk)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_registry_keys, parse_raw_registry, read_registry};
    use crate::{
        accessor::entry::handle::FileHandle,
        artifacts::os::windows::registry::{
            helper::{get_registry_keys_handle, lookup_sk_info},
            parser::Params,
        },
    };
    use regex::Regex;
    use std::{collections::HashMap, path::PathBuf};

    #[test]
    fn test_read_registry() {
        let test = [
            "ntfs:C:\\Windows\\appcompat\\Programs\\Amcache.hve",
            "ntfs:C:\\Windows\\AppCompat\\Programs\\Amcache.hve",
        ];
        let mut pass = false;
        for entry in test {
            let buffer = read_registry(entry).unwrap_or_default();
            if buffer.is_empty() {
                continue;
            }

            assert!(buffer.len() > 1);
            pass = true;
            break;
        }

        assert!(pass)
    }

    #[test]
    fn test_parse_raw_registry() {
        let test = [
            "ntfs:C:\\Windows\\appcompat\\Programs\\Amcache.hve",
            "ntfs:C:\\Windows\\AppCompat\\Programs\\Amcache.hve",
        ];

        let mut pass = false;
        for entry in test {
            let buffer = read_registry(entry).unwrap_or_default();
            let mut params = Params {
                start_path: String::from("{"),
                path_regex: Regex::new("").unwrap(),
                registry_list: Vec::new(),
                key_tracker: Vec::new(),
                offset_tracker: HashMap::new(),
                registry_path: String::new(),
            };
            let result = parse_raw_registry(&buffer, &mut params, &mut None, None);
            if result.is_err() {
                continue;
            }
            assert!(result.unwrap().1.len() > 100);
            pass = true;

            break;
        }
        assert!(pass)
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
        assert_eq!(result[0].last_modified, "2019-12-07T09:16:14.599Z");
        assert_eq!(result[0].depth, 4);
    }

    #[test]
    fn test_get_registry_keys_handle() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER.DAT");
        let start_path = String::from("ROOT\\SOFTWARE\\Microsoft\\");
        let regex = Regex::new(r".*\\typedurls").unwrap();
        let result =
            get_registry_keys_handle(start_path, regex, &FileHandle::host(test_location)).unwrap();
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
        assert_eq!(result[0].last_modified, "2019-12-07T09:16:14.599Z");
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
        // This Registry contains an infinite loop. An offset points to parent offset
        test_location.push("tests\\test_data\\windows\\registry\\win10\\NTUSER_Bad.DAT");
        let start_path = "";
        let regex = Regex::new("").unwrap();
        let result =
            get_registry_keys(start_path, &regex, &test_location.display().to_string()).unwrap();
        // The infinite loop causes the parser to skip two values
        assert_eq!(result.len(), 664);
    }
}
