/**
 * Linux Executable and Linkable Format `ELF` is the native executable format of Linux programs  
 * We currently parse out basic amount of metadata
 *
 * References:  
 *   `https://en.wikipedia.org/wiki/Executable_and_Linkable_Format`
 *
 * Other Parsers:  
 *   `https://github.com/radareorg/radare2`  
 *   `https://lief-project.github.io/`
 */
use crate::filesystem::files::{file_reader, file_too_large};
use common::linux::ElfInfo;
use elf::ElfBytes;
use elf::endian::AnyEndian;
use log::error;
use std::io::{Error, ErrorKind, Read, Seek, SeekFrom};

/// Parse an `ELF` file at provided path
pub(crate) fn parse_elf_file(path: &str) -> Result<ElfInfo, elf::parse::ParseError> {
    let reader_result = file_reader(path);
    let mut reader = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[elf] Could not get reader for {path}: {err:?}");
            return Err(elf::ParseError::IOError(Error::new(
                ErrorKind::NotFound,
                err,
            )));
        }
    };
    let mut buff = [0; 4];
    if reader.read(&mut buff).is_err() {
        return Err(elf::ParseError::IOError(Error::new(
            ErrorKind::InvalidInput,
            "",
        )));
    }
    let elf_magic = [127, 69, 76, 70];
    if buff != elf_magic {
        return Err(elf::ParseError::BadMagic(buff));
    }

    if reader.seek(SeekFrom::Start(0)).is_err() {
        return Err(elf::ParseError::IOError(Error::new(
            ErrorKind::InvalidData,
            "Could not seek to start",
        )));
    }

    if file_too_large(path) {
        return Err(elf::ParseError::IOError(Error::new(
            ErrorKind::InvalidInput,
            "File larger than 2GB",
        )));
    }

    let mut data = Vec::new();

    // Allow File read_to_end because we partially read the file above to check for Magic Header
    #[allow(clippy::verbose_file_reads)]
    let data_result = reader.read_to_end(&mut data);
    match data_result {
        Ok(_) => {}
        Err(_) => {
            return Err(elf::ParseError::IOError(Error::new(
                ErrorKind::InvalidInput,
                "Could not read file to end",
            )));
        }
    };

    let elf_data = ElfBytes::<AnyEndian>::minimal_parse(&data)?;
    let sections = elf_sections(&elf_data)?;
    let machine_type = elf::to_str::e_machine_to_string(elf_data.ehdr.e_machine);
    let symbols = elf_symbols(&elf_data)?;

    let elf_info = ElfInfo {
        symbols,
        sections,
        machine_type,
    };
    Ok(elf_info)
}

/// Get the sections of an `ELF` binary
fn elf_sections(elf_data: &ElfBytes<'_, AnyEndian>) -> Result<Vec<String>, elf::parse::ParseError> {
    let (sections, string_table) = elf_data.section_headers_with_strtab()?;
    let mut sections_vec: Vec<String> = Vec::new();
    if let Some(sects) = sections
        && let Some(table) = string_table
    {
        for sect in sects {
            let sect_name = table.get(sect.sh_name as usize).unwrap_or_default();
            if sect_name.is_empty() {
                continue;
            }

            sections_vec.push(sect_name.to_string());
        }
    }

    Ok(sections_vec)
}

/// Get the symbols of an `ELF` binary
fn elf_symbols(elf_data: &ElfBytes<'_, AnyEndian>) -> Result<Vec<String>, elf::parse::ParseError> {
    let sym_table = elf_data.symbol_table()?;
    let mut symbols_vec: Vec<String> = Vec::new();

    if let Some((symbols, string_table)) = sym_table {
        for sym in symbols {
            let sym_name = string_table.get(sym.st_name as usize).unwrap_or_default();
            if sym_name.is_empty() {
                continue;
            }

            symbols_vec.push(sym_name.to_string());
        }
    }

    let dyn_sym_table = elf_data.dynamic_symbol_table()?;
    if let Some((symbols, string_table)) = dyn_sym_table {
        for sym in symbols {
            let sym_name = string_table.get(sym.st_name as usize).unwrap_or_default();
            if sym_name.is_empty() {
                continue;
            }

            symbols_vec.push(sym_name.to_string());
        }
    }
    Ok(symbols_vec)
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::linux::executable::parser::{
        elf_sections, elf_symbols, parse_elf_file,
    };
    use crate::filesystem::files::read_file;
    use elf::ElfBytes;
    use elf::endian::AnyEndian;
    use std::path::PathBuf;

    #[test]
    fn test_parse_elf_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/elf/xinit");

        let result = parse_elf_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.symbols.len(), 51);
        assert_eq!(result.sections.len(), 25);
    }

    #[test]
    fn test_elf_symbols() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/elf/xinit");
        let data = read_file(&test_location.display().to_string()).unwrap();
        let elf_data = ElfBytes::<AnyEndian>::minimal_parse(&data).unwrap();
        let symbols = elf_symbols(&elf_data).unwrap();
        assert_eq!(symbols.len(), 51);
    }

    #[test]
    fn test_elf_sections() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/elf/xinit");
        let data = read_file(&test_location.display().to_string()).unwrap();
        let elf_data = ElfBytes::<AnyEndian>::minimal_parse(&data).unwrap();
        let sections = elf_sections(&elf_data).unwrap();
        assert_eq!(sections.len(), 25);
    }
}
