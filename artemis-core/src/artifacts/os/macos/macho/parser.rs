/**
 * macOS `Macho` is the native executable format of macOS programs
 * We currently parse out a basic amount of `Macho` information
 *
 * References:  
 *   `https://github.com/aidansteele/osx-abi-macho-file-format-reference`
 *
 * Other Parsers:  
 *   `https://github.com/radareorg/radare2`  
 *   `https://lief-project.github.io/`
 */
use super::{
    commands::{command::Commands, dylib::DylibCommand, segments::Segment64},
    error::MachoError,
    fat::FatHeader,
    header::MachoHeader,
};
use log::error;
use plist::Dictionary;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct MachoInfo {
    pub(crate) cpu_type: String,
    pub(crate) cpu_subtype: String,
    pub(crate) filetype: String,
    pub(crate) segments: Vec<Segment64>,
    pub(crate) dylib_command: Vec<DylibCommand>,
    pub(crate) id: String,
    pub(crate) team_id: String,
    pub(crate) entitlements: Dictionary,
    pub(crate) certs: String,
    pub(crate) minos: String,
    pub(crate) sdk: String,
}

impl MachoInfo {
    /// Parse a macho file
    pub(crate) fn parse_macho(data: &[u8]) -> Result<Vec<MachoInfo>, MachoError> {
        let min_header_len = 4;
        let min_size = 100;
        if data.len() < min_header_len && data.len() < min_size {
            return Ok(Vec::new());
        }

        let is_fat_results = FatHeader::is_fat(data);
        let is_fat = match is_fat_results {
            Ok((_, result)) => result,
            Err(_err) => {
                return Err(MachoError::Header);
            }
        };

        let mut macho_info: Vec<MachoInfo> = Vec::new();
        if is_fat {
            let fat_header_results = MachoInfo::parse_fat(data);
            let fat_header = match fat_header_results {
                Ok(result) => result,
                Err(_err) => {
                    return Ok(macho_info);
                }
            };
            // Parse in another function returning MachoInfo
            for arch in fat_header.archs {
                let binary_data_results = MachoHeader::binary_start(data, arch.offset, arch.size);
                let binary_data = match binary_data_results {
                    Ok((_, results)) => results,
                    Err(_err) => {
                        continue;
                    }
                };
                let header_results = MachoHeader::parse_header(binary_data);
                let (command_data, header_data) = match header_results {
                    Ok((command, result)) => (command, result),
                    Err(_err) => {
                        continue;
                    }
                };

                let commands_results =
                    MachoInfo::parse_commands(command_data, &header_data, binary_data);
                let commands = match commands_results {
                    Ok(results) => results,
                    Err(_err) => {
                        return Err(MachoError::Data);
                    }
                };

                let macho_data = MachoInfo {
                    cpu_type: header_data.cpu_type,
                    cpu_subtype: header_data.cpu_subtype,
                    filetype: header_data.filetype,
                    segments: commands.segments64,
                    dylib_command: commands.dylib_commands,
                    id: commands.code_sign.directory.id,
                    team_id: commands.code_sign.directory.team_id,
                    entitlements: commands.code_sign.entitlements,
                    certs: commands.code_sign.certs,
                    minos: commands.build_system.minos,
                    sdk: commands.build_system.sdk,
                };
                macho_info.push(macho_data);
            }
            return Ok(macho_info);
        }
        let is_macho_results = MachoHeader::is_macho(data);

        let is_macho = match is_macho_results {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[macho] Failed to check macho magic number: {err:?}");
                return Err(MachoError::Magic);
            }
        };
        if !is_macho {
            return Ok(macho_info);
        }

        let header_results = MachoHeader::parse_header(data);
        let (command_data, header_data) = match header_results {
            Ok((command, result)) => (command, result),
            Err(err) => {
                error!("[macho] Failed to parse MACHO header: {err:?}");
                return Err(MachoError::Header);
            }
        };

        let commands_results = MachoInfo::parse_commands(command_data, &header_data, data);
        let commands = match commands_results {
            Ok(results) => results,
            Err(err) => {
                return Err(err);
            }
        };
        let macho_data = MachoInfo {
            cpu_type: header_data.cpu_type,
            cpu_subtype: header_data.cpu_subtype,
            filetype: header_data.filetype,
            segments: commands.segments64,
            dylib_command: commands.dylib_commands,
            id: commands.code_sign.directory.id,
            team_id: commands.code_sign.directory.team_id,
            entitlements: commands.code_sign.entitlements,
            certs: commands.code_sign.certs,
            minos: commands.build_system.minos,
            sdk: commands.build_system.sdk,
        };
        macho_info.push(macho_data);

        Ok(macho_info)
    }

    /// Parse a FAT macho file. Contains two or more binaries
    fn parse_fat(data: &[u8]) -> Result<FatHeader, MachoError> {
        let fat_results = FatHeader::parse_header(data);
        let fat_header = match fat_results {
            Ok((_, result)) => result,
            Err(_err) => {
                return Err(MachoError::FatHeader);
            }
        };
        Ok(fat_header)
    }

    /// Parse the commands following the macho header
    fn parse_commands(
        command_data: &[u8],
        header_data: &MachoHeader,
        binary_data: &[u8],
    ) -> Result<Commands, MachoError> {
        let command_results =
            Commands::parse_commands(header_data.number_commands, command_data, binary_data);

        let commands = match command_results {
            Ok((_, results)) => results,
            Err(_err) => {
                return Err(MachoError::Data);
            }
        };

        Ok(commands)
    }
}

#[cfg(test)]
mod tests {
    use super::MachoInfo;
    use crate::filesystem::{directory::is_directory, files::read_file};
    use std::path::PathBuf;
    use walkdir::WalkDir;

    #[test]
    fn test_parse_macho() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/macho/fat/ls");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let result = MachoInfo::parse_macho(&buffer).unwrap();
        assert_eq!(result[0].certs.len(), 5924);
        assert_eq!(result[0].minos, "10.14.0");
        assert_eq!(result[0].sdk, "12.3.0");
        assert_eq!(result[0].cpu_subtype, "386");
        assert_eq!(result[0].cpu_type, "X86_64");
        assert_eq!(result[0].id, "com.apple.ls");
    }

    #[test]
    fn test_all_bin() {
        let start_walk = WalkDir::new("/usr/bin").same_file_system(true);
        let begin_walk = start_walk.max_depth(20);
        let mut results: Vec<MachoInfo> = Vec::new();
        for entries in begin_walk.into_iter() {
            let entry_data = entries.unwrap();
            if !entry_data.metadata().unwrap().is_file() || entry_data.file_name() == "sudo" {
                continue;
            }
            let buffer = read_file(&entry_data.path().display().to_string()).unwrap();
            let mut result = MachoInfo::parse_macho(&buffer).unwrap();
            results.append(&mut result)
        }
    }

    #[test]
    fn test_homebrew() {
        let mut path = "/usr/local/Cellar";
        if !is_directory(path) {
            path = "/opt/homebrew/Cellar";
        }
        let start_walk = WalkDir::new(path).same_file_system(true);
        let begin_walk = start_walk.max_depth(20);
        let mut results: Vec<MachoInfo> = Vec::new();

        for entries in begin_walk.into_iter() {
            let entry_data = entries.unwrap();

            if !entry_data.metadata().unwrap().is_file()
                || entry_data.path().extension().unwrap_or_default() == "class"
                || entry_data.path().extension().unwrap_or_default() == "a"
            {
                continue;
            }

            let buffer = read_file(&entry_data.path().display().to_string()).unwrap();
            let mut result = MachoInfo::parse_macho(&buffer).unwrap();
            results.append(&mut result)
        }
        assert!(results.len() > 12);
    }

    #[test]
    #[ignore = "requires root for xcode"]
    fn test_all_apps() {
        let start_walk = WalkDir::new("/Applications").same_file_system(true);
        let begin_walk = start_walk.max_depth(20);
        let mut results: Vec<MachoInfo> = Vec::new();

        for entries in begin_walk.into_iter() {
            let entry_data = entries.unwrap();
            if !entry_data.metadata().unwrap().is_file() {
                continue;
            }

            let buffer = read_file(&entry_data.path().display().to_string()).unwrap();
            //let mut result = MachoInfo::parse_macho(&buffer).unwrap();
            //results.append(&mut result);

            let macho_result = MachoInfo::parse_macho(&buffer);
            match macho_result {
                Ok(mut result) => results.append(&mut result),
                Err(_) => continue,
            }
        }
        assert!(results.len() > 12);
    }
}
