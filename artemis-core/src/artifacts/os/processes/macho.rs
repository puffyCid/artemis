use super::error::ProcessError;
use crate::artifacts::os::macos::macho::parser::MachoInfo;
use log::error;

/// Get macho metadata for processes
pub(crate) fn macho_metadata(path: &str) -> Result<Vec<MachoInfo>, ProcessError> {
    let binary_results = MachoInfo::parse_macho(path);
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
