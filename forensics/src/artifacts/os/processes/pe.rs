use super::error::ProcessError;
use crate::artifacts::os::windows::pe::parser::parse_pe_file;
use common::windows::PeInfo;
use log::warn;

/// Parse PE metadata from provided path
pub(crate) fn pe_metadata(path: &str) -> Result<PeInfo, ProcessError> {
    let info_result = parse_pe_file(path);
    let info = match info_result {
        Ok(result) => result,
        Err(err) => {
            warn!("[processes] Could not parse PE process {path}: {err:?}");
            return Err(ProcessError::ParseProcFile);
        }
    };
    Ok(info)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::pe_metadata;

    #[test]
    fn test_pe_metadata() {
        let test = "C:\\Windows\\explorer.exe";
        let result = pe_metadata(test).unwrap();
        assert!(result.icons.len() > 3);
        assert!(result.cert.len() > 1000);
        assert_eq!(result.file_description, "Windows Explorer");
    }
}
