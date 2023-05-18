use super::error::ProcessError;
use crate::{artifacts::os::macos::macho::parser::MachoInfo, filesystem::files::read_file};
use log::error;

/// Get macho metadata for processes
pub(crate) fn macho_metadata(path: &str) -> Result<Vec<MachoInfo>, ProcessError> {
    let buffer_results = read_file(path);
    let buffer = match buffer_results {
        Ok(results) => results,
        Err(err) => {
            error!("[processes] Failed to read file: {path}, error: {err:?}");
            return Err(ProcessError::ParseProcFile);
        }
    };

    let binary_results = MachoInfo::parse_macho(&buffer);
    match binary_results {
        Ok(results) => Ok(results),
        Err(err) => {
            error!("[processes] Failed to parse process binary {path}, error: {err:?}");
            Err(ProcessError::ParseProcFile)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::macho_metadata;

    #[test]
    fn test_macho_metadata() {
        let test_path = "/bin/ls";
        let results = macho_metadata(test_path).unwrap();

        assert_eq!(results.len(), 2)
    }
}
