use super::error::ProcessError;
use crate::artifacts::os::windows::pe::parser::{parse_pe_file, PeInfo};
use log::warn;

/// Parse PE metadata from provided path
pub(crate) fn pe_metadata(path: &str) -> Result<Vec<PeInfo>, ProcessError> {
    let info_result = parse_pe_file(path);
    let info = match info_result {
        Ok(result) => result,
        Err(err) => {
            warn!("[processes] Could not parse PE process {path}: {err:?}");
            return Err(ProcessError::ParseProcFile);
        }
    };
    Ok(vec![info])
}

#[cfg(test)]
mod tests {
    use super::pe_metadata;

    #[test]
    fn test_pe_metadata() {
        let test = "C:\\Windows\\explorer.exe";
        let result = pe_metadata(test).unwrap();
        assert!(result[0].icons.len() > 3);
        assert!(result[0].cert.len() > 1000);
        assert_eq!(result[0].file_description, "Windows Explorer");
    }
}
