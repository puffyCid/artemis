use super::{
    error::JournalError,
    header::{IncompatFlags, JournalHeader},
    objects::{
        array::EntryArray,
        entry::Entry,
        header::{ObjectHeader, ObjectType},
    },
};
use crate::{filesystem::files::file_reader, utils::artemis_toml::Output};
use log::error;
use std::{collections::HashMap, fs::File, io::Read};

/**
 * TODO
 * 1. Finalize final format
 * 2. tests
 * 3. support compression (thoguh appears to be disabled by default)
 * 3.5 Comments
 * 4. deno bindings
 *
 * data and field hash tables r used to resolve collisions?
 *
 * References:
 *  - https://systemd.io/JOURNAL_FILE_FORMAT/
 *  - https://wiki.archlinux.org/title/Systemd/Journal
 *  - https://github.com/systemd/systemd/blob/main/src/libsystemd/sd-journal/journal-def.h
 *  - https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html
 */
// final output should have something like. might need to use hashmap?
/**
* Fri 2023-06-30 21:34:10.320959 EDT [s=9c59e07a1e4d4ae8b55752d99b33fde2;i=40b07;b=487df2a82dcf42889f6bfffda0d93181;m=565c044;t=5ff62ee1d2e45;x=5a9514c0e1ebc1c2]
   PRIORITY=6
   SYSLOG_FACILITY=3
   TID=1786
   SYSLOG_IDENTIFIER=systemd
   _TRANSPORT=journal
   _PID=1786
   _UID=1000
   _GID=1000
   _COMM=systemd
   _EXE=/usr/lib/systemd/systemd
   _CMDLINE=/usr/lib/systemd/systemd --user
   _CAP_EFFECTIVE=0
   _AUDIT_SESSION=4
   _AUDIT_LOGINUID=1000
   _SYSTEMD_CGROUP=/user.slice/user-1000.slice/user@1000.service/init.scope
   _SYSTEMD_OWNER_UID=1000
   _SYSTEMD_UNIT=user@1000.service
   _SYSTEMD_USER_UNIT=init.scope
   _SYSTEMD_SLICE=user-1000.slice
   _SYSTEMD_USER_SLICE=-.slice
   _SYSTEMD_INVOCATION_ID=3e0ac308547e46858c2769349536cd01
   _BOOT_ID=487df2a82dcf42889f6bfffda0d93181
   _MACHINE_ID=2baf13cfb28b4c62a39a8d92e080c348
   _HOSTNAME=archlinux
   _RUNTIME_SCOPE=system
   CODE_FILE=src/core/unit.c
   USER_INVOCATION_ID=548fcd49228646078e347bd82f28dfd2
   USER_UNIT=org.gnome.Shell@wayland.service
   CODE_LINE=5681
   CODE_FUNC=unit_log_skip
   MESSAGE_ID=0e4284a0caca4bfc81c0bb6786972673
   MESSAGE=org.gnome.Shell@wayland.service: Skipped due to 'exec-condition'.
   UNIT_RESULT=exec-condition
   _SOURCE_REALTIME_TIMESTAMP=1688175250320959
*/
// _ underscore entries are trusted fields
// Everythign else is optional or can technically be repeated
// https://www.freedesktop.org/software/systemd/man/systemd.journal-fields.html
// Message_ID fields point to extra info at the catalog file at  /usr/lib/systemd/catalog/systemd.catalog
// Technically catalogs exists for multiple languages. Will focus on english for now
// maybe make an option in TOML? catalog_langague = "en|fr|ru|etc" // Default is english

pub(crate) struct Journal {
    uid: u32,
    gid: u32,
    pid: u32,
    comm: String,
    priority: Priority,
    syslog_facility: Facility,
    thread_id: u32,
    syslog_identifier: String,
    executable: String,
    cmdline: String,
    cap_effective: usize,
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
}

// https://wiki.archlinux.org/title/Systemd/Journal
pub(crate) enum Priority {
    Emergency,
    Alert,
    Critical,
    Error,
    Warning,
    Notice,
    Informational,
    Debug,
}

// https://wiki.archlinux.org/title/Systemd/Journal
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
}

impl Journal {
    pub(crate) fn parse_journal(path: &str, output: &mut Output) -> Result<(), JournalError> {
        let mut reader = file_reader(path).unwrap();

        // We technically only need first 232 bytes but version 252 is 264 bytes in size
        let mut header_buff = [0; 264];
        let _ = reader.read(&mut header_buff).unwrap();

        let (_, journal_header) = JournalHeader::parse_header(&header_buff).unwrap();
        let is_compact = if journal_header
            .incompatible_flags
            .contains(&IncompatFlags::Compact)
        {
            true
        } else {
            false
        };

        let entry_array = Journal::get_entries(
            &mut reader,
            journal_header.entry_array_offset,
            is_compact,
            output,
        )?;

        Ok(())
    }

    pub(crate) fn parse_journal_file(path: &str) {}

    fn get_entries(
        reader: &mut File,
        array_offset: u64,
        is_compact: bool,
        output: &mut Output,
    ) -> Result<(), JournalError> {
        let mut offset = array_offset;
        let last_entry = 0;

        let mut entries = EntryArray {
            entries: Vec::new(),
            next_entry_array_offset: 0,
        };

        let limit = 100000;
        while offset != last_entry {
            let object_header = ObjectHeader::parse_header(reader, offset)?;
            if object_header.obj_type != ObjectType::EntryArray {
                panic!("[journal] Did not get Entry Array type at entry_array_offset. Got: {:?}. Returning early", object_header.obj_type);
                return Ok(());
            }

            let (_, mut entry_array) =
                EntryArray::walk_entries(reader, &object_header.payload, is_compact).unwrap();
            entries.entries.append(&mut entry_array.entries);
            println!("{}", entry_array.entries.len());
            offset = entry_array.next_entry_array_offset;

            // The Journal file can be configured to be very large. Default size is usually ~10MB
            // To limit memory usage we output every 100k entries

            if entries.entries.len() >= limit {
                if Journal::output_messages(&entries.entries, output).is_err() {
                    error!("[journal] Could not output Journal messages!");
                }
            }
        }

        Ok(())
    }

    fn output_messages(entries: &[Entry], output: &mut Output) -> Result<(), JournalError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::Journal;
    use crate::utils::artemis_toml::Output;
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
        let mut output = output_options("journal_test", "json", "./tmp", false);

        let result =
            Journal::parse_journal(&test_location.display().to_string(), &mut output).unwrap();
    }
}
