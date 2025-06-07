use super::{
    build::Build,
    codesign::{CodeDirectory, CodeSign},
    dylib::parse_dylyb_command,
    segments::{parse_segment32, parse_segment64},
};
use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    uuid::format_guid_be_bytes,
};
use common::macos::{DylibCommand, Segment64};
use log::error;
use nom::bytes::complete::take;
use plist::Dictionary;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Commands {
    pub(crate) segments64: Vec<Segment64>,
    pub(crate) segments32: Vec<Segment64>,
    pub(crate) dylib_commands: Vec<DylibCommand>,
    pub(crate) build_system: Build,
    pub(crate) uuid: String,
    pub(crate) code_sign: CodeSign,
}

impl Commands {
    /// Parse all the commands that are in a macho binary
    pub(crate) fn parse_commands<'a>(
        number_cmds: u32,
        binary_data: &'a [u8],
        original_macho_data: &'a [u8],
    ) -> nom::IResult<&'a [u8], Commands> {
        let mut cmd_count = 0;
        let mut commands = Commands {
            segments64: Vec::new(),
            segments32: Vec::new(),
            dylib_commands: Vec::new(),
            uuid: String::new(),
            build_system: Build {
                platform: 0,
                minos: String::new(),
                sdk: String::new(),
                ntools: 0,
            },
            code_sign: CodeSign {
                directory: CodeDirectory {
                    id: String::new(),
                    team_id: String::new(),
                    flags: 0,
                    magic: 0,
                    length: 0,
                    version: 0,
                    hash_offset: 0,
                    ident_offset: 0,
                    n_special_slots: 0,
                    n_code_slots: 0,
                    code_limit: 0,
                    hash_size: 0,
                    hash_type: 0,
                    platform: 0,
                    page_size: 0,
                    spare2: 0,
                    hash_pages: Vec::new(),
                },
                entitlements: Dictionary::new(),
                embedded_entitlements: String::new(),
                certs: String::new(),
            },
        };

        let mut macho_data = binary_data;
        while cmd_count < number_cmds && !macho_data.is_empty() {
            let (command_data, cmd_type) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
            let (command_data, size) = nom_unsigned_four_bytes(command_data, Endian::Le)?;
            let cmd_meta = 8;

            // Size includes cmd and cmd_size
            let (data, command_data) = take(size - cmd_meta)(command_data)?;
            macho_data = data;
            match cmd_type {
                0x19 => Commands::get_segment64(command_data, &mut commands.segments64),
                0x1 => Commands::get_segment32(command_data, &mut commands.segments32),
                0x1b => Commands::get_uuid(command_data, &mut commands),
                0xc => Commands::get_dylib_command(command_data, &mut commands.dylib_commands),
                0x32 => Commands::get_build(command_data, &mut commands),
                0x1d => {
                    let codesign_start_result =
                        Commands::get_codesign_start(command_data, original_macho_data);
                    match codesign_start_result {
                        Ok((_, result)) => Commands::get_codesignature(result, &mut commands),
                        Err(_err) => {}
                    }
                }
                // Not all command types are supported. https://github.com/aidansteele/osx-abi-macho-file-format-reference#table-4-mach-o-load-commands
                _ => {}
            }
            cmd_count += 1;
        }

        Ok((macho_data, commands))
    }

    /// Get `Segment64` command
    fn get_segment64(data: &[u8], segments: &mut Vec<Segment64>) {
        let segment_result = parse_segment64(data);
        match segment_result {
            Ok((_, result)) => segments.push(result),
            Err(err) => {
                error!("[macho] Failed to parse segment64: {err:?}");
            }
        }
    }

    /// Get `Segment32` command
    fn get_segment32(data: &[u8], segments: &mut Vec<Segment64>) {
        let segment_result = parse_segment32(data);
        match segment_result {
            Ok((_, result)) => segments.push(result),
            Err(err) => {
                error!("[macho] Failed to parse segment32: {err:?}");
            }
        }
    }

    /// Get `UUID` command
    fn get_uuid(data: &[u8], segments: &mut Commands) {
        segments.uuid = format_guid_be_bytes(data);
    }

    /// Get `DYLIB` command
    fn get_dylib_command(data: &[u8], dylib_commands: &mut Vec<DylibCommand>) {
        let segment_result = parse_dylyb_command(data);
        match segment_result {
            Ok((_, result)) => dylib_commands.push(result),
            Err(err) => {
                error!("[macho] Failed to parse DYLIB command: {err:?}");
            }
        }
    }

    /// Get `DYLIB` command
    fn get_build(data: &[u8], command: &mut Commands) {
        let segment_result = Build::parse_build_version(data);
        match segment_result {
            Ok((_, result)) => command.build_system = result,
            Err(err) => {
                error!("[macho] Failed to parse build system data {err:?}");
            }
        }
    }

    /// Get `CodeSignature` command
    fn get_codesignature(data: &[u8], command: &mut Commands) {
        let codesign_results = CodeSign::parse_codesign(data);
        match codesign_results {
            Ok((_, result)) => command.code_sign = result,
            Err(err) => {
                error!("[macho] Failed to parse code signature data {err:?}");
            }
        }
    }

    /// Get start of the codesignature data
    fn get_codesign_start<'a>(
        data: &'a [u8],
        macho_data: &'a [u8],
    ) -> nom::IResult<&'a [u8], &'a [u8]> {
        let (codesign_meta, codesign_offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (codesign_meta, codesign_size) = nom_unsigned_four_bytes(codesign_meta, Endian::Le)?;

        let (codesign_start, _) = take(codesign_offset)(macho_data)?;
        let (_, codesign_data) = take(codesign_size)(codesign_start)?;
        Ok((codesign_meta, codesign_data))
    }
}

#[cfg(test)]
mod tests {
    use common::macos::Segment64;

    use super::Commands;
    use crate::{
        artifacts::os::macos::macho::commands::dylib::parse_dylyb_command,
        utils::uuid::format_guid_be_bytes,
    };

    #[test]
    fn test_parse_commands_one_segment() {
        let test_data = [
            25, 0, 0, 0, 72, 0, 0, 0, 95, 95, 80, 65, 71, 69, 90, 69, 82, 79, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let cmds = 1;
        let (_, result) = Commands::parse_commands(cmds, &test_data, &test_data).unwrap();
        assert_eq!(result.segments64[0].name, "__PAGEZERO");
        assert_eq!(result.segments64[0].vmaddr, 0);
        assert_eq!(result.segments64[0].vmsize, 0x100000000);
        assert_eq!(result.segments64[0].file_offset, 0);
        assert_eq!(result.segments64[0].file_size, 0);
        assert_eq!(result.segments64[0].max_prot, 0);
        assert_eq!(result.segments64[0].init_prot, 0);
        assert_eq!(result.segments64[0].nsects, 0);
        assert_eq!(result.segments64[0].flags, 0);
    }

    #[test]
    fn test_get_codesign_start() {
        let test_data = vec![1, 0, 0, 0, 1, 0, 0, 0];

        let (_, results) = Commands::get_codesign_start(&test_data, &test_data).unwrap();
        assert_eq!(results, [0]);
    }

    #[test]
    fn test_parse_commands_multiple_segments() {
        let test_data = [
            25, 0, 0, 0, 72, 0, 0, 0, 95, 95, 80, 65, 71, 69, 90, 69, 82, 79, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 25, 0, 0, 0, 216, 1, 0, 0, 95,
            95, 84, 69, 88, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 128, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 5, 0, 0, 0,
            5, 0, 0, 0, 0, 0, 0, 0, 95, 95, 116, 101, 120, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95,
            95, 84, 69, 88, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 48, 56, 0, 0, 1, 0, 0, 0, 16, 60, 0,
            0, 0, 0, 0, 0, 48, 56, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 128, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 97, 117, 116, 104, 95, 115, 116, 117, 98, 115, 0, 0,
            0, 0, 95, 95, 84, 69, 88, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 64, 116, 0, 0, 1, 0, 0, 0,
            32, 5, 0, 0, 0, 0, 0, 0, 64, 116, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 4, 0,
            128, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 95, 95, 99, 111, 110, 115, 116, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 95, 95, 84, 69, 88, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 96, 121, 0, 0, 1,
            0, 0, 0, 220, 0, 0, 0, 0, 0, 0, 0, 96, 121, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 99, 115, 116, 114, 105, 110,
            103, 0, 0, 0, 0, 0, 0, 0, 95, 95, 84, 69, 88, 84, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 60,
            122, 0, 0, 1, 0, 0, 0, 233, 4, 0, 0, 0, 0, 0, 0, 60, 122, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 117, 110, 119, 105,
            110, 100, 95, 105, 110, 102, 111, 0, 0, 0, 95, 95, 84, 69, 88, 84, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 40, 127, 0, 0, 1, 0, 0, 0, 216, 0, 0, 0, 0, 0, 0, 0, 40, 127, 0, 0, 2, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 25, 0, 0, 0,
            56, 1, 0, 0, 95, 95, 68, 65, 84, 65, 95, 67, 79, 78, 83, 84, 0, 0, 0, 0, 0, 128, 0, 0,
            1, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0,
            3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 16, 0, 0, 0, 95, 95, 97, 117, 116, 104, 95, 103,
            111, 116, 0, 0, 0, 0, 0, 0, 95, 95, 68, 65, 84, 65, 95, 67, 79, 78, 83, 84, 0, 0, 0, 0,
            0, 128, 0, 0, 1, 0, 0, 0, 144, 2, 0, 0, 0, 0, 0, 0, 0, 128, 0, 0, 3, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 6, 0, 0, 0, 82, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 103, 111, 116,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 68, 65, 84, 65, 95, 67, 79, 78, 83, 84, 0, 0,
            0, 0, 144, 130, 0, 0, 1, 0, 0, 0, 48, 0, 0, 0, 0, 0, 0, 0, 144, 130, 0, 0, 3, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 164, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 99,
            111, 110, 115, 116, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 68, 65, 84, 65, 95, 67, 79, 78,
            83, 84, 0, 0, 0, 0, 192, 130, 0, 0, 1, 0, 0, 0, 104, 2, 0, 0, 0, 0, 0, 0, 192, 130, 0,
            0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            25, 0, 0, 0, 56, 1, 0, 0, 95, 95, 68, 65, 84, 65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192,
            0, 0, 1, 0, 0, 0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0,
            0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 95, 95, 100, 97, 116, 97, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 68, 65, 84, 65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 192,
            0, 0, 1, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 192, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 99, 111, 109, 109, 111,
            110, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 68, 65, 84, 65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32,
            192, 0, 0, 1, 0, 0, 0, 176, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 98, 115, 115, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 95, 95, 68, 65, 84, 65, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 208, 192,
            0, 0, 1, 0, 0, 0, 80, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 25, 0, 0, 0, 72, 0, 0, 0, 95, 95,
            76, 73, 78, 75, 69, 68, 73, 84, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, 0, 128, 0, 0,
            0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 160, 90, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let cmds = 5;
        let (_, result) = Commands::parse_commands(cmds, &test_data, &test_data).unwrap();
        assert_eq!(result.segments64[0].name, "__PAGEZERO");
        assert_eq!(result.segments64[0].vmaddr, 0);
        assert_eq!(result.segments64[0].vmsize, 0x100000000);
        assert_eq!(result.segments64[0].file_offset, 0);
        assert_eq!(result.segments64[0].file_size, 0);
        assert_eq!(result.segments64[0].max_prot, 0);
        assert_eq!(result.segments64[0].init_prot, 0);
        assert_eq!(result.segments64[0].nsects, 0);
        assert_eq!(result.segments64[0].flags, 0);

        assert_eq!(result.segments64[2].name, "__DATA_CONST");
        assert_eq!(result.segments64[2].vmaddr, 0x100008000);
        assert_eq!(result.segments64[2].vmsize, 0x4000);
        assert_eq!(result.segments64[2].file_offset, 0x8000);
        assert_eq!(result.segments64[2].file_size, 0x4000);
        assert_eq!(result.segments64[2].max_prot, 3);
        assert_eq!(result.segments64[2].init_prot, 3);
        assert_eq!(result.segments64[2].nsects, 3);
        assert_eq!(result.segments64[2].flags, 0x10);

        assert_eq!(result.segments64[4].name, "__LINKEDIT");
        assert_eq!(result.segments64[4].vmaddr, 0x100010000);
        assert_eq!(result.segments64[4].vmsize, 0x8000);
        assert_eq!(result.segments64[4].file_offset, 0x10000);
        assert_eq!(result.segments64[4].file_size, 0x5aa0);
        assert_eq!(result.segments64[4].max_prot, 1);
        assert_eq!(result.segments64[4].init_prot, 1);
        assert_eq!(result.segments64[4].nsects, 0);
        assert_eq!(result.segments64[4].flags, 0);
    }

    #[test]
    fn test_get_segment64() {
        let test_data = [
            95, 95, 80, 65, 71, 69, 90, 69, 82, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let mut segs: Vec<Segment64> = Vec::new();

        Commands::get_segment64(&test_data, &mut segs);
        assert_eq!(segs[0].name, "__PAGEZERO");
        assert_eq!(segs[0].vmaddr, 0);
        assert_eq!(segs[0].vmsize, 0x100000000);
        assert_eq!(segs[0].file_offset, 0);
        assert_eq!(segs[0].file_size, 0);
        assert_eq!(segs[0].max_prot, 0);
        assert_eq!(segs[0].init_prot, 0);
        assert_eq!(segs[0].nsects, 0);
        assert_eq!(segs[0].flags, 0);
    }

    #[test]
    fn test_get_segment32() {
        let test_data = [
            95, 95, 80, 65, 71, 69, 90, 69, 82, 79, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let mut segs: Vec<Segment64> = Vec::new();

        Commands::get_segment32(&test_data, &mut segs);
        assert_eq!(segs[0].name, "__PAGEZERO");
        assert_eq!(segs[0].vmaddr, 0);
        assert_eq!(segs[0].vmsize, 0x1);
        assert_eq!(segs[0].file_offset, 0);
        assert_eq!(segs[0].file_size, 0);
        assert_eq!(segs[0].max_prot, 0);
        assert_eq!(segs[0].init_prot, 0);
        assert_eq!(segs[0].nsects, 0);
        assert_eq!(segs[0].flags, 0);
    }

    #[test]
    fn test_get_uuid() {
        let test_data = [
            118, 176, 112, 103, 44, 205, 62, 212, 191, 187, 89, 4, 99, 208, 235, 224,
        ];
        let data = format_guid_be_bytes(&test_data);
        assert_eq!(data, "76b07067-2ccd-3ed4-bfbb-590463d0ebe0")
    }

    #[test]
    fn test_parse_dylyb_command() {
        let test_data = [
            24, 0, 0, 0, 2, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 47, 117, 115, 114, 47, 108, 105, 98,
            47, 108, 105, 98, 117, 116, 105, 108, 46, 100, 121, 108, 105, 98, 0, 0,
        ];
        let (_, result) = parse_dylyb_command(&test_data).unwrap();

        assert_eq!(result.name, "/usr/lib/libutil.dylib");
        assert_eq!(result.timestamp, 2);
        assert_eq!(result.current_version, "1.0.0");
        assert_eq!(result.compatibility_version, "1.0.0");
    }
}
