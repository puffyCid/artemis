use crate::utils::nom_helper::{nom_signed_four_bytes, Endian};
use nom::bytes::complete::take;

/// Using data from the header. Find and assemble all data assicated with Sector Allocation Table (SAT)
pub(crate) fn assemble_sat_data<'a>(
    data: &'a [u8],
    sat_sectors: &[u32],
    sat_size: u32,
) -> nom::IResult<&'a [u8], Vec<i32>> {
    let input = data;

    let mut sat_slots = Vec::new();

    let unused = -1;
    for entry in sat_sectors {
        let sat_offset = entry * sat_size;
        let (sat_start, _) = take(sat_offset)(input)?;

        let (_, mut data) = take(sat_size)(sat_start)?;
        // Go through SAT data and extract the slot values
        // These values are used to assemble the Directory data
        loop {
            let (sat_remaining, sat_slot) = nom_signed_four_bytes(data, Endian::Le)?;
            if sat_slot == unused {
                break;
            }
            sat_slots.push(sat_slot);
            data = sat_remaining;
        }
    }

    Ok((input, sat_slots))
}

#[cfg(test)]
mod tests {
    use super::assemble_sat_data;
    use crate::artifacts::os::windows::ole::header::OleHeader;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_assemble_sat_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/jumplists/win11/automatic/1b4dd67f29cb1962.automaticDestinations-ms");
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (input, header) = OleHeader::parse_header(&data).unwrap();
        let size: u32 = 2;
        let (_, result) = assemble_sat_data(
            input,
            &header.msat_sectors,
            size.pow(header.sector_size as u32),
        )
        .unwrap();
        assert_eq!(result, [-3, 6, -2, 4, 5, 7, -2, 8, 9, -2]);
    }
}
