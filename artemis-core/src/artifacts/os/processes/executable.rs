use super::error::ProcessError;
use crate::artifacts::os::linux::executable::parser::{parse_elf_file, ElfInfo};
use log::error;

/// Get elf metadata for processes
pub(crate) fn elf_metadata(path: &str) -> Result<Vec<ElfInfo>, ProcessError> {
    let binary_results = parse_elf_file(path);
    let info = match binary_results {
        Ok(results) => vec![results],
        Err(err) => {
            error!("[processes] Failed to parse process binary {path}, error: {err:?}");
            return Err(ProcessError::ParseProcFile);
        }
    };
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::elf_metadata;

    #[test]
    fn test_elf_metadata() {
        let test_path = "/bin/ls";
        let results = elf_metadata(test_path).unwrap();

        assert_eq!(results.len(), 1);
    }
}
