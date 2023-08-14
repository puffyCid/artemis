use super::{
    directory::{assemble_ole_data, parse_directory},
    header::OleHeader,
    sat::assemble_sat_data,
    ssat::assemble_ssat_data,
};
use crate::artifacts::os::windows::ole::ssat::add_ssat_slots;
use log::error;

#[derive(Debug)]
pub(crate) struct OleData {
    pub(crate) name: String,
    /**Raw bytes associated with OLE, includes slack space */
    pub(crate) data: Vec<u8>,
    pub(crate) directory_type: DirectoryType,
}

#[derive(Debug, PartialEq)]
pub(crate) enum DirectoryType {
    Empty,
    Storage,
    Stream,
    LockBytes,
    Property,
    Root,
    Unknown,
}

impl OleData {
    /// Parse Object Link Embedded (OLE) data and return Vector of `OleData`
    pub(crate) fn parse_ole(data: &[u8]) -> nom::IResult<&[u8], Vec<OleData>> {
        let (input, header) = OleHeader::parse_header(data)?;

        // All Sector sizes are actually exponents to apply to two (2)
        let size: u32 = 2;
        let sector_size = size.pow(header.sector_size as u32);
        let small_size = size.pow(header.short_sector_size as u32);
        let (_, sat_slots) = assemble_sat_data(input, &header.msat_sectors, sector_size)?;

        let no_ssat = -2;
        let ssat_slots = if header.sector_id_ssat != no_ssat {
            let (_, mut ssat_slots) =
                assemble_ssat_data(input, header.sector_id_ssat, sector_size)?;
            let (_, mut additional_ssat) =
                add_ssat_slots(input, &sat_slots, header.sector_id_ssat as u32, sector_size)?;
            ssat_slots.append(&mut additional_ssat);
            ssat_slots
        } else {
            Vec::new()
        };

        let (_, directory_data) =
            assemble_ole_data(input, &sat_slots, header.sector_id_chain, sector_size)?;
        let dir_result = parse_directory(&directory_data);
        let directories = match dir_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[ole] Failed to get OLE Directories");
                Vec::new()
            }
        };

        let mut root_data = Vec::new();

        // Now get RootDirectory data. Root data is used for Directory data strams smaller than the sector stream (4096)
        for directory in directories.iter() {
            if directory.directory_type != DirectoryType::Root || directory.sector_id < 0 {
                continue;
            }

            let (_, results) =
                assemble_ole_data(input, &sat_slots, directory.sector_id as u32, sector_size)?;
            root_data = results;
        }

        // Now have all data needed to assemble all of the OLE data! Includes slack data!
        let mut ole_vec = Vec::new();
        for directory in directories {
            // Cant get data if sector_id is negative
            if directory.sector_id < 0 {
                continue;
            }

            // Already got Root data
            if directory.directory_type == DirectoryType::Root {
                let ole_data = OleData {
                    name: directory.name,
                    data: root_data.clone(),
                    directory_type: directory.directory_type,
                };
                ole_vec.push(ole_data);
                continue;
            }

            let empty = 0;
            // Skip empty directories
            if directory.directory_size == empty {
                let ole_data = OleData {
                    name: directory.name,
                    data: Vec::new(),
                    directory_type: directory.directory_type,
                };
                ole_vec.push(ole_data);
                continue;
            }

            // If the directory size is smaller than default stream size. Data is stored in SSAT
            if directory.directory_size < header.min_stream_size {
                let dir_results = assemble_ole_data(
                    &root_data,
                    &ssat_slots,
                    directory.sector_id as u32,
                    small_size,
                );
                let results = match dir_results {
                    Ok((_, results)) => results,
                    Err(_err) => {
                        error!(
                            "[ole] Could not parse SSAT data associated with directory: {}",
                            directory.name
                        );
                        continue;
                    }
                };

                let ole_data = OleData {
                    name: directory.name,
                    data: results.clone(),
                    directory_type: directory.directory_type,
                };

                ole_vec.push(ole_data);
            } else {
                let dir_results =
                    assemble_ole_data(input, &sat_slots, directory.sector_id as u32, sector_size);
                let results = match dir_results {
                    Ok((_, results)) => results,
                    Err(_err) => {
                        error!(
                            "[ole] Could not parse SAT data associated with directory: {}",
                            directory.name
                        );
                        continue;
                    }
                };

                let ole_data = OleData {
                    name: directory.name,
                    data: results.clone(),
                    directory_type: directory.directory_type,
                };

                ole_vec.push(ole_data);
            }
        }

        Ok((data, ole_vec))
    }
}

#[cfg(test)]
mod tests {
    use super::OleData;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_parse_ole() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push(
            "tests/test_data/dfir/windows/jumplists/win7/1b4dd67f29cb1962.automaticDestinations-ms",
        );
        let data = read_file(&test_location.display().to_string()).unwrap();

        let (_, results) = OleData::parse_ole(&data).unwrap();
        assert_eq!(results.len(), 8);
        assert_eq!(results[1].data.len(), 448);
    }

    #[test]
    fn test_parse_ole_large() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/jumplists/win11/automatic/3d2110c4a0cb6d15.automaticDestinations-ms");
        let file_data = read_file(&test_location.display().to_string()).unwrap();
        let (_, results) = OleData::parse_ole(&file_data).unwrap();
        assert_eq!(results.len(), 44);

        for result in results {
            if result.name == "DestList" {
                assert_eq!(result.data.len(), 12458);
            } else if result.name == "26" {
                assert_eq!(result.data.len(), 832);
            }
        }
    }
}
