use crate::utils::nom_helper::{nom_unsigned_one_byte, Endian};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct DataRun {
    cluster_offset: u64,
    cluster_length: u64,
    run_type: RunType,
    size: u8,
    data: Vec<u8>,
}

#[derive(Serialize)]
pub(crate) enum RunType {
    Standard,
    Sparse,
}

/// Try to get Data runs
pub(crate) fn parse_data_run(data: &[u8]) -> nom::IResult<&[u8], Vec<DataRun>> {
    let mut remaining = data;

    let bits = 0xf;
    let offset_adjust = 4;

    let mut runs = Vec::new();
    let min_size = 3;
    while remaining.len() >= min_size {
        let (input, cluster_block) = nom_unsigned_one_byte(remaining, Endian::Le)?;
        let offset = (cluster_block & bits) >> offset_adjust;
        let length = cluster_block & bits;

        let (input, size) = nom_unsigned_one_byte(input, Endian::Le)?;

        if size == 0 || input.len() < length as usize {
            break;
        }

        let (input, data) = take(length)(input)?;
        let run = DataRun {
            cluster_offset: offset as u64,
            cluster_length: length as u64,
            size,
            run_type: if offset == 0 {
                RunType::Sparse
            } else {
                RunType::Standard
            },
            data: data.to_vec(),
        };

        remaining = input;
        runs.push(run);
    }
    Ok((remaining, runs))
}

#[cfg(test)]
mod tests {
    use super::parse_data_run;

    #[test]
    fn test_parse_data_run() {
        let test = [
            77, 88, 40, 228, 51, 192, 136, 0, 215, 1, 217, 50, 128, 23, 107, 82, 126, 50, 64, 76,
            171, 27, 104, 51, 128, 136, 0, 148, 37, 57, 66, 192, 92, 134, 16, 39, 255, 50, 0, 127,
            174, 230, 122, 66, 128, 5, 176, 69, 222, 254, 0, 0, 0, 0, 0, 0,
        ];
        let (_, runs) = parse_data_run(&test).unwrap();
        assert_eq!(runs.len(), 5);
        assert_eq!(runs[0].cluster_length, 13);
        assert_eq!(runs[0].cluster_offset, 0);
        assert_eq!(runs[0].size, 88);
    }
}
