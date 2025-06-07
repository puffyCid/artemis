use super::build::Build;
use crate::utils::{
    nom_helper::{Endian, nom_unsigned_four_bytes},
    strings::extract_utf8_string,
};
use common::macos::DylibCommand;
use nom::bytes::complete::{take, take_while};
use std::mem::size_of;

/// Parse DYLIB command data
pub(crate) fn parse_dylyb_command(data: &[u8]) -> nom::IResult<&[u8], DylibCommand> {
    let (dylib_data, _name_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let (dylib_data, timestamp) = nom_unsigned_four_bytes(dylib_data, Endian::Le)?;
    let (dylib_data, current_data) = take(size_of::<u32>())(dylib_data)?;
    let (dylib_data, compat_data) = take(size_of::<u32>())(dylib_data)?;

    let (_, current_version) = Build::get_versions(current_data)?;
    let (_, compatibility_version) = Build::get_versions(compat_data)?;
    let (dylib_data, name_data) = take_while(|b| b != 0)(dylib_data)?;

    let dylib = DylibCommand {
        name: extract_utf8_string(name_data),
        timestamp,
        current_version,
        compatibility_version,
    };
    Ok((dylib_data, dylib))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::macos::macho::commands::dylib::parse_dylyb_command;

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
