use nom::bytes::complete::take;

/// Using the SAT Slot values, assemble the Directory data
pub(crate) fn assemble_directory_data<'a>(
    data: &'a [u8],
    sat_slots: &[i32],
    first_sector: u32,
    sat_size: u32,
) -> nom::IResult<&'a [u8], Vec<u8>> {
    let mut dir_data = Vec::new();

    // Go to start of first Directory sector
    let (dir_start, _) = take(first_sector * sat_size)(data)?;
    // Get data of based on sector size
    let (_, value) = take(sat_size)(dir_start)?;

    dir_data.append(&mut value.to_vec());

    let mut slot_value = first_sector;

    // Now use the SAT slots to determine the Directory data
    // Loop until negative slot value is encountered
    while sat_slots.len() > slot_value as usize {
        // first_sector also represents first slot index
        let slot = sat_slots[slot_value as usize];
        // Any negative value means we have reached end
        if slot < 0 {
            break;
        }

        // Use slot value to jump to next Directory sector
        let (_, dir_start) = take(slot as u32 * sat_size)(data)?;
        // Get data of based on sector size
        let (_, value) = take(sat_size)(dir_start)?;
        // the slot value then points to the next slot
        slot_value = slot as u32;

        dir_data.append(&mut value.to_vec());
    }

    Ok((data, dir_data))
}

#[cfg(test)]
mod tests {
    use super::assemble_directory_data;
    use crate::artifacts::os::windows::ole::header::OleHeader;
    use crate::artifacts::os::windows::ole::sat::assemble_sat_data;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_assemble_directory_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/jumplists/win11/automatic/1b4dd67f29cb1962.automaticDestinations-ms");
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (input, header) = OleHeader::parse_header(&data).unwrap();
        let size: u32 = 2;
        let (_, sat) = assemble_sat_data(
            input,
            &header.msat_sectors,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        let size: u32 = 2;
        let (_, result) = assemble_directory_data(
            input,
            &sat,
            header.sector_id_chain,
            size.pow(header.sector_size as u32),
        )
        .unwrap();

        assert_eq!(result.len(), 1024);
    }
}
