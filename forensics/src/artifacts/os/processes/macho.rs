use super::error::ProcessError;
use crate::artifacts::os::macos::macho::parser::parse_macho;
use common::macos::MachoInfo;
use tracing::error;

/// Get macho metadata for processes
pub(crate) fn macho_metadata(path: &str) -> Result<Vec<MachoInfo>, ProcessError> {
    let binary_results = parse_macho(path);
    let info = match binary_results {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to parse process binary {path}, error: {err:?}");
            return Err(ProcessError::ParseProcFile {
                path: path.to_string(),
                source: err.to_string(),
            });
        }
    };
    Ok(info)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::macho_metadata;

    #[test]
    fn test_macho_metadata() {
        let test_path = "/bin/ls";
        let results = macho_metadata(test_path).unwrap();

        assert_eq!(results.len(), 2);
    }
}
