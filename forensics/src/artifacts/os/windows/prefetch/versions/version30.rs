use crate::utils::{
    nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
    time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use log::warn;
use nom::{Needed, bytes::complete::take};
use serde::Serialize;
use std::mem::size_of;

#[derive(Debug, Serialize)]
pub(crate) struct Version30 {
    pub(crate) file_array_offset: u32,
    pub(crate) number_files: u32,
    pub(crate) trace_chain_offset: u32,
    pub(crate) number_trace_chains: u32,
    pub(crate) filename_offset: u32,
    pub(crate) filename_size: u32,
    pub(crate) volume_info_offset: u32,
    pub(crate) number_volumes: u32,
    pub(crate) volume_info_size: u32,
    pub(crate) _unknown: u64,
    pub(crate) run_times: Vec<String>,
    pub(crate) _unknown2: Vec<u8>, // may be 16 or 8 bytes. Depending on size of prefetch data (224 vs 216)
    pub(crate) run_count: u32,
    pub(crate) _unknown3: u32,
    pub(crate) _unknown4: u32,
    pub(crate) _unknown5: Vec<u8>,
}

impl Version30 {
    /// Get fileinfo for Prefetch version 30 (Win10+)
    pub(crate) fn parse_file_info_ver30(data: &[u8]) -> nom::IResult<&[u8], Version30> {
        let (input, file_array_offset) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, number_files) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, trace_chain_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, number_trace_chains) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, filename_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, filename_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, volume_info_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, number_volumes) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, volume_info_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (mut input, unknown) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        let mut run_times: Vec<String> = Vec::new();

        let max_runtime_count = 8;
        let mut count = 0;
        while count != max_runtime_count {
            let (runs_data, runtime) = nom_unsigned_eight_bytes(input, Endian::Le)?;

            let no_runs = 0;
            if runtime != no_runs {
                run_times.push(unixepoch_to_iso(&filetime_to_unixepoch(&runtime)));
            }
            count += 1;
            input = runs_data;
        }

        // Version 30 has been seen with two (2) variants.
        // Determines the offset for the file metrics array
        let variant1 = 304; // Also matches version 26
        let variant2 = 296;

        let unknown2;
        if file_array_offset == variant1 {
            let (remaining_input, unknown2_data) = take(size_of::<u128>())(input)?;
            input = remaining_input;
            unknown2 = unknown2_data.to_vec();
        } else if file_array_offset == variant2 {
            let (remaining_input, unknown2_data) = take(size_of::<u64>())(input)?;
            input = remaining_input;
            unknown2 = unknown2_data.to_vec();
        } else {
            warn!("[prefetch] Unknown prefetch version 30 variant, size: {file_array_offset}");
            return Err(nom::Err::Incomplete(Needed::Unknown));
        }

        let (input, run_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, unknown3) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, unknown4) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, unknown5) = take(size_of::<u32>())(input)?;

        let version = Version30 {
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
            _unknown2: unknown2,
            run_count,
            _unknown3: unknown3,
            _unknown4: unknown4,
            _unknown5: unknown5.to_vec(),
        };

        Ok((input, version))
    }
}

#[cfg(test)]
mod tests {
    use super::Version30;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_parse_file_info() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/versions/version30.raw");

        let buffer = fs::read(test_location).unwrap();

        let (_, result) = Version30::parse_file_info_ver30(&buffer).unwrap();

        assert_eq!(result.file_array_offset, 296);
        assert_eq!(result.number_files, 64);
        assert_eq!(result.trace_chain_offset, 2344);
        assert_eq!(result.number_trace_chains, 4459);
        assert_eq!(result.filename_offset, 38016);
        assert_eq!(result.filename_size, 10344);
        assert_eq!(result.number_volumes, 1);
        assert_eq!(result.volume_info_size, 2572);
        assert_eq!(result.run_count, 1);

        assert_eq!(result._unknown, 4294967311);
        assert_eq!(result.run_times, vec!["2022-10-16T02:17:45.000Z"]);
        assert_eq!(result._unknown4, 0);
        assert_eq!(result._unknown2, vec![0, 0, 0, 0, 0, 0, 0, 0]);
        assert_eq!(result._unknown3, 1);
        assert_eq!(result._unknown5, vec![232, 188, 0, 0]);
    }
}
