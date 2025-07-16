use super::{version23::Version23, version26::Version26, version30::Version30};
use log::error;

pub(crate) struct VersionInfo {
    pub(crate) file_array_offset: u32,
    pub(crate) number_files: u32,
    pub(crate) trace_chain_offset: u32,
    pub(crate) number_trace_chains: u32,
    pub(crate) filename_offset: u32,
    pub(crate) filename_size: u32,
    pub(crate) volume_info_offset: u32,
    pub(crate) number_volumes: u32,
    pub(crate) volume_info_size: u32,
    pub(crate) run_times: Vec<String>,
    pub(crate) run_count: u32,
}

impl VersionInfo {
    /// Get Prefetch version data based on version value
    pub(crate) fn get_version_info(data: &[u8], version: u32) -> nom::IResult<&[u8], VersionInfo> {
        let version23 = 23; // Win7
        let version26 = 26; // Win8
        let version30 = 30; // Win10+
        let version31 = 31; // Windows 11

        let mut version_info = VersionInfo {
            file_array_offset: 0,
            number_files: 0,
            trace_chain_offset: 0,
            number_trace_chains: 0,
            filename_offset: 0,
            filename_size: 0,
            volume_info_offset: 0,
            number_volumes: 0,
            volume_info_size: 0,
            run_times: Vec::new(),
            run_count: 0,
        };

        let (pf_data, result) = if version == version26 {
            Version26::parse_file_info_ver26(data)?
        } else if version == version30 || version == version31 {
            Version30::parse_file_info_ver30(data)?
        } else if version == version23 {
            Version23::parse_file_info_ver23(data)?
        } else {
            error!("[prefetch] Unsupported Prefetch version: {version}");
            return Err(nom::Err::Incomplete(nom::Needed::Unknown));
        };

        version_info.file_array_offset = result.file_array_offset;
        version_info.number_files = result.number_files;
        version_info.trace_chain_offset = result.trace_chain_offset;
        version_info.number_trace_chains = result.number_trace_chains;
        version_info.filename_offset = result.filename_offset;

        version_info.filename_size = result.filename_size;
        version_info.volume_info_offset = result.volume_info_offset;
        version_info.number_volumes = result.number_volumes;
        version_info.volume_info_size = result.volume_info_size;
        version_info.run_times = result.run_times;
        version_info.run_count = result.run_count;

        Ok((pf_data, version_info))
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::prefetch::versions::version::VersionInfo;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_get_version_info_version30() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/versions/version30.raw");

        let buffer = fs::read(test_location).unwrap();

        let (_, result) = VersionInfo::get_version_info(&buffer, 30).unwrap();

        assert_eq!(result.file_array_offset, 296);
        assert_eq!(result.number_files, 64);
        assert_eq!(result.trace_chain_offset, 2344);
        assert_eq!(result.number_trace_chains, 4459);
        assert_eq!(result.filename_offset, 38016);
        assert_eq!(result.filename_size, 10344);
        assert_eq!(result.number_volumes, 1);
        assert_eq!(result.volume_info_size, 2572);
        assert_eq!(result.run_count, 1);

        assert_eq!(result.run_times, vec!["2022-10-16T02:17:45.000Z"]);
    }
}
