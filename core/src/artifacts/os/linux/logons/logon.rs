use crate::utils::{
    nom_helper::{nom_signed_four_bytes, nom_signed_two_bytes, nom_unsigned_four_bytes, Endian},
    strings::extract_utf8_string,
    time::unixepoch_to_iso,
};
use log::error;
use nom::{
    branch::alt,
    bytes::complete::{take, take_until},
    Parser,
};
use serde::Serialize;
use std::{
    fs::File,
    io::Read,
    mem::size_of,
    net::{Ipv4Addr, Ipv6Addr},
};

#[derive(Debug, Serialize)]
pub(crate) struct Logon {
    logon_type: LogonType,
    pid: u32,
    terminal: String,
    terminal_id: u32,
    username: String,
    hostname: String,
    termination_status: i16,
    exit_status: i16,
    session: i32,
    timestamp: String,
    microseconds: i32,
    ip: String,
    status: Status,
}

#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum LogonType {
    Unknown,
    RunLevel,
    BootTime,
    NewTime,
    OldTime,
    InitProcess,
    LoginProcess,
    UserProcess,
    DeadProcess,
    Accounting,
}

#[derive(Debug, Serialize, PartialEq, Clone)]
pub(crate) enum Status {
    Success,
    Failed,
}

impl Logon {
    /// Stream the logon info
    pub(crate) fn logon_reader(reader: &mut File, status: &Status) -> Vec<Logon> {
        let mut logon_buff = [0; 384];
        let mut logon_size = logon_buff.len();
        let mut logons = Vec::new();

        while logon_size == logon_buff.len() {
            let read_result = reader.read(&mut logon_buff);
            let read_size = match read_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[logons] Could not read logon data: {err:?}");
                    break;
                }
            };
            logon_size = read_size;

            // Check to make we read 384 bytes
            if logon_size != logon_buff.len() {
                break;
            }

            let result = Logon::parse_logon(&logon_buff, status, &mut logons);
            if result.is_err() {
                error!("[logons] Could not parse logon file");
            }
        }

        logons
    }

    /// Parse utmp, wtmp, or btmp files and pull `Logon` info
    fn parse_logon<'a>(
        data: &'a [u8],
        status: &Status,
        logons: &mut Vec<Logon>,
    ) -> nom::IResult<&'a [u8], ()> {
        let (remaining, logon_type) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (remaining, pid) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        let terminal_size: u8 = 32;
        let (remaining, term_data) = take(terminal_size)(remaining)?;
        let (remaining, terminal_id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

        let (remaining, username_data) = take(terminal_size)(remaining)?;

        let hostname_size: u16 = 256;
        let (remaining, hostname_data) = take(hostname_size)(remaining)?;

        let (remaining, termination_status) = nom_signed_two_bytes(remaining, Endian::Le)?;
        let (remaining, exit_status) = nom_signed_two_bytes(remaining, Endian::Le)?;

        let (remaining, session) = nom_signed_four_bytes(remaining, Endian::Le)?;
        let (remaining, timestamp) = nom_signed_four_bytes(remaining, Endian::Le)?;
        let (remaining, microseconds) = nom_signed_four_bytes(remaining, Endian::Le)?;
        let (remaining, ip_data) = take(size_of::<u128>())(remaining)?;

        let reserved_size: u8 = 20;
        let (remaining, _) = take(reserved_size)(remaining)?;

        let ipv4_end = [0, 0, 0, 0];
        let (_, ip_data) =
            alt((take_until(ipv4_end.as_slice()), take(ip_data.len()))).parse(ip_data)?;

        // IP source is either IPv4 or IPv6. Based on data we nommed, convert to IP string. If the data is empty the IP is 0.0.0.0
        let ip = if ip_data.len() == 4 {
            Ipv4Addr::new(ip_data[0], ip_data[1], ip_data[2], ip_data[3]).to_string()
        } else if ip_data.len() == 16 {
            let mut ipv6_data: Vec<u16> = Vec::new();
            // Convert data to IPv6 (&[u16])
            let min_byte_size = 2;
            for wide_char in data.chunks(min_byte_size) {
                ipv6_data.push(u16::from_ne_bytes([wide_char[0], wide_char[1]]));
            }
            Ipv6Addr::new(
                ipv6_data[0],
                ipv6_data[1],
                ipv6_data[2],
                ipv6_data[3],
                ipv6_data[4],
                ipv6_data[5],
                ipv6_data[6],
                ipv6_data[7],
            )
            .to_string()
        } else {
            String::from("0.0.0.0")
        };

        let logon = Logon {
            logon_type: Logon::get_logon_type(&logon_type),
            pid,
            terminal: extract_utf8_string(term_data),
            terminal_id,
            username: extract_utf8_string(username_data),
            hostname: extract_utf8_string(hostname_data),
            termination_status,
            exit_status,
            session,
            timestamp: unixepoch_to_iso(&(timestamp as i64)),
            microseconds,
            ip,
            status: status.clone(),
        };

        logons.push(logon);

        Ok((remaining, ()))
    }

    /// Get the `LogonType`
    fn get_logon_type(logon: &u32) -> LogonType {
        match logon {
            1 => LogonType::RunLevel,
            2 => LogonType::BootTime,
            3 => LogonType::NewTime,
            4 => LogonType::OldTime,
            5 => LogonType::InitProcess,
            6 => LogonType::LoginProcess,
            7 => LogonType::UserProcess,
            8 => LogonType::DeadProcess,
            9 => LogonType::Accounting,
            _ => LogonType::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Logon;
    use crate::{
        artifacts::os::linux::logons::logon::{LogonType, Status},
        filesystem::files::{file_reader, read_file},
    };
    use std::path::PathBuf;

    #[test]
    fn test_logon_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/logons/ubuntu18.04/wtmp");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let results = Logon::logon_reader(&mut reader, &Status::Success);
        assert_eq!(results.len(), 13);

        assert_eq!(results[4].hostname, "5.4.0-84-generic");
        assert_eq!(results[4].timestamp, "2023-07-04T06:13:44.000Z");
        assert_eq!(results[0].terminal, "~");
    }

    #[test]
    fn test_parse_logon() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/logons/ubuntu18.04/wtmp");

        let data = read_file(&test_location.display().to_string()).unwrap();
        let mut results = Vec::new();
        let (_, _) = Logon::parse_logon(&data, &Status::Success, &mut results).unwrap();
        assert_eq!(results.len(), 1);
    }

    #[test]
    fn test_get_logon_type() {
        let test = 9;
        let logon_type = Logon::get_logon_type(&test);
        assert_eq!(logon_type, LogonType::Accounting);
    }
}
