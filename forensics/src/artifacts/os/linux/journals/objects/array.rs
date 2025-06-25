use super::{
    entry::Entry,
    header::{ObjectHeader, ObjectType},
};
use crate::{
    artifacts::os::linux::artifacts::output_data,
    structs::toml::Output,
    utils::{
        nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
        time::unixepoch_microseconds_to_iso,
    },
};
use common::linux::{Facility, Journal, Priority};
use log::{error, warn};
use std::{collections::HashMap, fs::File};

#[derive(Debug)]
pub(crate) struct EntryArray {
    /**Log messages */
    pub(crate) entries: Vec<Entry>,
    pub(crate) next_entry_array_offset: u64,
}

impl EntryArray {
    /// Walk through Array of entries and output the results
    /// Returns offset to next array of data
    pub(crate) fn walk_entries<'a>(
        reader: &mut File,
        data: &'a [u8],
        is_compact: &bool,
        output: &mut Output,
        filter: &bool,
        start_time: &u64,
    ) -> nom::IResult<&'a [u8], u64> {
        let (mut input, next_entry_array_offset) = nom_unsigned_eight_bytes(data, Endian::Le)?;

        let min_size = 4;
        let mut entry_array = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset,
        };
        let last_entry = 0;
        // Limit memory usage by outputting every 1k log entries
        let limit = 1000;
        while !input.is_empty() && input.len() >= min_size {
            let (remaining_input, offset) = if *is_compact {
                let (remaining_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
                (remaining_input, offset as u64)
            } else {
                nom_unsigned_eight_bytes(input, Endian::Le)?
            };
            input = remaining_input;

            if offset == last_entry {
                break;
            }
            let object_result = ObjectHeader::parse_header(reader, offset);
            let object_header = match object_result {
                Ok(result) => result,
                Err(err) => {
                    error!["[journal] Could not parse object header for entry in array: {err:?}"];
                    continue;
                }
            };

            if object_header.obj_type != ObjectType::Entry {
                warn!("[journal] Did not get Entry object type!");
                continue;
            }

            // Parse the log entry
            let entry_result = Entry::parse_entry(reader, &object_header.payload, is_compact);
            let entry = match entry_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[journal] Could not parse entry data");
                    continue;
                }
            };
            entry_array.entries.push(entry);

            // The Journal file can be configured to be very large. Default size is usually ~10MB
            // To limit memory usage we output every 1k entries
            if entry_array.entries.len() >= limit {
                let messages = EntryArray::parse_messages(&entry_array.entries);
                let serde_data_result = serde_json::to_value(messages);
                let mut serde_data = match serde_data_result {
                    Ok(results) => results,
                    Err(err) => {
                        error!("[journal] Failed to serialize journal data: {err:?}");
                        continue;
                    }
                };

                let _ = output_data(&mut serde_data, "journal", output, start_time, filter);
                // Now empty the vec
                entry_array.entries = Vec::new();
            }
        }

        if entry_array.entries.is_empty() {
            return Ok((input, entry_array.next_entry_array_offset));
        }

        // Output any leftover messages
        let messages = EntryArray::parse_messages(&entry_array.entries);
        let serde_data_result = serde_json::to_value(messages);
        let mut serde_data: serde_json::Value = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[journal] Failed to serialize last journal data: {err:?}");
                return Ok((input, entry_array.next_entry_array_offset));
            }
        };

        let _ = output_data(&mut serde_data, "journal", output, start_time, filter);

        Ok((input, entry_array.next_entry_array_offset))
    }

    /// Walk through Array of entries and return to caller.
    /// Used for JS runtime
    pub(crate) fn walk_all_entries<'a>(
        reader: &mut File,
        data: &'a [u8],
        is_compact: &bool,
    ) -> nom::IResult<&'a [u8], EntryArray> {
        let (mut input, next_entry_array_offset) = nom_unsigned_eight_bytes(data, Endian::Le)?;

        let min_size = 4;
        let mut entry_array = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset,
        };
        let last_entry = 0;
        while !input.is_empty() && input.len() >= min_size {
            let (remaining_input, offset) = if *is_compact {
                let (remaining_input, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
                (remaining_input, offset as u64)
            } else {
                nom_unsigned_eight_bytes(input, Endian::Le)?
            };
            input = remaining_input;

            if offset == last_entry {
                break;
            }
            let object_result = ObjectHeader::parse_header(reader, offset);
            let object_header = match object_result {
                Ok(result) => result,
                Err(err) => {
                    error!["[journal] Could not parse object header for entry in array: {err:?}"];
                    continue;
                }
            };

            if object_header.obj_type != ObjectType::Entry {
                warn!("[journal] Did not get Entry object type!");
                continue;
            }

            let entry_result = Entry::parse_entry(reader, &object_header.payload, is_compact);
            let entry = match entry_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[journal] Could not parse entry data");
                    continue;
                }
            };
            entry_array.entries.push(entry);
        }

        Ok((input, entry_array))
    }

    /// Parse the `Journal` message
    pub(crate) fn parse_messages(entries: &[Entry]) -> Vec<Journal> {
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
                source_realtime: String::new(),
                realtime: unixepoch_microseconds_to_iso(entry.realtime as i64),
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
                        journal.source_realtime = unixepoch_microseconds_to_iso(
                            timestamp.parse::<i64>().unwrap_or_default(),
                        );
                    }
                } else if data.message.starts_with("PRIORITY=") {
                    if let Some((_, priority)) = data.message.split_once('=') {
                        journal.priority =
                            EntryArray::get_priority(&priority.parse::<u32>().unwrap_or_default());
                    }
                } else if data.message.starts_with("SYSLOG_FACILITY=") {
                    if let Some((_, facility)) = data.message.split_once('=') {
                        journal.syslog_facility =
                            EntryArray::get_facility(&facility.parse::<u32>().unwrap_or_default());
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
    use super::EntryArray;
    use crate::{
        artifacts::os::linux::journals::{
            header::{IncompatFlags, JournalHeader},
            objects::header::{ObjectHeader, ObjectType},
        },
        filesystem::files::file_reader,
        structs::toml::Output,
    };
    use common::linux::{Facility, Priority};
    use std::{io::Read, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
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
    fn test_walk_entries() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut buff = [0; 264];
        let _ = reader.read(&mut buff).unwrap();

        let (_, header) = JournalHeader::parse_header(&buff).unwrap();
        let object = ObjectHeader::parse_header(&mut reader, header.entry_array_offset).unwrap();
        assert_eq!(object.obj_type, ObjectType::EntryArray);
        let is_compact = if header.incompatible_flags.contains(&IncompatFlags::Compact) {
            true
        } else {
            false
        };
        let mut output = output_options("journal_test", "local", "./tmp", false);

        let (_, result) = EntryArray::walk_entries(
            &mut reader,
            &object.payload,
            &is_compact,
            &mut output,
            &false,
            &0,
        )
        .unwrap();
        assert_eq!(result, 3744448);
    }

    #[test]
    fn test_walk_all_entries() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut buff = [0; 264];
        let _ = reader.read(&mut buff).unwrap();

        let (_, header) = JournalHeader::parse_header(&buff).unwrap();
        let object = ObjectHeader::parse_header(&mut reader, header.entry_array_offset).unwrap();
        assert_eq!(object.obj_type, ObjectType::EntryArray);
        let is_compact = if header.incompatible_flags.contains(&IncompatFlags::Compact) {
            true
        } else {
            false
        };

        let (_, result) =
            EntryArray::walk_all_entries(&mut reader, &object.payload, &is_compact).unwrap();
        assert_eq!(result.entries.len(), 4);
        assert_eq!(result.entries[2].realtime, 1688346965580106);
    }

    #[test]
    fn test_parse_messages() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/journal/user-1000@e755452aab34485787b6d73f3035fb8c-000000000000068d-0005ff8ae923c73b.journal");

        let mut reader = file_reader(&test_location.display().to_string()).unwrap();
        let mut buff = [0; 264];
        let _ = reader.read(&mut buff).unwrap();

        let (_, header) = JournalHeader::parse_header(&buff).unwrap();
        let object = ObjectHeader::parse_header(&mut reader, header.entry_array_offset).unwrap();
        assert_eq!(object.obj_type, ObjectType::EntryArray);
        let is_compact = if header.incompatible_flags.contains(&IncompatFlags::Compact) {
            true
        } else {
            false
        };

        let (_, result) =
            EntryArray::walk_all_entries(&mut reader, &object.payload, &is_compact).unwrap();
        assert_eq!(result.entries.len(), 4);
        assert_eq!(result.entries[2].realtime, 1688346965580106);

        let messages = EntryArray::parse_messages(&result.entries);
        assert_eq!(
            messages[2].message,
            "Started grub-boot-success.timer - Mark boot as successful after the user session has run 2 minutes."
        );
        assert_eq!(messages[1].boot_id, "05a969ef57fe4934900b598c83f62d76");
        assert_eq!(messages[3].executable, "/usr/lib/systemd/systemd");
    }

    #[test]
    fn test_get_priority() {
        let test = [0, 1, 2, 3, 4, 5, 6, 7];
        for entry in test {
            let result = EntryArray::get_priority(&entry);
            assert!(result != Priority::None);
        }
    }

    #[test]
    fn test_get_facility() {
        let test: Vec<u32> = (0..23).collect();
        for entry in test {
            let result = EntryArray::get_facility(&entry);
            assert!(result != Facility::None);
        }
    }
}
