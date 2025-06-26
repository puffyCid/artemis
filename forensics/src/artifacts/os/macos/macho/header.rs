use crate::utils::nom_helper::{Endian, nom_unsigned_four_bytes};
use log::warn;
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct MachoHeader {
    signature: u32,
    pub cpu_type: String,
    pub cpu_subtype: String,
    pub filetype: String,
    pub(crate) number_commands: u32,
    pub(crate) commands_size: u32,
    pub(crate) flags: Vec<MachoFlags>,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum MachoFlags {
    NoUndefines,
    IncrementalLink,
    DynamicLink,
    BindAtLoad,
    Prebound,
    SplitSegs,
    LazyInit,
    TwoLevel,
    ForceFlat,
    NoMultiDefinitions,
    NoFixPrebinding,
    Prebindable,
    AllModulesBound,
    SubsectionsViaSymbol,
    Canonical,
    WeakDefines,
    AllowStackExecution,
    RootSafe,
    SetuidSafe,
    NoReexportedDylib,
    Pie,
    DeadStrippableDylib,
    HasTlvDescription,
    NoHeapExecution,
    AppExtensionSafe,
    NlistAutofsyncWithDyldinfo,
    SimSupport,
    BindToWeak,
}

impl MachoHeader {
    /// Parse and return metadata from the Macho header
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], MachoHeader> {
        let (macho_data, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (macho_data, cpu_type) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
        let (macho_data, cpu_subtype) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
        let (macho_data, filetype) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
        let (macho_data, number_commands) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
        let (macho_data, commands_size) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
        let (macho_data, flags) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;

        let (cpu_type_str, subtype_str) = MachoHeader::get_cpu_type(cpu_type, cpu_subtype);
        let header = MachoHeader {
            signature,
            cpu_type: cpu_type_str,
            cpu_subtype: subtype_str,
            filetype: MachoHeader::get_filetype(filetype),
            number_commands,
            commands_size,
            flags: MachoHeader::get_flags(flags),
        };
        if header.cpu_type == "X86_64" || header.cpu_type == "ARM64" {
            let (macho_data, _reserved) = nom_unsigned_four_bytes(macho_data, Endian::Le)?;
            return Ok((macho_data, header));
        }

        Ok((macho_data, header))
    }

    /// Get start of macho in FAT binary
    pub(crate) fn binary_start(data: &[u8], offset: u32, size: u32) -> nom::IResult<&[u8], &[u8]> {
        let (binary_start, _) = take(offset)(data)?;
        let (_, binary_data) = take(size)(binary_start)?;
        Ok((data, binary_data))
    }

    /// Check if macho file
    pub(crate) fn is_macho(data: &[u8]) -> nom::IResult<&[u8], bool> {
        let (_, signature) = nom_unsigned_four_bytes(data, Endian::Be)?;

        let macho_sig = 0xcffaedfe;
        if signature != macho_sig {
            return Ok((data, false));
        }
        Ok((data, true))
    }

    /// Check for CPU Type associated with binary
    fn get_cpu_type(cpu_type: u32, subtype: u32) -> (String, String) {
        let (cpu, sub) = match cpu_type {
            0x1 => (String::from("VAX"), String::from("Not supported")),
            0x6 => (String::from("MC680x0"), String::from("Not supported")),
            0x7 => (String::from("X86"), MachoHeader::get_intel_subtype(subtype)),
            0x1000007 => (
                String::from("X86_64"),
                MachoHeader::get_intel_subtype(subtype),
            ),
            0xa => (String::from("MC98000"), String::from("Not supported")),
            0xb => (String::from("HPPA"), String::from("Not supported")),
            0x100000C => (String::from("ARM64"), MachoHeader::get_arm_subtype(subtype)),
            0xc => (String::from("ARM"), MachoHeader::get_arm_subtype(subtype)),
            0xd => (String::from("MC88000"), String::from("Not supported")),
            0xe => (String::from("SPARC"), String::from("Not supported")),
            0xf => (String::from("I860"), String::from("Not supported")),
            0x12 => (String::from("POWERPC"), String::from("Not supported")),
            0x1000012 => (String::from("POWERPC64"), String::from("Not supported")),
            _ => (
                String::from("Unknown MachO CPU Type"),
                String::from("Not supported"),
            ),
        };
        (cpu, sub)
    }

    /// Get CPU subtype for Intel
    fn get_intel_subtype(subtype: u32) -> String {
        let sub = match subtype {
            0 => "ALL",
            0x3 => "386",
            0x4 => "486",
            0x84 => "486SX",
            0x5 => "586PENT",
            0x16 => "PENTPRO",
            0x36 => "PENTIIM3",
            0x56 => "PENTIIM5",
            0x67 => "CELERON",
            0x77 => "CELERONMOBILE",
            0x8 => "PENTIUM3",
            0x18 => "PENTIUM3M",
            0x28 => "PENTIMUM3XEON",
            0x9 => "PENTIUMM",
            0xa => "PENTIUM4",
            0x1a => "PENTIUM4M",
            0xb => "ITANIUM",
            0x1b => "ITANIUM2",
            0xc => "XEON",
            0x1c => "XEONMP",
            0x80000003 => "x86 ALL",
            _ => {
                warn!("[macho] Unknown MachO sub CPU Intel Type: {subtype}");
                "Unknown MachO sub CPU Intel Type"
            }
        };

        sub.to_string()
    }

    /// Get CPU subtype for ARM
    fn get_arm_subtype(subtype: u32) -> String {
        let sub = match subtype {
            0 => "ALL",
            0x1 => "ARM64v8",
            0x2 | 0x80000002 => "ARM64E",
            0x5 => "ARMv4T",
            0x6 => "ARMv6",
            0x7 => "ARMv5|ARMv5TEJ",
            0x8 => "ARMvXSCALE",
            0x9 => "ARMv7",
            0xa => "ARMv7F",
            0xb => "ARMv7S",
            0xc => "ARMv7K",
            0xe => "ARMv6M",
            0xf => "ARMv7M",
            0x10 => "ARMv7EM",
            0xc0000002 => "A14",
            _ => {
                warn!("[macho] Unknown MachO sub CPU ARM Type: {subtype}");
                "Unknown MachO sub CPU ARM Type"
            }
        };

        sub.to_string()
    }

    /// Check filetype for binary
    fn get_filetype(filetype: u32) -> String {
        let file = match filetype {
            0x1 => "OBJECT",
            0x2 => "EXECUTE",
            0x3 => "FVMLIB",
            0x4 => "CORE",
            0x5 => "PRELOAD",
            0x6 => "DYLIB",
            0x7 => "DYLINKER",
            0x8 => "BUNDLE",
            0x9 => "DYLIB_STUB",
            0xa => "DSYM",
            0xb => "KEXT_BUNDLE",
            0xc => "FILESET",
            _ => {
                warn!("[macho] Unknown MachO File Type: {filetype}");
                "Unknown MachO FILE Type"
            }
        };
        file.to_string()
    }

    /// Get all flags for binary
    fn get_flags(flag_data: u32) -> Vec<MachoFlags> {
        let mut flags = Vec::new();

        let no_undefines = 0x1;
        let incrlink = 0x2;
        let dyldlink = 0x4;
        let bind_at_load = 0x8;
        let prebound = 0x10;
        let split_segs = 0x20;
        let lazy_init = 0x40;
        let two_level = 0x80;
        let force_flat = 0x100;
        let no_multi_defs = 0x200;
        let no_fix_prebinding = 0x400;
        let prebind = 0x800;
        let all_mods_bound = 0x1000;
        let subsections_symbols = 0x2000;
        let canonical = 0x4000;
        let weak_defines = 0x8000;
        let binds_to_weak = 0x10000;
        let allow_stack_execution = 0x20000;
        let root_safe = 0x40000;
        let setuid_safe = 0x80000;
        let no_reexported_dylibs = 0x100000;
        let pie = 0x200000;
        let dead_strip_dylib = 0x400000;
        let has_tlv = 0x800000;
        let no_heap_execution = 0x1000000;
        let app_extension = 0x2000000;
        let nlist = 0x4000000;
        let sim = 0x8000000;

        if (flag_data & no_undefines) != 0 {
            flags.push(MachoFlags::NoUndefines);
        }
        if (flag_data & incrlink) != 0 {
            flags.push(MachoFlags::IncrementalLink);
        }
        if (flag_data & dyldlink) != 0 {
            flags.push(MachoFlags::DynamicLink);
        }
        if (flag_data & bind_at_load) != 0 {
            flags.push(MachoFlags::BindAtLoad);
        }
        if (flag_data & prebound) != 0 {
            flags.push(MachoFlags::Prebound);
        }
        if (flag_data & split_segs) != 0 {
            flags.push(MachoFlags::SplitSegs);
        }
        if (flag_data & lazy_init) != 0 {
            flags.push(MachoFlags::LazyInit);
        }
        if (flag_data & two_level) != 0 {
            flags.push(MachoFlags::TwoLevel);
        }
        if (flag_data & force_flat) != 0 {
            flags.push(MachoFlags::ForceFlat);
        }
        if (flag_data & no_multi_defs) != 0 {
            flags.push(MachoFlags::NoMultiDefinitions);
        }
        if (flag_data & no_fix_prebinding) != 0 {
            flags.push(MachoFlags::NoFixPrebinding);
        }
        if (flag_data & all_mods_bound) != 0 {
            flags.push(MachoFlags::AllModulesBound);
        }
        if (flag_data & subsections_symbols) != 0 {
            flags.push(MachoFlags::SubsectionsViaSymbol);
        }
        if (flag_data & canonical) != 0 {
            flags.push(MachoFlags::Canonical);
        }
        if (flag_data & weak_defines) != 0 {
            flags.push(MachoFlags::WeakDefines);
        }
        if (flag_data & binds_to_weak) != 0 {
            flags.push(MachoFlags::BindToWeak);
        }
        if (flag_data & allow_stack_execution) != 0 {
            flags.push(MachoFlags::AllowStackExecution);
        }
        if (flag_data & root_safe) != 0 {
            flags.push(MachoFlags::RootSafe);
        }
        if (flag_data & setuid_safe) != 0 {
            flags.push(MachoFlags::SetuidSafe);
        }
        if (flag_data & no_reexported_dylibs) != 0 {
            flags.push(MachoFlags::NoReexportedDylib);
        }
        if (flag_data & pie) != 0 {
            flags.push(MachoFlags::Pie);
        }
        if (flag_data & dead_strip_dylib) != 0 {
            flags.push(MachoFlags::DeadStrippableDylib);
        }
        if (flag_data & has_tlv) != 0 {
            flags.push(MachoFlags::HasTlvDescription);
        }
        if (flag_data & no_heap_execution) != 0 {
            flags.push(MachoFlags::NoHeapExecution);
        }
        if (flag_data & app_extension) != 0 {
            flags.push(MachoFlags::AppExtensionSafe);
        }
        if (flag_data & sim) != 0 {
            flags.push(MachoFlags::SimSupport);
        }
        if (flag_data & nlist) != 0 {
            flags.push(MachoFlags::NlistAutofsyncWithDyldinfo);
        }
        if (flag_data & prebind) != 0 {
            flags.push(MachoFlags::Prebindable);
        }

        flags
    }
}

#[cfg(test)]
mod tests {
    use super::MachoHeader;
    use crate::artifacts::os::macos::macho::header::MachoFlags;

    #[test]
    fn test_parse_intel_header() {
        let test_data = [
            207, 250, 237, 254, 7, 0, 0, 1, 3, 0, 0, 0, 2, 0, 0, 0, 18, 0, 0, 0, 24, 7, 0, 0, 133,
            0, 32, 0, 0, 0, 0, 0,
        ];
        let (_, result) = MachoHeader::parse_header(&test_data).unwrap();
        assert_eq!(result.signature, 0xfeedfacf);
        assert_eq!(result.cpu_type, "X86_64");
        assert_eq!(result.cpu_subtype, "386");
        assert_eq!(result.filetype, "EXECUTE");
        assert_eq!(result.number_commands, 18);
        assert_eq!(result.commands_size, 1816);
        assert_eq!(
            result.flags,
            vec![
                MachoFlags::NoUndefines,
                MachoFlags::DynamicLink,
                MachoFlags::TwoLevel,
                MachoFlags::Pie
            ]
        );
    }

    #[test]
    fn test_parse_arm_header() {
        let test_data = [
            207, 250, 237, 254, 12, 0, 0, 1, 2, 0, 0, 128, 2, 0, 0, 0, 19, 0, 0, 0, 192, 6, 0, 0,
            133, 0, 32, 0, 0, 0, 0, 0,
        ];
        let (_, result) = MachoHeader::parse_header(&test_data).unwrap();
        assert_eq!(result.signature, 0xfeedfacf);
        assert_eq!(result.cpu_type, "ARM64");
        assert_eq!(result.cpu_subtype, "ARM64E");
        assert_eq!(result.filetype, "EXECUTE");
        assert_eq!(result.number_commands, 19);
        assert_eq!(result.commands_size, 1728);
        assert_eq!(
            result.flags,
            vec![
                MachoFlags::NoUndefines,
                MachoFlags::DynamicLink,
                MachoFlags::TwoLevel,
                MachoFlags::Pie
            ]
        );
    }

    #[test]
    fn test_get_intel_subtype() {
        let test_data = 0;
        let result = MachoHeader::get_intel_subtype(test_data);
        assert_eq!(result, "ALL")
    }

    #[test]
    fn test_get_arm_subtype() {
        let test_data = 0;
        let result = MachoHeader::get_arm_subtype(test_data);
        assert_eq!(result, "ALL")
    }

    #[test]
    fn test_get_cpu_type() {
        let test_data = 0x1000007;
        let subdata = 0;
        let result = MachoHeader::get_cpu_type(test_data, subdata);
        assert_eq!(result, (String::from("X86_64"), String::from("ALL")))
    }

    #[test]
    fn test_get_filetype() {
        let test_data = 3;
        let result = MachoHeader::get_filetype(test_data);
        assert_eq!(result, "FVMLIB")
    }

    #[test]
    fn test_get_flags() {
        let test_data = 0x00200085;
        let result = MachoHeader::get_flags(test_data);
        assert_eq!(
            result,
            vec![
                MachoFlags::NoUndefines,
                MachoFlags::DynamicLink,
                MachoFlags::TwoLevel,
                MachoFlags::Pie
            ]
        );
    }
}
