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
pub fn timeline_artifact(data: &mut Value, artifact: &Artifacts) -> Option<()> {
    match artifact {
        Artifacts::Processes => processes(data),
        Artifacts::Files => files(data),
        Artifacts::Journal => journal(data),
        Artifacts::Logons => logons(data),
        Artifacts::SudoLinux => sudo_linux(data),
        Artifacts::Ext4Files => ext4_filelisting(data),
        Artifacts::Amcache => amcache(data),
        Artifacts::Bits => bits(data),
        Artifacts::Eventlogs => eventlogs(data),
        Artifacts::Jumplist => jumplists(data),
        Artifacts::RawFiles => raw_files(data),
        Artifacts::Outlook => outlook(data),
        Artifacts::Prefetch => prefetch(data),
        Artifacts::RecycleBin => recycle_bin(data),
        Artifacts::Registry => registry(data),
        Artifacts::Search => search(data),
        Artifacts::Services => services(data),
        Artifacts::Shellbags => shellbags(data),
        Artifacts::Shimcache => shimcache(data),
        Artifacts::ShimDb => shimdb(data),
        Artifacts::Shortcuts => shortcuts(data),
        Artifacts::Srum => srum(data),
        Artifacts::Tasks => tasks(data),
        Artifacts::Userassist => userassist(data),
        Artifacts::UsersWindows => users(data),
        Artifacts::UsnJrnl => usnjrnl(data),
        Artifacts::Wmi => wmi(data),
        Artifacts::Mft => mft(data),
        Artifacts::UsersMacos => users_macos(data),
        Artifacts::GroupsMacos => groups_macos(data),
        Artifacts::Emond => emond(data),
        Artifacts::LaunchDaemon => launchd(data),
        Artifacts::Fsevents => fsevents(data),
        Artifacts::ExecPolicy => execpolicy(data),
        Artifacts::LoginItems => loginitems(data),
        Artifacts::Spotlight => spotlight(data),
        Artifacts::UnifiedLogs => unifiedlogs(data),
        Artifacts::SudoMacos => sudo_macos(data),
        Artifacts::Connections => network(data),
        Artifacts::Unknown => {
            warn!("Got unknown artifact");
            None
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

        timeline_artifact(&mut result, &Artifacts::Files).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 20);
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

        timeline_artifact(&mut result, &Artifacts::Amcache).unwrap();
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

        timeline_artifact(&mut result, &Artifacts::Bits).unwrap();
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

        timeline_artifact(&mut result, &Artifacts::Jumplist).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 78);
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

        timeline_artifact(&mut result, &Artifacts::Registry).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 23);
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

        timeline_artifact(&mut result, &Artifacts::Search).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 11);
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

        timeline_artifact(&mut result, &Artifacts::Shortcuts).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 12);
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

        timeline_artifact(&mut result, &Artifacts::Prefetch).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 27);
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

        timeline_artifact(&mut result, &Artifacts::ShimDb).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 1);
        assert_eq!(result.to_string().len(), 1277);
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

        timeline_artifact(&mut result, &Artifacts::Spotlight).unwrap();
        assert_eq!(result.as_array().unwrap().len(), 18);
        assert_eq!(result.to_string().len(), 80453);
    }
}
