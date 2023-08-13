use crate::utils::nom_helper::{nom_signed_four_bytes, Endian};
use nom::bytes::complete::take;

/// Using data from the header. Find the all the slots associated with the Short Sector Allocation Table (SSAT)
pub(crate) fn assemble_ssat_data(
    data: &[u8],
    start_sector: i32,
    sat_size: u32,
) -> nom::IResult<&[u8], Vec<i32>> {
    let no_ssat = 0;
    if start_sector < no_ssat {
        return Ok((data, Vec::new()));
    }

    let (input, _) = take(start_sector as u32 * sat_size)(data)?;
    let (_, mut input) = take(sat_size)(input)?;

    let mut ssat_slots = Vec::new();
    let unused = -11;

    while !input.is_empty() {
        let (ssat_remaining, ssat_slot) = nom_signed_four_bytes(input, Endian::Le)?;
        if ssat_slot == unused {
            break;
        }
        ssat_slots.push(ssat_slot);
        input = ssat_remaining;
    }

    Ok((data, ssat_slots))
}

/// SSAT may have additional data. Need to use SAT slots to find it.
pub(crate) fn add_ssat_slots<'a>(
    data: &'a [u8],
    slots: &[i32],
    start: u32,
    size: u32,
) -> nom::IResult<&'a [u8], Vec<i32>> {
    let mut ssat_slots = Vec::new();
    let mut slot_value = start;

    // Now use the SAT slots to determine the next SSAT slots
    // Loop until negative slot value is encountered
    while slots.len() > slot_value as usize {
        // start also represents first slot index
        let slot = slots[slot_value as usize];
        // Any negative value means we have reached end
        if slot < 0 {
            break;
        }

        // Use slot value to jump to next sector
        let (dir_start, _) = take(slot as u32 * size)(data)?;
        // Get data of based on sector size
        let (_, mut value) = take(size)(dir_start)?;

        let unused = -11;
        // nom ssat slots until end or unused
        while !value.is_empty() {
            let (ssat_remaining, sat_slot) = nom_signed_four_bytes(value, Endian::Le)?;
            if sat_slot == unused {
                break;
            }
            ssat_slots.push(sat_slot);
            value = ssat_remaining;
        }

        // the slot value then points to the next SAT slot
        slot_value = slot as u32;
    }

    Ok((data, ssat_slots))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::ole::header::OleHeader;
    use crate::artifacts::os::windows::ole::sat::assemble_sat_data;
    use crate::artifacts::os::windows::ole::ssat::{add_ssat_slots, assemble_ssat_data};
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_assemble_ssat_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/jumplists/win11/automatic/1b4dd67f29cb1962.automaticDestinations-ms");
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (input, header) = OleHeader::parse_header(&data).unwrap();
        let size: u32 = 2;
        let (_, result) = assemble_ssat_data(
            input,
            header.sector_id_ssat,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        assert!(result.starts_with(&[1, 2, 3, 4, 5, 6]));
        assert_eq!(result.len(), 128);
    }

    #[test]
    fn test_add_ssat_slots() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/jumplists/win11/automatic/3d2110c4a0cb6d15.automaticDestinations-ms");
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (input, header) = OleHeader::parse_header(&data).unwrap();
        let size: u32 = 2;
        let (_, result) = assemble_ssat_data(
            input,
            header.sector_id_ssat,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        assert!(result.starts_with(&[1, 2, 3, 4, 5, 6]));
        // Consumed whole first SSAT sector
        assert_eq!(result.len(), 128);

        let (_, sat_slots) = assemble_sat_data(
            input,
            &header.msat_sectors,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        // now need to use SAT slots to get additional SSAT slots
        let (_, additional_ssat) = add_ssat_slots(
            input,
            &sat_slots,
            header.sector_id_ssat as u32,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        assert_eq!(additional_ssat.len(), 768);
    }
}
