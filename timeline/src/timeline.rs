use crate::artifacts::{
    files::files,
    linux::{ext4_filelisting, journal, logons, sudo_linux},
    macos::{
        emond, execpolicy, fsevents, groups_macos, launchd, loginitems, spotlight, sudo_macos,
        unifiedlogs, users_macos,
    },
    processes::{network, processes},
    windows::{
        amcache, bits, eventlogs, jumplists, mft, outlook, prefetch, raw_files, recycle_bin,
        registry, search, services, shellbags, shimcache, shimdb, shortcuts, srum, tasks,
        userassist, users, usnjrnl, wmi,
    },
};
use log::warn;
use serde_json::Value;

#[derive(PartialEq, Debug)]
pub enum Artifacts {
    Processes,
    Files,
    Connections,
    // Linux
    Journal,
    Logons,
    SudoLinux,
    Ext4Files,
    // Windows
    UsersWindows,
    Amcache,
    Bits,
    Eventlogs,
    Jumplist,
    RawFiles,
    Outlook,
    Prefetch,
    RecycleBin,
    Registry,
    Search,
    Services,
    Shellbags,
    Shimcache,
    ShimDb,
    Shortcuts,
    Srum,
    Tasks,
    Userassist,
    UsnJrnl,
    Wmi,
    Mft,
    // macOS
    UsersMacos,
    GroupsMacos,
    LaunchDaemon,
    Fsevents,
    Emond,
    ExecPolicy,
    LoginItems,
    Spotlight,
    UnifiedLogs,
    SudoMacos,

    Unknown,
}

/// Timeline a parsed artifact
pub fn timeline_artifact(
    data: &mut Value,
    artifact: &Artifacts,
    start: &Option<String>,
    end: &Option<String>,
) -> Option<()> {
    match artifact {
        Artifacts::Processes => todo!(),
        Artifacts::Files => todo!(),
        Artifacts::Journal => journal(data, start, end),
        Artifacts::Logons => logons(data, start, end),
        Artifacts::SudoLinux => sudo_linux(data, start, end),
        Artifacts::Ext4Files => ext4_filelisting(data, start, end),
        Artifacts::Amcache => amcache(data, start, end),
        Artifacts::Bits => bits(data, start, end),
        Artifacts::Eventlogs => eventlogs(data, start, end),
        Artifacts::Jumplist => jumplists(data, start, end),
        Artifacts::RawFiles => raw_files(data, start, end),
        Artifacts::Outlook => outlook(data, start, end),
        Artifacts::Prefetch => prefetch(data, start, end),
        Artifacts::RecycleBin => recycle_bin(data, start, end),
        Artifacts::Registry => registry(data, start, end),
        Artifacts::Search => search(data, start, end),
        Artifacts::Services => services(data, start, end),
        Artifacts::Shellbags => shellbags(data, start, end),
        Artifacts::Shimcache => shimcache(data, start, end),
        Artifacts::ShimDb => shimdb(data, start, end),
        Artifacts::Shortcuts => shortcuts(data, start, end),
        Artifacts::Srum => srum(data, start, end),
        Artifacts::Tasks => tasks(data, start, end),
        Artifacts::Userassist => userassist(data, start, end),
        Artifacts::UsersWindows => users(data, start, end),
        Artifacts::UsnJrnl => usnjrnl(data, start, end),
        Artifacts::Wmi => wmi(data, start, end),
        Artifacts::Mft => mft(data, start, end),
        Artifacts::UsersMacos => users_macos(data, start, end),
        Artifacts::GroupsMacos => groups_macos(data, start, end),
        Artifacts::Emond => emond(data, start, end),
        Artifacts::LaunchDaemon => launchd(data, start, end),
        Artifacts::Fsevents => fsevents(data, start, end),
        Artifacts::ExecPolicy => execpolicy(data, start, end),
        Artifacts::LoginItems => loginitems(data, start, end),
        Artifacts::Spotlight => spotlight(data, start, end),
        Artifacts::UnifiedLogs => unifiedlogs(data, start, end),
        Artifacts::SudoMacos => sudo_macos(data, start, end),
        Artifacts::Connections => todo!(),
        Artifacts::Unknown => {
            warn!("Got unknown artifact");
            None
        }
    }
}

pub fn timeline_artifact_ng(
    data: &mut Value,
    artifact: &str,
    start: &Option<String>,
    end: &Option<String>,
) -> bool {
    match artifact.to_ascii_lowercase().as_str() {
        "amcache" => todo!(),
        "bits" => todo!(),
        "files" => todo!(),
        "journal" => todo!(),
        "registry" => todo!(),
        "processes" => processes(data, start, end),
        "prefetch" => todo!(),
        "mft" => todo!(),
        "srum" => todo!(),
        "search" => todo!(),
        "rawfiles" => todo!(),
        "recyclebin" => todo!(),
        "shimcache" => todo!(),
        "shimdb" => todo!(),
        "shellbags" => todo!(),
        "shortcuts" => todo!(),
        "tasks" => todo!(),
        "userassist" => todo!(),
        "usnjrnl" => todo!(),
        "wmipersist" => todo!(),
        "services" => todo!(),
        "jumplists" => todo!(),
        "eventlogs" => todo!(),
        "emond" => todo!(),
        "launchd" => todo!(),
        "outlook" => todo!(),
        "loginitems" => todo!(),
        "fseventsd" => todo!(),
        "users-macos" => todo!(),
        "groups-macos" => todo!(),
        "execpolicy" => todo!(),
        "unifiedlogs" => todo!(),
        "sudologs-macos" => todo!(),
        "spotlight" => todo!(),
        "logons" => todo!(),
        "sudologs-linux" => todo!(),
        "users-windows" => todo!(),
        "connections" => network(data),
        "ext4files" => todo!(),
        _ => {
            warn!("Got unknown artifact: {artifact}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Artifacts, timeline_artifact};
    use serde_json::Value;
    use std::{fs::read_to_string, path::PathBuf};

    #[test]
    fn test_timeline_artifact_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/files.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Files, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1296);
    }

    #[test]
    fn test_timeline_artifact_amcache() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/amcache.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Amcache, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 4);
    }

    #[test]
    fn test_timeline_artifact_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/bits.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Bits, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 9);
    }

    #[test]
    fn test_timeline_artifact_jumplist() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/jumplist.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Jumplist, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 109);
    }

    #[test]
    fn test_timeline_artifact_tasks() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/tasks.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Tasks, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 109);
    }

    #[test]
    fn test_timeline_artifact_registry() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/registry.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Registry, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 133);
    }

    #[test]
    fn test_timeline_artifact_search() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/search.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Search, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 208);
    }

    #[test]
    fn test_timeline_artifact_shortcuts() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/shortcuts.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Shortcuts, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 13);
    }

    #[test]
    fn test_timeline_artifact_prefetch() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/prefetch.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Prefetch, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 325);
    }

    #[test]
    fn test_timeline_artifact_shimdb() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/shimdb.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::ShimDb, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1);
        assert_eq!(result.to_string().len(), 1851);
    }

    #[test]
    fn test_timeline_artifact_spotlight() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/spotlight.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::Spotlight, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 66);
        assert_eq!(result.to_string().len(), 251620);
    }

    #[test]
    fn test_timeline_artifact_unifiedlogs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/unifiedlogs.jsonl");
        let mut data = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            data.push(serde_json::from_str(line).unwrap())
        }
        let mut result = Value::Array(data);

        timeline_artifact(&mut result, &Artifacts::UnifiedLogs, &None, &None).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 168);
        assert_eq!(result.to_string().len(), 382407);
    }
}
