/**
 * Amcache stores metadata related to execution of Windows applications.  
 * Data is stored in the Amcache.hve Registry file. It also contains other metadata such as OS, hardware, and application info
 *
 * References:  
 *   `https://github.com/libyal/dtformats/blob/main/documentation/AMCache%20file%20(AMCache.hve)%20format.asciidoc`  
 *   `https://www.ssi.gouv.fr/uploads/2019/01/anssi-coriin_2019-analysis_amcache.pdf`
 *
 * Other parsers:  
 *   `https://f001.backblazeb2.com/file/EricZimmermanTools/RegistryExplorer.zip`  
 *   `https://github.com/Velocidex/velociraptor`
 */
use super::error::AmcacheError;
use crate::{
    artifacts::os::{
        systeminfo::info::get_win_kernel_version, windows::registry::helper::get_registry_keys,
    },
    structs::artifacts::os::windows::AmcacheOptions,
    utils::{environment::get_systemdrive, regex_options::create_regex},
};
use common::windows::{Amcache, RegistryEntry};
use log::error;

/// Get Windows `Amcache` for all users based on optional drive, otherwise default drive letter is used
pub(crate) fn grab_amcache(options: &AmcacheOptions) -> Result<Vec<Amcache>, AmcacheError> {
    if let Some(alt_drive) = options.alt_drive {
        return alt_drive_amcache(&alt_drive);
    }
    default_amcache()
}

/// Get the default driver letter and parse the `Amcache`
fn default_amcache() -> Result<Vec<Amcache>, AmcacheError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[amcache] Could not get default systemdrive letter: {err:?}");
            return Err(AmcacheError::DefaultDrive);
        }
    };
    amcache_file(&drive)
}

/// Parse `Amcache` associated with provided alternative driver letter
fn alt_drive_amcache(drive: &char) -> Result<Vec<Amcache>, AmcacheError> {
    amcache_file(drive)
}

/// Based on Windows version get the path to `Amcache` file
fn amcache_file(drive: &char) -> Result<Vec<Amcache>, AmcacheError> {
    let kernel_version = get_win_kernel_version();
    let win10 = 10240.0;
    let path = if kernel_version < win10 {
        format!("{drive}:\\Windows\\AppCompat\\Programs\\Amcache.hve")
    } else {
        format!("{drive}:\\Windows\\appcompat\\Programs\\Amcache.hve")
    };

    parse_amcache(&path)
}

/**
 * `Amcache` is typically stored at the Registry file `C:\Windows\appcompat\Promgrams\Amcache.hve`
 * Parse the raw Registry file and get the entries related to file execution
 */
fn parse_amcache(path: &str) -> Result<Vec<Amcache>, AmcacheError> {
    let start_path = "";
    // Should always be valid
    let path_regex = create_regex(r"root\\(inventoryapplicationfile|file)\\.*").unwrap();

    let amcache_result = get_registry_keys(start_path, &path_regex, path);
    let amcache = match amcache_result {
        Ok(result) => result,
        Err(err) => {
            error!("[amcache] Could not parse Amcache file: {err:?}");
            return Err(AmcacheError::GetRegistryData);
        }
    };

    let mut amcache_vec: Vec<Amcache> = Vec::new();
    for entry in amcache {
        let mut amcache_entry = Amcache {
            first_execution: entry.last_modified,
            path: String::new(),
            name: String::new(),
            original_name: String::new(),
            version: String::new(),
            binary_type: String::new(),
            product_version: String::new(),
            product_name: String::new(),
            language: String::new(),
            file_id: String::new(),
            link_date: String::new(),
            path_hash: String::new(),
            program_id: String::new(),
            publisher: String::new(),
            usn: String::new(),
            size: String::new(),
            sha1: String::new(),
            reg_path: entry.path.clone(),
        };

        let old_path_depth = 5;
        if entry.path.contains("Root\\File\\") && entry.path.split('\\').count() == old_path_depth {
            extract_old_path(entry, &mut amcache_entry);
        } else if entry.path.contains("InventoryApplicationFile") {
            extract_entry(entry, &mut amcache_entry);
        } else {
            continue;
        }

        amcache_vec.push(amcache_entry);
    }
    Ok(amcache_vec)
}

/// Older versions of `Amcache` (Windows 8 and 8.1) used numbers to represent Value names
fn extract_old_path(entry: RegistryEntry, amcache_entry: &mut Amcache) {
    for value in entry.values {
        match value.value.as_str() {
            "0" => amcache_entry.product_name = value.data,
            "1" => amcache_entry.publisher = value.data,
            "2" => amcache_entry.product_version = value.data,
            "3" => amcache_entry.language = value.data,
            "5" => amcache_entry.version = value.data,
            "6" => amcache_entry.size = value.data,
            "f" => amcache_entry.link_date = value.data,
            "15" => amcache_entry.path = value.data,
            "100" => {
                let extra_zeros = 3;
                amcache_entry.program_id = adjust_id(&value.data, extra_zeros);
            }
            "101" => {
                let extra_zeros = 4;
                amcache_entry.sha1 = adjust_id(&value.data, extra_zeros);
            }

            _ => continue,
        }
    }
}

/// Modern versions of `Amcache` have regular Value names
fn extract_entry(entry: RegistryEntry, amcache_entry: &mut Amcache) {
    // if entry.path contains \\File\\ parse as number or something
    for value in entry.values {
        match value.value.as_str() {
            "Langague" => amcache_entry.language = value.data,
            "LinkDate" => amcache_entry.link_date = value.data,
            "LongPathHash" => amcache_entry.path_hash = value.data,
            "LowerCaseLongPath" => amcache_entry.path = value.data,
            "Name" => amcache_entry.name = value.data,
            "OriginalFileName" => amcache_entry.original_name = value.data,
            "ProductName" => amcache_entry.product_name = value.data,
            "ProductVersion" => amcache_entry.product_version = value.data,
            "ProgramId" => {
                let extra_zeros = 3;
                amcache_entry.program_id = adjust_id(&value.data, extra_zeros);
            }
            "Publisher" => amcache_entry.publisher = value.data,
            "Size" => amcache_entry.size = value.data,
            "Usn" => amcache_entry.usn = value.data,
            "Version" => amcache_entry.version = value.data,
            "FileId" => {
                let extra_zeros = 4;
                amcache_entry.file_id = adjust_id(&value.data, extra_zeros);
                amcache_entry.sha1 = adjust_id(&value.data, extra_zeros);
            }
            "BinaryType" => amcache_entry.binary_type = value.data,
            _ => continue,
        }
    }
}

/// The IDs associated related to `Amcache` (`ProgramId` and `FileId`) have extra zeros prepended to them.
fn adjust_id(id: &str, count: usize) -> String {
    if id.len() < count {
        return id.to_string();
    }

    id[count..].to_string()
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::{
            amcache::parser::{
                adjust_id, alt_drive_amcache, amcache_file, default_amcache, extract_entry,
                extract_old_path, grab_amcache, parse_amcache, Amcache,
            },
            registry::helper::get_registry_keys,
        },
        structs::artifacts::os::windows::AmcacheOptions,
        utils::regex_options::create_regex,
    };
    use std::path::PathBuf;

    #[test]
    fn test_default_amcache() {
        let result = default_amcache().unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_alt_drive_amcache() {
        let result = alt_drive_amcache(&'C').unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_grab_amcache() {
        let options = AmcacheOptions { alt_drive: None };
        let result = grab_amcache(&options).unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_amcache_file() {
        let result = amcache_file(&'C').unwrap();
        assert!(result.len() > 10);
    }

    #[test]
    fn test_parse_amcache() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\amcache\\win81\\Amcache.hve");
        let result = parse_amcache(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 4);

        assert_eq!(result[0].first_execution, 1673412178);
        assert_eq!(
            result[0].path,
            "C:\\Users\\bob\\Documents\\artemis-core\\target\\release\\examples\\artemis_core.exe"
        );
        assert_eq!(result[0].language, "0");
        assert_eq!(result[0].link_date, "1673412152");
        assert_eq!(result[0].size, "5188608");
        assert_eq!(result[0].sha1, "8c55942db046700a0ccddea067e3a6e3cc259424");
        assert_eq!(result[0].reg_path, "{11517B7C-E79D-4e20-961B-75A811715ADD}\\Root\\File\\8195d9c8-2089-11ea-824e-806e6f6e6963\\20000667bc");

        assert_eq!(result[3].first_execution, 1673413026);
        assert_eq!(
            result[3].path,
            "c:\\program files (x86)\\microsoft\\edge\\application\\msedge.exe"
        );
        assert_eq!(result[3].name, "msedge.exe");
        assert_eq!(result[3].original_name, "msedge.exe");
        assert_eq!(result[3].version, "108.0.1462.76");
        assert_eq!(result[3].product_version, "108.0.1462.76");
        assert_eq!(result[3].binary_type, "pe64_amd64");
        assert_eq!(result[3].product_name, "microsoft edge");
        assert_eq!(result[3].language, "");
        assert_eq!(
            result[3].file_id,
            "57f7a64c05fbc31830754108ccb6f65bd6c0f9bc"
        );
        assert_eq!(result[3].link_date, "01/04/2023 23:15:18");
        assert_eq!(result[3].path_hash, "msedge.exe|d27b57360cd4a4cf");
        assert_eq!(
            result[3].program_id,
            "66afc7e33c2fa0155f7f4969e8f4ea64b00000904"
        );
        assert_eq!(result[3].size, "3879368");
        assert_eq!(result[3].publisher, "microsoft corporation");
        assert_eq!(result[3].usn, "1570250352");
        assert_eq!(result[3].sha1, "57f7a64c05fbc31830754108ccb6f65bd6c0f9bc");
        assert_eq!(result[3].reg_path, "{11517B7C-E79D-4e20-961B-75A811715ADD}\\Root\\InventoryApplicationFile\\msedge.exe|d27b57360cd4a4cf");
    }

    #[test]
    fn test_adjust_id() {
        let test = "000aaaaa";
        let count = 3; // start at zero
        let result = adjust_id(&test, count);
        assert_eq!(result, "aaaaa");
    }

    #[test]
    fn test_extract_old_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\amcache\\win81\\Amcache.hve");

        let start_path = "";
        let path_regex = create_regex(r"root\\(inventoryapplicationfile|file)\\.*").unwrap();
        let amcache = get_registry_keys(
            start_path,
            &path_regex,
            &test_location.display().to_string(),
        )
        .unwrap();
        let mut amcache_vec: Vec<Amcache> = Vec::new();
        for entry in amcache {
            let mut amcache_entry = Amcache {
                first_execution: entry.last_modified,
                path: String::new(),
                name: String::new(),
                original_name: String::new(),
                version: String::new(),
                binary_type: String::new(),
                product_version: String::new(),
                product_name: String::new(),
                language: String::new(),
                file_id: String::new(),
                link_date: String::new(),
                path_hash: String::new(),
                program_id: String::new(),
                publisher: String::new(),
                usn: String::new(),
                size: String::new(),
                sha1: String::new(),
                reg_path: entry.path.clone(),
            };

            let old_path_depth = 5;
            if entry.path.contains("Root\\File\\")
                && entry.path.split('\\').collect::<Vec<&str>>().len() == old_path_depth
            {
                extract_old_path(entry, &mut amcache_entry)
            } else {
                continue;
            }
            amcache_vec.push(amcache_entry);
        }
        assert_eq!(amcache_vec.len(), 2);
    }

    #[test]
    fn test_extract_entry() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\amcache\\win81\\Amcache.hve");

        let start_path = "";
        let path_regex = create_regex(r"root\\(inventoryapplicationfile|file)\\.*").unwrap();
        let amcache = get_registry_keys(
            start_path,
            &path_regex,
            &test_location.display().to_string(),
        )
        .unwrap();
        let mut amcache_vec: Vec<Amcache> = Vec::new();
        for entry in amcache {
            let mut amcache_entry = Amcache {
                first_execution: entry.last_modified,
                path: String::new(),
                name: String::new(),
                original_name: String::new(),
                version: String::new(),
                binary_type: String::new(),
                product_version: String::new(),
                product_name: String::new(),
                language: String::new(),
                file_id: String::new(),
                link_date: String::new(),
                path_hash: String::new(),
                program_id: String::new(),
                publisher: String::new(),
                usn: String::new(),
                size: String::new(),
                sha1: String::new(),
                reg_path: entry.path.clone(),
            };

            if entry.path.contains("InventoryApplicationFile") {
                extract_entry(entry, &mut amcache_entry)
            } else {
                continue;
            }
            amcache_vec.push(amcache_entry);
        }
        assert_eq!(amcache_vec.len(), 2);
    }
}
