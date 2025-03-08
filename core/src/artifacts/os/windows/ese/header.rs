use crate::utils::{
    nom_helper::{
        Endian, nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_sixteen_bytes,
        nom_unsigned_two_bytes,
    },
    strings::extract_utf8_string,
};
use log::error;
use nom::{bytes::complete::take, error::ErrorKind};
use std::mem::size_of;

#[derive(Debug, PartialEq)]
pub(crate) struct EseHeader {
    checksum: u32,
    signature: u32,
    file_format_version: u32,
    file_type: FileType,
    database_time: String,
    database_signature: DatabaseSig,
    database_state: DatabaseState,
    consistent_position: LogPosition,
    consistent_date_time: String,
    attach_date_time: String,
    attach_position: LogPosition,
    detach_date_time: String,
    detach_position: LogPosition,
    dbid: u32,
    log_signature: DatabaseSig,
    previous_full_backup: BackupInfo,
    previous_incremental_backup: BackupInfo,
    current_full_backup: BackupInfo,
    shadowing_disable: u32,
    last_object_id: u32,
    major_versioon: u32,
    minor_version: u32,
    build_number: u32,
    service_pack_number: u32,
    file_format_revision: u32,
    pub(crate) page_size: u32,
    repair_count: u32,
    repair_date_time: String,
    unknown: u32,
    scrub_database_time: String,
    scrub_date_time: String,
    required_log: u32,
    required_log2: u32,
    upgrade_exchange_format: u32,
    upgrade_free_pages: u32,
    upgrade_space_map_page: u32,
    current_shadow_copy_backup: BackupInfo,
    creation_file_format_version: u32,
    creation_file_format_revision: u32,
    unknown2: u128,
    old_repair_count: u32,
    ecc_fix_success_count: u32,
    last_ecc_success_date_time: String,
    old_ecc_fix_error_count: u32,
    bad_checksum_error_count: u32,
    last_bad_checksum_error_date_time: String,
    old_bad_checksum_error_count: u32,
    committed_log: u32,
    previous_shadow_copy_backup: BackupInfo,
    previous_differential_backup: BackupInfo,
    unknown3: Vec<u8>, // 40 bytes
    nls_major_version: u32,
    nls_minor_version: u32,
    unknown4: Vec<u8>, // 148 bytes
    flags: u32,
}

#[derive(Debug, PartialEq)]
enum FileType {
    Database,
    Stream,
    Unknown,
}

#[derive(Debug, PartialEq)]
struct DatabaseSig {
    random_number: u32,
    creation_date_time: String,
    netbios_computer_name: String,
}

#[derive(Debug, PartialEq)]
enum DatabaseState {
    JustCreated,
    DirtyShutdown,
    CleanShutdown,
    BeingConverted,
    ForceDetach,
    Unknown,
}

#[derive(Debug, PartialEq)]
struct LogPosition {
    block: u16,
    sector: u16,
    generation: u32,
}

#[derive(Debug, PartialEq)]
struct BackupInfo {
    backup_position: LogPosition,
    backup_creation_date_time: String,
    generation_lower_number: u32,
    generation_upper_number: u32,
}

impl EseHeader {
    /// Parse the header associated with `ESE` data. This will get us the `page size`
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], EseHeader> {
        let (input, checksum) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, signature) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let sig = 2309737967;
        if signature != sig {
            error!("[ese] Not an ESE file");
            return Err(nom::Err::Failure(nom::error::Error::new(
                input,
                ErrorKind::Fail,
            )));
        }
        let (input, file_format_version) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, file_type) = EseHeader::get_file_type(input)?;
        let (input, database_time) = EseHeader::get_database_time(input)?;
        let (input, database_signature) = EseHeader::get_database_sig(input)?;

        let (input, database_state) = EseHeader::get_database_state(input)?;
        let (input, consistent_position) = EseHeader::get_log_position(input)?;
        let (input, consistent_date_time) = EseHeader::get_log_time(input)?;

        let (input, attach_date_time) = EseHeader::get_log_time(input)?;
        let (input, attach_position) = EseHeader::get_log_position(input)?;
        let (input, detach_date_time) = EseHeader::get_log_time(input)?;
        let (input, detach_position) = EseHeader::get_log_position(input)?;

        let (input, dbid) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, log_signature) = EseHeader::get_database_sig(input)?;
        let (input, previous_full_backup) = EseHeader::get_backup_info(input)?;
        let (input, previous_incremental_backup) = EseHeader::get_backup_info(input)?;
        let (input, current_full_backup) = EseHeader::get_backup_info(input)?;

        let (input, shadowing_disable) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, last_object_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, major_versioon) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, minor_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, build_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, service_pack_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, file_format_revision) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, page_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, repair_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, repair_date_time) = EseHeader::get_log_time(input)?;
        let (input, unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, scrub_database_time) = EseHeader::get_database_time(input)?;
        let (input, scrub_date_time) = EseHeader::get_log_time(input)?;
        let (input, required_log) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, required_log2) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, upgrade_exchange_format) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, upgrade_free_pages) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, upgrade_space_map_page) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, current_shadow_copy_backup) = EseHeader::get_backup_info(input)?;
        let (input, creation_file_format_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, creation_file_format_revision) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (input, unknown2) = nom_unsigned_sixteen_bytes(input, Endian::Le)?;
        let (input, old_repair_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, ecc_fix_success_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, last_ecc_success_date_time) = EseHeader::get_log_time(input)?;
        let (input, old_ecc_fix_error_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, bad_checksum_error_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, last_bad_checksum_error_date_time) = EseHeader::get_log_time(input)?;
        let (input, old_bad_checksum_error_count) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, committed_log) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, previous_shadow_copy_backup) = EseHeader::get_backup_info(input)?;
        let (input, previous_differential_backup) = EseHeader::get_backup_info(input)?;

        let unknown3_data: u8 = 40;
        let (input, unknown3) = take(unknown3_data)(input)?;
        let (input, nls_major_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, nls_minor_version) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let unknown4_data: u8 = 148;
        let (input, unknown4) = take(unknown4_data)(input)?;
        let (input, flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let ese_header = EseHeader {
            checksum,
            signature,
            file_format_version,
            file_type,
            database_time,
            database_signature,
            database_state,
            consistent_position,
            consistent_date_time,
            attach_date_time,
            attach_position,
            detach_date_time,
            detach_position,
            dbid,
            log_signature,
            previous_full_backup,
            previous_incremental_backup,
            current_full_backup,
            shadowing_disable,
            last_object_id,
            major_versioon,
            minor_version,
            build_number,
            service_pack_number,
            file_format_revision,
            page_size,
            repair_count,
            repair_date_time,
            unknown,
            scrub_database_time,
            scrub_date_time,
            required_log,
            required_log2,
            upgrade_exchange_format,
            upgrade_free_pages,
            upgrade_space_map_page,
            current_shadow_copy_backup,
            creation_file_format_version,
            creation_file_format_revision,
            unknown2,
            old_repair_count,
            ecc_fix_success_count,
            last_ecc_success_date_time,
            old_ecc_fix_error_count,
            bad_checksum_error_count,
            last_bad_checksum_error_date_time,
            old_bad_checksum_error_count,
            committed_log,
            previous_shadow_copy_backup,
            previous_differential_backup,
            unknown3: unknown3.to_vec(),
            nls_major_version,
            nls_minor_version,
            unknown4: unknown4.to_vec(),
            flags,
        };

        Ok((input, ese_header))
    }

    /// Get type of `ESE` file either: Database or Stream or Unknown
    fn get_file_type(data: &[u8]) -> nom::IResult<&[u8], FileType> {
        let (input, file_type) = nom_unsigned_four_bytes(data, Endian::Le)?;

        let database = 0;
        let streaming = 1;
        if file_type == database {
            Ok((input, FileType::Database))
        } else if file_type == streaming {
            Ok((input, FileType::Stream))
        } else {
            Ok((input, FileType::Unknown))
        }
    }

    /// Get the state of the `ESE` database
    fn get_database_state(data: &[u8]) -> nom::IResult<&[u8], DatabaseState> {
        let (input, state) = nom_unsigned_four_bytes(data, Endian::Le)?;

        let database_state = match state {
            1 => DatabaseState::JustCreated,
            2 => DatabaseState::DirtyShutdown,
            3 => DatabaseState::CleanShutdown,
            4 => DatabaseState::BeingConverted,
            5 => DatabaseState::ForceDetach,
            _ => DatabaseState::Unknown,
        };

        Ok((input, database_state))
    }

    /// Parse the structure of the database signature
    fn get_database_sig(data: &[u8]) -> nom::IResult<&[u8], DatabaseSig> {
        let (input, random_number) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, creation_date_time) = EseHeader::get_log_time(input)?;
        let (input, netbios_data) = take(size_of::<u128>())(input)?;

        let netbios_computer_name = extract_utf8_string(netbios_data);

        let database_sig = DatabaseSig {
            random_number,
            creation_date_time,
            netbios_computer_name,
        };

        Ok((input, database_sig))
    }

    /// Parse the log position of `ESE` data
    fn get_log_position(data: &[u8]) -> nom::IResult<&[u8], LogPosition> {
        let (input, block) = nom_unsigned_two_bytes(data, Endian::Le)?;
        let (input, sector) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, generation) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let log_position = LogPosition {
            block,
            sector,
            generation,
        };
        Ok((input, log_position))
    }

    /// Get backup details of the `ESE` data
    fn get_backup_info(data: &[u8]) -> nom::IResult<&[u8], BackupInfo> {
        let (input, backup_position) = EseHeader::get_log_position(data)?;
        let (input, backup_creation_date_time) = EseHeader::get_log_time(input)?;
        let (input, generation_lower_number) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, generation_upper_number) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let backup_info = BackupInfo {
            backup_position,
            backup_creation_date_time,
            generation_lower_number,
            generation_upper_number,
        };
        Ok((input, backup_info))
    }

    /**
     * `LogTime` is odd, its eight (8) bytes but every byte represents part of the date time (YYYY-MM-DD HH:MM:SS)
     * The Year starts from 1900. Ex: A year value of zero (0) is 1900
     * Last two (2) bytes are filler
     */
    fn get_log_time(data: &[u8]) -> nom::IResult<&[u8], String> {
        let (input, seconds) = nom_unsigned_one_byte(data, Endian::Le)?;
        let (input, mins) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, hours) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, days) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, months) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, years) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _filler) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, _filler2) = nom_unsigned_one_byte(input, Endian::Le)?;

        let start_year = 1900;
        let year = years as u16 + start_year;
        Ok((
            input,
            format!("{year}-{months}-{days} {hours}:{mins}:{seconds} UTC"),
        ))
    }

    /**
     * `DatabaseTime` is odd, its eight (8) bytes but every two (2) bytes represents part of the time (HH:MM:SS)
     * First two (2) bytes are the HH
     * Second two (2) bytes are the MM
     * Third two (2) bytes are the SS
     * Last two (2) bytes are padding
     */
    pub(crate) fn get_database_time(data: &[u8]) -> nom::IResult<&[u8], String> {
        let (input, hours) = nom_unsigned_two_bytes(data, Endian::Le)?;
        let (input, mins) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, seconds) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, _padding) = nom_unsigned_two_bytes(input, Endian::Le)?;

        Ok((input, format!("{hours}:{mins}:{seconds}")))
    }
}

#[cfg(test)]
mod tests {
    use super::EseHeader;
    use crate::artifacts::os::windows::ese::header::DatabaseState::{self, DirtyShutdown};
    use crate::artifacts::os::windows::ese::header::FileType::{self, Database};
    use crate::artifacts::os::windows::ese::header::{BackupInfo, DatabaseSig, LogPosition};

    #[test]
    fn test_parse_header() {
        let test_data = [
            134, 25, 151, 65, 239, 205, 171, 137, 32, 6, 0, 0, 0, 0, 0, 0, 112, 55, 0, 0, 0, 0, 0,
            0, 122, 144, 81, 232, 41, 4, 2, 21, 10, 119, 237, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 2, 0, 0, 0, 54, 0, 103, 0, 14, 0, 0, 0, 38, 3, 1, 17, 2, 123, 109, 8,
            54, 3, 1, 17, 2, 123, 203, 4, 104, 2, 105, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 29, 12, 232, 212, 41, 4, 2, 21, 10, 119, 203, 10, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 11, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 101, 74, 0, 0, 0, 0, 0, 0, 110, 0, 0,
            0, 0, 64, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 14, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 32, 6, 0, 0, 20, 0, 0, 0, 16,
            40, 21, 29, 1, 123, 7, 8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 31, 228, 47, 4, 54, 3, 1, 17, 2, 123, 203, 100, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 134, 44, 244, 1, 54, 3, 1, 17, 2, 123, 203, 4,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 14, 0, 0, 0, 240, 0, 0, 0, 220, 35, 0,
            0, 0, 0, 0, 0, 104, 2, 105, 0, 14, 0, 0, 0, 0, 0, 0, 1,
        ];

        let (_, result) = EseHeader::parse_header(&test_data).unwrap();
        assert_eq!(
            result,
            EseHeader {
                checksum: 1100421510,
                signature: 2309737967,
                file_format_version: 1568,
                file_type: Database,
                database_time: String::from("14192:0:0"),
                database_signature: DatabaseSig {
                    random_number: 3897659514,
                    creation_date_time: String::from("2019-10-21 2:4:41 UTC"),
                    netbios_computer_name: String::from("")
                },
                database_state: DirtyShutdown,
                consistent_position: LogPosition {
                    block: 54,
                    sector: 103,
                    generation: 14
                },
                consistent_date_time: String::from("2023-2-17 1:3:38 UTC"),
                attach_date_time: String::from("2023-2-17 1:3:54 UTC"),
                attach_position: LogPosition {
                    block: 616,
                    sector: 105,
                    generation: 14
                },
                detach_date_time: String::from("1900-0-0 0:0:0 UTC"),
                detach_position: LogPosition {
                    block: 0,
                    sector: 0,
                    generation: 0
                },
                dbid: 1,
                log_signature: DatabaseSig {
                    random_number: 3571977245,
                    creation_date_time: String::from("2019-10-21 2:4:41 UTC"),
                    netbios_computer_name: String::from("")
                },
                previous_full_backup: BackupInfo {
                    backup_position: LogPosition {
                        block: 0,
                        sector: 0,
                        generation: 0
                    },
                    backup_creation_date_time: String::from("1900-0-0 0:0:0 UTC"),
                    generation_lower_number: 0,
                    generation_upper_number: 0
                },
                previous_incremental_backup: BackupInfo {
                    backup_position: LogPosition {
                        block: 0,
                        sector: 0,
                        generation: 0
                    },
                    backup_creation_date_time: String::from("1900-0-0 0:0:0 UTC"),
                    generation_lower_number: 0,
                    generation_upper_number: 0
                },
                current_full_backup: BackupInfo {
                    backup_position: LogPosition {
                        block: 0,
                        sector: 0,
                        generation: 0
                    },
                    backup_creation_date_time: String::from("1900-0-0 0:0:0 UTC"),
                    generation_lower_number: 0,
                    generation_upper_number: 0
                },
                shadowing_disable: 0,
                last_object_id: 11,
                major_versioon: 10,
                minor_version: 0,
                build_number: 19045,
                service_pack_number: 0,
                file_format_revision: 110,
                page_size: 16384,
                repair_count: 0,
                repair_date_time: String::from("1900-0-0 0:0:0 UTC"),
                unknown: 0,
                scrub_database_time: String::from("0:0:0"),
                scrub_date_time: String::from("1900-0-0 0:0:0 UTC"),
                required_log: 0,
                required_log2: 0,
                upgrade_exchange_format: 0,
                upgrade_free_pages: 0,
                upgrade_space_map_page: 0,
                current_shadow_copy_backup: BackupInfo {
                    backup_position: LogPosition {
                        block: 0,
                        sector: 0,
                        generation: 14
                    },
                    backup_creation_date_time: String::from("1900-0-0 0:0:14 UTC"),
                    generation_lower_number: 0,
                    generation_upper_number: 0
                },
                creation_file_format_version: 0,
                creation_file_format_revision: 0,
                unknown2: 0,
                old_repair_count: 1568,
                ecc_fix_success_count: 20,
                last_ecc_success_date_time: String::from("2023-1-29 21:40:16 UTC"),
                old_ecc_fix_error_count: 0,
                bad_checksum_error_count: 0,
                last_bad_checksum_error_date_time: String::from("1900-0-0 0:0:0 UTC"),
                old_bad_checksum_error_count: 0,
                committed_log: 0,
                previous_shadow_copy_backup: BackupInfo {
                    backup_position: LogPosition {
                        block: 0,
                        sector: 0,
                        generation: 0
                    },
                    backup_creation_date_time: String::from("1900-0-0 0:0:0 UTC"),
                    generation_lower_number: 0,
                    generation_upper_number: 0
                },
                previous_differential_backup: BackupInfo {
                    backup_position: LogPosition {
                        block: 0,
                        sector: 0,
                        generation: 0
                    },
                    backup_creation_date_time: String::from("1900-14-0 0:0:0 UTC"),
                    generation_lower_number: 0,
                    generation_upper_number: 0
                },
                unknown3: vec![
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0
                ],
                nls_major_version: 0,
                nls_minor_version: 0,
                unknown4: vec![
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 31, 228, 47, 4, 54, 3, 1, 17, 2, 123, 203, 100, 0,
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 134, 44, 244, 1, 54, 3, 1, 17, 2,
                    123, 203, 4
                ],
                flags: 0
            }
        )
    }

    #[test]
    fn test_get_file_type() {
        let test = [1, 0, 0, 0];
        let (_, results) = EseHeader::get_file_type(&test).unwrap();
        assert_eq!(results, FileType::Stream);
    }

    #[test]
    fn test_get_database_state() {
        let test = [1, 0, 0, 0];
        let (_, results) = EseHeader::get_database_state(&test).unwrap();
        assert_eq!(results, DatabaseState::JustCreated);
    }

    #[test]
    fn test_get_database_sig() {
        let test = [
            122, 144, 81, 232, 41, 4, 2, 21, 10, 119, 237, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 54, 0, 103, 0, 14, 0, 0, 0, 38, 3, 1, 17, 2, 123, 109, 8, 54,
        ];
        let (_, results) = EseHeader::get_database_sig(&test).unwrap();
        assert_eq!(results.creation_date_time, "2019-10-21 2:4:41 UTC");
        assert_eq!(results.netbios_computer_name, "");
        assert_eq!(results.random_number, 3897659514);
    }

    #[test]
    fn test_get_log_position() {
        let test = [1, 0, 0, 0, 0, 0, 0, 0];
        let (_, results) = EseHeader::get_log_position(&test).unwrap();
        assert_eq!(results.block, 1);
        assert_eq!(results.sector, 0);
        assert_eq!(results.generation, 0);
    }

    #[test]
    fn test_get_backup_info() {
        let test = [
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (_, results) = EseHeader::get_backup_info(&test).unwrap();
        assert_eq!(results.backup_creation_date_time, "1900-0-0 0:0:0 UTC");
        assert_eq!(results.generation_lower_number, 0);
        assert_eq!(results.generation_upper_number, 0);
    }

    #[test]
    fn test_get_log_time() {
        let test = [41, 4, 2, 21, 10, 119, 237, 10];
        let (_, results) = EseHeader::get_log_time(&test).unwrap();
        assert_eq!(results, "2019-10-21 2:4:41 UTC");
    }

    #[test]
    fn test_get_database_time() {
        let test = [112, 55, 0, 0, 0, 0, 0, 0];
        let (_, results) = EseHeader::get_database_time(&test).unwrap();
        assert_eq!(results, "14192:0:0");
    }
}
