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
use super::{commands::command::Commands, error::MachoError, fat::FatHeader, header::MachoHeader};
use crate::filesystem::files::{file_reader, file_too_large};
use common::macos::MachoInfo;
use log::error;
use std::io::{Read, Seek, SeekFrom};

/// Parse a macho file
pub(crate) fn parse_macho(path: &str) -> Result<Vec<MachoInfo>, MachoError> {
    let reader_result = file_reader(path);
    let mut reader = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[macho] Could not get file reader for {path}. Error: {err:?}");
            return Err(MachoError::Path);
        }
    };

    let mut buff = [0; 4];

    if reader.read(&mut buff).is_err() {
        return Err(MachoError::Buffer);
    }
    let fat_binary = [202, 254, 186, 190];
    let binary = [250, 237, 254];

    if buff != fat_binary && !buff.ends_with(&binary) {
        return Err(MachoError::Magic);
    }

    if reader.seek(SeekFrom::Start(0)).is_err() {
        return Err(MachoError::Buffer);
    }

    if file_too_large(path) {
        return Err(MachoError::Buffer);
    }

    let mut data = Vec::new();

    // Allow File read_to_end because we partially read the file above to check for Magic Header
    #[allow(clippy::verbose_file_reads)]
    let data_result = reader.read_to_end(&mut data);
    match data_result {
        Ok(_) => {}
        Err(_) => return Err(MachoError::Buffer),
    };

    let min_header_len = 4;
    let min_size = 100;
    if data.len() < min_header_len && data.len() < min_size {
        return Ok(Vec::new());
    }

    let is_fat_results = FatHeader::is_fat(&data);
    let is_fat = match is_fat_results {
        Ok((_, result)) => result,
        Err(_err) => {
            return Err(MachoError::Header);
        }
    };

    let mut macho_info: Vec<MachoInfo> = Vec::new();
    if is_fat {
        let fat_header_results = parse_fat(&data);
        let fat_header = match fat_header_results {
            Ok(result) => result,
            Err(_err) => {
                return Ok(macho_info);
            }
        };
        // Parse in another function returning MachoInfo
        for arch in fat_header.archs {
            let binary_data_results = MachoHeader::binary_start(&data, arch.offset, arch.size);
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

            let commands_results = parse_commands(command_data, &header_data, binary_data);
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
    let is_macho_results = MachoHeader::is_macho(&data);

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

    let header_results = MachoHeader::parse_header(&data);
    let (command_data, header_data) = match header_results {
        Ok((command, result)) => (command, result),
        Err(err) => {
            error!("[macho] Failed to parse MACHO header: {err:?}");
            return Err(MachoError::Header);
        }
    };

    let commands_results = parse_commands(command_data, &header_data, &data);
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

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::MachoInfo;
    use crate::{
        artifacts::os::macos::macho::parser::parse_macho, filesystem::directory::is_directory,
    };
    use std::path::PathBuf;
    use walkdir::WalkDir;

    #[test]
    fn test_parse_macho() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/macho/fat/ls");

        let result = parse_macho(&test_location.display().to_string()).unwrap();
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
        for entries in begin_walk.into_iter() {
            let entry_data = entries.unwrap();
            if !entry_data.metadata().unwrap().is_file() || entry_data.file_name() == "sudo" {
                continue;
            }
            let _ = parse_macho(&entry_data.path().display().to_string());
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

        for entries in begin_walk.into_iter() {
            let entry_data = entries.unwrap();

            let _ = parse_macho(&entry_data.path().display().to_string());
        }
    }

    #[test]
    fn test_all_apps() {
        let start_walk = WalkDir::new("/Applications").same_file_system(true);
        let begin_walk = start_walk.max_depth(4);
        let mut results: Vec<MachoInfo> = Vec::new();

        for entries in begin_walk.into_iter() {
            let entry_data = entries.unwrap();
            if !entry_data.metadata().unwrap().is_file() {
                continue;
            }

            let macho_result = parse_macho(&entry_data.path().display().to_string());
            match macho_result {
                Ok(mut result) => results.append(&mut result),
                Err(_) => continue,
            }
        }
        assert!(results.len() > 12);
    }
}
