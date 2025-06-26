use super::version30::Version30;
use crate::utils::{
    nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
    time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use nom::bytes::complete::take;
use std::mem::size_of;

pub(crate) type Version23 = Version30;

impl Version23 {
    /// Get fileinfo for Prefetch version 23 (Win7)
    pub(crate) fn parse_file_info_ver23(data: &[u8]) -> nom::IResult<&[u8], Version23> {
        let (input, file_array_offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, number_files) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, trace_chain_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, number_trace_chains) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, filename_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, filename_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, volume_info_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, number_volumes) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, volume_info_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, unknown) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let mut run_times: Vec<String> = Vec::new();
        let (input, runtime) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        run_times.push(unixepoch_to_iso(filetime_to_unixepoch(runtime)));

        let (input, unknown2_data) = take(size_of::<u128>())(input)?;

        let (input, run_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let unknown_size: usize = 80;
        let (input, _unknown4_data) = take(unknown_size)(input)?;

        let version = Version23 {
            file_array_offset,
            number_files,
            trace_chain_offset,
            number_trace_chains,
            filename_offset,
            filename_size,
            volume_info_offset,
            number_volumes,
            volume_info_size,
            _unknown: unknown,
            run_times,
            _unknown2: unknown2_data.to_vec(),
            run_count,
            _unknown3: unknown3,
            _unknown4: 0,
            _unknown5: Vec::new(),
        };

        Ok((input, version))
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::prefetch::versions::version23::Version23;

    #[test]
    fn test_parse_file_info_ver23() {
        let test_data = vec![
            240, 0, 0, 0, 58, 0, 0, 0, 48, 8, 0, 0, 139, 6, 0, 0, 180, 86, 0, 0, 112, 25, 0, 0, 40,
            112, 0, 0, 1, 0, 0, 0, 102, 8, 0, 0, 12, 0, 0, 0, 1, 0, 0, 0, 89, 131, 223, 40, 210,
            236, 216, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = Version23::parse_file_info_ver23(&test_data).unwrap();
        assert_eq!(result.file_array_offset, 240);
        assert_eq!(result.number_files, 58);
        assert_eq!(result.trace_chain_offset, 2096);
        assert_eq!(result.number_trace_chains, 1675);
        assert_eq!(result.filename_offset, 22196);
        assert_eq!(result.filename_size, 6512);
        assert_eq!(result.number_volumes, 1);
        assert_eq!(result.volume_info_size, 2150);
        assert_eq!(result.run_count, 1);
        assert_eq!(result.run_times, vec!["2022-10-31T02:40:38.000Z"]);
    }
}
