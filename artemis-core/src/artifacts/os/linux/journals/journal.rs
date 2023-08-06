use super::{
    error::JournalError,
    header::{IncompatFlags, JournalHeader},
    objects::{
        array::EntryArray,
        entry::Entry,
        header::{ObjectHeader, ObjectType},
    },
};
use crate::{
    artifacts::os::linux::artifacts::output_data, filesystem::files::file_reader,
    utils::artemis_toml::Output,
};
use log::error;
use serde::Serialize;
use std::{collections::HashMap, fs::File, io::Read};

#[derive(Debug, Serialize)]
pub(crate) struct Journal {
    uid: u32,
    gid: u32,
    pid: usize,
    pub(crate) comm: String,
    priority: Priority,
    syslog_facility: Facility,
    thread_id: usize,
    syslog_identifier: String,
    executable: String,
    cmdline: String,
    cap_effective: String,
    audit_session: usize,
    audit_loginuid: u32,
    systemd_cgroup: String,
    systemd_owner_uid: usize,
    systemd_unit: String,
    systemd_user_unit: String,
    systemd_slice: String,
    systemd_user_slice: String,
    systemd_invocation_id: String,
    boot_id: String,
    machine_id: String,
    hostname: String,
    runtime_scope: String,
    source_realtime: u64,
    realtime: u64,
    transport: String,
    message: String,
    message_id: String,
    unit_result: String,
    code_line: usize,
    code_function: String,
    code_file: String,
    user_invocation_id: String,
    user_unit: String,
    custom: HashMap<String, String>,
    seqnum: u64,
}

// https://wiki.archlinux.org/title/Systemd/Journal
#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum Priority {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Informational,
    Debug,
    None,
}

// https://wiki.archlinux.org/title/Systemd/Journal
#[derive(Debug, Serialize, PartialEq)]
pub(crate) enum Facility {
    Kernel,
    User,
    Mail,
    Daemon,
    Authentication,
    Syslog,
    LinePrinter,
    News,
    Uucp,
    Clock,
    AuthenticationPriv,
    Ftp,
    Ntp,
    LogAudit,
    LogAlert,
    Cron,
    Local0,
    Local1,
    Local2,
    Local3,
    Local4,
    Local5,
    Local6,
    Local7,
    None,
}

impl Journal {
    /// Parse provided `Journal` file path. Will output results when finished. Use `parse_journal_file` if you want the results
    pub(crate) fn parse_journal(
        path: &str,
        output: &mut Output,
        filter: &bool,
        start_time: &u64,
    ) -> Result<(), JournalError> {
        let reader_result = file_reader(path);
        let mut reader = match reader_result {
            Ok(result) => result,
            Err(err) => {
                error!("[journal] Could not create reader for file {path}: {err:?}");
                return Err(JournalError::ReaderError);
            }
        };

        // We technically only need first 232 bytes but version 252 is 264 bytes in size
        let mut header_buff = [0; 264];
        if reader.read(&mut header_buff).is_err() {
            error!("[journal] Could not read file header {path}");
            return Err(JournalError::ReadError);
        }

        let header_result = JournalHeader::parse_header(&header_buff);
        let journal_header = match header_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[journal] Could not parser file header {path}");
                return Err(JournalError::JournalHeader);
            }
        };

        let is_compact = journal_header
            .incompatible_flags
            .contains(&IncompatFlags::Compact);

        Journal::get_entries(
            &mut reader,
            journal_header.entry_array_offset,
            is_compact,
            output,
            filter,
            start_time,
        )?;

        Ok(())
    }

    /// Parse provided `Journal` file path. Returns parsed entries.
    pub(crate) fn parse_journal_file(path: &str) -> Result<Vec<Journal>, JournalError> {
        let reader_result = file_reader(path);
        let mut reader = match reader_result {
            Ok(result) => result,
            Err(err) => {
                error!("[journal] Could not create reader for file {path}: {err:?}");
                return Err(JournalError::ReaderError);
            }
        };

        // We technically only need first 232 bytes but version 252 is 264 bytes in size
        let mut header_buff = [0; 264];
        if reader.read(&mut header_buff).is_err() {
            error!("[journal] Could not read file header {path}");
            return Err(JournalError::ReadError);
        }

        let header_result = JournalHeader::parse_header(&header_buff);
        let journal_header = match header_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[journal] Could not parser file header {path}");
                return Err(JournalError::JournalHeader);
            }
        };

        let is_compact = journal_header
            .incompatible_flags
            .contains(&IncompatFlags::Compact);

        let mut offset = journal_header.entry_array_offset;
        // Track offsets to make sure we do not encounter infinite loops
        let mut offset_tracker: HashMap<u64, bool> = HashMap::new();
        offset_tracker.insert(offset, false);

        let last_entry = 0;

        let mut entries = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset: 0,
        };

        while offset != last_entry {
            let object_header = ObjectHeader::parse_header(&mut reader, offset)?;
            if object_header.obj_type != ObjectType::EntryArray {
                error!("[journal] Did not get Entry Array type at entry_array_offset. Got: {:?}. Exiting early", object_header.obj_type);
                break;
            }

            let entry_result =
                EntryArray::walk_entries(&mut reader, &object_header.payload, is_compact);
            let mut entry_array = match entry_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[journal] Could walk journal entries. Exiting early");
                    break;
                }
            };

            entries.entries.append(&mut entry_array.entries);
            offset = entry_array.next_entry_array_offset;

            if offset_tracker.get(&offset).is_some() {
                error!("[journal] Found recursive offset. Exiting now");
                break;
            }
        }

        let messages = Journal::parse_messages(&entries.entries);

        Ok(messages)
    }

    /// Loop through the `Journal` entries and get the data
    fn get_entries(
        reader: &mut File,
        array_offset: u64,
        is_compact: bool,
        output: &mut Output,
        filter: &bool,
        start_time: &u64,
    ) -> Result<(), JournalError> {
        let mut offset = array_offset;
        let last_entry = 0;

        let mut entries = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset: 0,
        };

        let limit = 100000;
        // Track offsets to make sure we do not encounter infinite loops
        let mut offset_tracker: HashMap<u64, bool> = HashMap::new();
        offset_tracker.insert(offset, false);

        while offset != last_entry {
            let object_header = ObjectHeader::parse_header(reader, offset)?;
            if object_header.obj_type != ObjectType::EntryArray {
                error!("[journal] Did not get Entry Array type at entry_array_offset. Got: {:?}. Exiting early", object_header.obj_type);
                break;
            }

            let entry_result = EntryArray::walk_entries(reader, &object_header.payload, is_compact);
            let mut entry_array = match entry_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[journal] Could walk journal entries. Exiting early");
                    break;
                }
            };
            entries.entries.append(&mut entry_array.entries);
            offset = entry_array.next_entry_array_offset;
            if offset_tracker.get(&offset).is_some() {
                error!("[journal] Found recursive offset. Exiting now");
                break;
            }

            offset_tracker.insert(offset, false);

            // The Journal file can be configured to be very large. Default size is usually ~10MB
            // To limit memory usage we output every 100k entries

            if entries.entries.len() >= limit {
                let messages = Journal::parse_messages(&entries.entries);
                let serde_data_result = serde_json::to_value(messages);
                let serde_data = match serde_data_result {
                    Ok(results) => results,
                    Err(err) => {
                        error!("[journal] Failed to serialize journal data: {err:?}");
                        continue;
                    }
                };

                let _ = output_data(&serde_data, "journal", output, start_time, filter);
                // Now empty the vec
                entries.entries = Vec::new();
            }
        }

        let messages = Journal::parse_messages(&entries.entries);
        let serde_data_result = serde_json::to_value(messages);
        let serde_data: serde_json::Value = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[journal] Failed to serialize last journal data: {err:?}");
                return Err(JournalError::Serialize);
            }
        };

        let _ = output_data(&serde_data, "journal", output, start_time, filter);
        Ok(())
    }

    /// Parse the `Journal` message
    fn parse_messages(entries: &[Entry]) -> Vec<Journal> {
        let mut journal_entries: Vec<Journal> = Vec::new();

        for entry in entries {
            let mut journal = Journal {
                uid: 0,
                gid: 0,
                pid: 0,
                comm: String::new(),
                priority: Priority::None,
                syslog_facility: Facility::None,
                thread_id: 0,
                syslog_identifier: String::new(),
                executable: String::new(),
                cmdline: String::new(),
                cap_effective: String::new(),
                audit_session: 0,
                audit_loginuid: 0,
                systemd_cgroup: String::new(),
                systemd_owner_uid: 0,
                systemd_unit: String::new(),
                systemd_user_unit: String::new(),
                systemd_slice: String::new(),
                systemd_user_slice: String::new(),
                systemd_invocation_id: String::new(),
                boot_id: String::new(),
                machine_id: String::new(),
                hostname: String::new(),
                runtime_scope: String::new(),
                source_realtime: 0,
                realtime: entry.realtime,
                seqnum: entry.seqnum,
                transport: String::new(),
                message: String::new(),
                message_id: String::new(),
                unit_result: String::new(),
                code_line: 0,
                code_function: String::new(),
                code_file: String::new(),
                user_invocation_id: String::new(),
                user_unit: String::new(),
                custom: HashMap::new(),
            };

            for data in &entry.data_objects {
                if data.message.starts_with("_PID=") {
                    if let Some((_, pid)) = data.message.split_once('=') {
                        journal.pid = pid.parse::<usize>().unwrap_or_default();
                    }
                } else if data.message.starts_with("_TRANSPORT=") {
                    if let Some((_, transport)) = data.message.split_once('=') {
                        journal.transport = transport.to_string();
                    }
                } else if data.message.starts_with("_UID=") {
                    if let Some((_, uid)) = data.message.split_once('=') {
                        journal.uid = uid.parse::<u32>().unwrap_or_default();
                    }
                } else if data.message.starts_with("_GID=") {
                    if let Some((_, gid)) = data.message.split_once('=') {
                        journal.gid = gid.parse::<u32>().unwrap_or_default();
                    }
                } else if data.message.starts_with("_COMM=") {
                    if let Some((_, comm)) = data.message.split_once('=') {
                        journal.comm = comm.to_string();
                    }
                } else if data.message.starts_with("_EXE=") {
                    if let Some((_, exe)) = data.message.split_once('=') {
                        journal.executable = exe.to_string();
                    }
                } else if data.message.starts_with("_CMDLINE=") {
                    if let Some((_, cmdline)) = data.message.split_once('=') {
                        journal.cmdline = cmdline.to_string();
                    }
                } else if data.message.starts_with("_CAP_EFFECTIVE=") {
                    if let Some((_, cap)) = data.message.split_once('=') {
                        journal.cap_effective = cap.to_string();
                    }
                } else if data.message.starts_with("_AUDIT_SESSION=") {
                    if let Some((_, session)) = data.message.split_once('=') {
                        journal.audit_session = session.parse::<usize>().unwrap_or_default();
                    }
                } else if data.message.starts_with("_SYSTEMD_INVOCATION_ID=") {
                    if let Some((_, invoc)) = data.message.split_once('=') {
                        journal.systemd_invocation_id = invoc.to_string();
                    }
                } else if data.message.starts_with("_AUDIT_LOGINUID=") {
                    if let Some((_, audit)) = data.message.split_once('=') {
                        journal.audit_loginuid = audit.parse::<u32>().unwrap_or_default();
                    }
                } else if data.message.starts_with("_SYSTEMD_CGROUP=") {
                    if let Some((_, cgroup)) = data.message.split_once('=') {
                        journal.systemd_cgroup = cgroup.to_string();
                    }
                } else if data.message.starts_with("_SYSTEMD_OWNER_UID=") {
                    if let Some((_, uid)) = data.message.split_once('=') {
                        journal.systemd_owner_uid = uid.parse::<usize>().unwrap_or_default();
                    }
                } else if data.message.starts_with("_SYSTEMD_UNIT=") {
                    if let Some((_, unit)) = data.message.split_once('=') {
                        journal.systemd_unit = unit.to_string();
                    }
                } else if data.message.starts_with("_SYSTEMD_USER_UNIT=") {
                    if let Some((_, unit)) = data.message.split_once('=') {
                        journal.systemd_user_unit = unit.to_string();
                    }
                } else if data.message.starts_with("_SYSTEMD_SLICE=") {
                    if let Some((_, slice)) = data.message.split_once('=') {
                        journal.systemd_slice = slice.to_string();
                    }
                } else if data.message.starts_with("_SYSTEMD_USER_SLICE=") {
                    if let Some((_, slice)) = data.message.split_once('=') {
                        journal.systemd_user_slice = slice.to_string();
                    }
                } else if data.message.starts_with("_BOOT_ID=") {
                    if let Some((_, boot)) = data.message.split_once('=') {
                        journal.boot_id = boot.to_string();
                    }
                } else if data.message.starts_with("_MACHINE_ID=") {
                    if let Some((_, id)) = data.message.split_once('=') {
                        journal.machine_id = id.to_string();
                    }
                } else if data.message.starts_with("_HOSTNAME=") {
                    if let Some((_, host)) = data.message.split_once('=') {
                        journal.hostname = host.to_string();
                    }
                } else if data.message.starts_with("_RUNTIME_SCOPE=") {
                    if let Some((_, scope)) = data.message.split_once('=') {
                        journal.runtime_scope = scope.to_string();
                    }
                } else if data.message.starts_with("_SOURCE_REALTIME_TIMESTAMP=") {
                    if let Some((_, timestamp)) = data.message.split_once('=') {
                        journal.source_realtime = timestamp.parse::<u64>().unwrap_or_default();
                    }
                } else if data.message.starts_with("PRIORITY=") {
                    if let Some((_, priority)) = data.message.split_once('=') {
                        journal.priority =
                            Journal::get_priority(&priority.parse::<u32>().unwrap_or_default());
                    }
                } else if data.message.starts_with("SYSLOG_FACILITY=") {
                    if let Some((_, facility)) = data.message.split_once('=') {
                        journal.syslog_facility =
                            Journal::get_facility(&facility.parse::<u32>().unwrap_or_default());
                    }
                } else if data.message.starts_with("TID=") {
                    if let Some((_, tid)) = data.message.split_once('=') {
                        journal.thread_id = tid.parse::<usize>().unwrap_or_default();
                    }
                } else if data.message.starts_with("SYSLOG_IDENTIFIER=") {
                    if let Some((_, id)) = data.message.split_once('=') {
                        journal.syslog_identifier = id.to_string();
                    }
                } else if data.message.starts_with("CODE_FILE=") {
                    if let Some((_, code)) = data.message.split_once('=') {
                        journal.code_file = code.to_string();
                    }
                } else if data.message.starts_with("USER_INVOCATION_ID=") {
                    if let Some((_, id)) = data.message.split_once('=') {
                        journal.user_invocation_id = id.to_string();
                    }
                } else if data.message.starts_with("USER_UNIT=") {
                    if let Some((_, unit)) = data.message.split_once('=') {
                        journal.user_unit = unit.to_string();
                    }
                } else if data.message.starts_with("CODE_LINE=") {
                    if let Some((_, code)) = data.message.split_once('=') {
                        journal.code_line = code.parse::<usize>().unwrap_or_default();
                    }
                } else if data.message.starts_with("CODE_FUNC=") {
                    if let Some((_, code)) = data.message.split_once('=') {
                        journal.code_function = code.to_string();
                    }
                } else if data.message.starts_with("MESSAGE_ID=") {
                    if let Some((_, message)) = data.message.split_once('=') {
                        journal.message_id = message.to_string();
                    }
                } else if data.message.starts_with("MESSAGE=") {
                    if let Some((_, message)) = data.message.split_once('=') {
                        journal.message = message.to_string();
                    }
                } else if data.message.starts_with("UNIT_RESULT=") {
                    if let Some((_, unit)) = data.message.split_once('=') {
                        journal.unit_result = unit.to_string();
                    }
                } else if let Some((field, field_data)) = data.message.split_once('=') {
                    journal
                        .custom
                        .insert(field.to_string(), field_data.to_string());
                }
            }

            journal_entries.push(journal);
        }
        journal_entries
    }

    /// Get message priority
    fn get_priority(priority: &u32) -> Priority {
        match priority {
            0 => Priority::Emergency,
            1 => Priority::Alert,
            2 => Priority::Critical,
            3 => Priority::Error,
            4 => Priority::Warning,
            5 => Priority::Notice,
            6 => Priority::Informational,
            7 => Priority::Debug,
            _ => Priority::None,
        }
    }

    /// Get syslog facility if any
    fn get_facility(facility: &u32) -> Facility {
        match facility {
            0 => Facility::Kernel,
            1 => Facility::User,
            2 => Facility::Mail,
            3 => Facility::Daemon,
            4 => Facility::Authentication,
            5 => Facility::Syslog,
            6 => Facility::LinePrinter,
            7 => Facility::News,
            8 => Facility::Uucp,
            9 => Facility::Clock,
            10 => Facility::AuthenticationPriv,
            11 => Facility::Ftp,
            12 => Facility::Ntp,
            13 => Facility::LogAudit,
            14 => Facility::LogAlert,
            15 => Facility::Cron,
            16 => Facility::Local0,
            17 => Facility::Local1,
            18 => Facility::Local2,
            19 => Facility::Local3,
            20 => Facility::Local4,
            21 => Facility::Local5,
            22 => Facility::Local6,
            23 => Facility::Local7,
            _ => Facility::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Journal;
    use crate::{
        artifacts::os::linux::journals::{
            journal::{Facility, Priority},
            objects::{
                array::EntryArray,
                header::{ObjectHeader, ObjectType},
            },
        },
        filesystem::files::file_reader,
        utils::artemis_toml::Output,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_parse_journal() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");
        let mut output = output_options("journal_test", "local", "./tmp", false);

        Journal::parse_journal(
            &test_location.display().to_string(),
            &mut output,
            &false,
            &0,
        )
        .unwrap();
    }

    #[test]
    fn test_get_entries() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut output = output_options("journal_test", "local", "./tmp", false);
        Journal::get_entries(&mut reader, 3738992, true, &mut output, &false, &0).unwrap();
    }

    #[test]
    fn test_parse_messages() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut offset = 3738992;
        let last_entry = 0;

        let mut entries = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset: 0,
        };

        while offset != last_entry {
            let object_header = ObjectHeader::parse_header(&mut reader, offset).unwrap();
            if object_header.obj_type != ObjectType::EntryArray {
                break;
            }

            let (_, mut entry_array) =
                EntryArray::walk_entries(&mut reader, &object_header.payload, true).unwrap();
            entries.entries.append(&mut entry_array.entries);
            offset = entry_array.next_entry_array_offset;
        }

        assert_eq!(entries.entries.len(), 410);
    }

    #[test]
    fn test_get_priority() {
        let test = 1;
        let result = Journal::get_priority(&test);
        assert_eq!(result, Priority::Alert);
    }

    #[test]
    fn test_get_facility() {
        let test = 1;
        let result = Journal::get_facility(&test);
        assert_eq!(result, Facility::User);
    }

    #[test]
    fn test_parse_journal_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let result = Journal::parse_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 410);
    }

    #[test]
    fn test_parse_journal_bad_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/bad_recursive.journal");

        let result = Journal::parse_journal_file(&test_location.display().to_string()).unwrap();
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_parse_journal_bad() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/bad_recursive.journal");
        let mut output = output_options("journal_test", "local", "./tmp", false);

        Journal::parse_journal(
            &test_location.display().to_string(),
            &mut output,
            &false,
            &0,
        )
        .unwrap();
    }
}
