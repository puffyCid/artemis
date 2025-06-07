use super::version30::Version30;

pub(crate) type Version26 = Version30;

impl Version26 {
    /// Parse Prefetch version 26
    /// Format is nearly identical to version 30
    pub(crate) fn parse_file_info_ver26(data: &[u8]) -> nom::IResult<&[u8], Version26> {
        Version26::parse_file_info_ver30(data)
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::prefetch::versions::version26::Version26;

    #[test]
    fn test_parse_file_info_ver26() {
        let test_data = vec![
            48, 1, 0, 0, 61, 0, 0, 0, 208, 8, 0, 0, 214, 10, 0, 0, 216, 138, 0, 0, 12, 27, 0, 0,
            232, 165, 0, 0, 1, 0, 0, 0, 80, 7, 0, 0, 13, 0, 0, 0, 1, 0, 0, 0, 94, 49, 206, 21, 234,
            211, 213, 1, 107, 179, 197, 137, 49, 200, 213, 1, 168, 50, 151, 5, 49, 200, 213, 1,
            183, 136, 145, 174, 48, 200, 213, 1, 246, 240, 0, 85, 48, 200, 213, 1, 166, 161, 134,
            38, 33, 200, 213, 1, 10, 18, 58, 103, 32, 200, 213, 1, 187, 237, 200, 240, 30, 200,
            213, 1, 0, 140, 134, 71, 0, 0, 0, 0, 0, 140, 134, 71, 0, 0, 0, 0, 45, 0, 0, 0, 3, 0, 0,
            0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = Version26::parse_file_info_ver26(&test_data).unwrap();
        assert_eq!(result.file_array_offset, 304);
        assert_eq!(result.number_files, 61);
        assert_eq!(result.trace_chain_offset, 2256);
        assert_eq!(result.number_trace_chains, 2774);
        assert_eq!(result.filename_offset, 35544);
        assert_eq!(result.filename_size, 6924);
        assert_eq!(result.number_volumes, 1);
        assert_eq!(result.volume_info_size, 1872);
        assert_eq!(result.run_count, 45);
        assert_eq!(
            result.run_times,
            vec![
                "2020-01-26T01:44:01.000Z",
                "2020-01-11T03:45:16.000Z",
                "2020-01-11T03:41:35.000Z",
                "2020-01-11T03:39:09.000Z",
                "2020-01-11T03:36:38.000Z",
                "2020-01-11T01:47:58.000Z",
                "2020-01-11T01:42:37.000Z",
                "2020-01-11T01:32:09.000Z"
            ]
        );
    }
}
