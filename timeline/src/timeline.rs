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
    Some(())
}

pub fn timeline_artifact_ng(
    data: &mut Value,
    artifact: &str,
    start: &Option<String>,
    end: &Option<String>,
) -> bool {
    match artifact.to_ascii_lowercase().as_str() {
        "amcache" => amcache(data, start, end),
        "bits" => bits(data, start, end),
        "files" => files(data, start, end),
        "journal" => journal(data, start, end),
        "registry" => registry(data, start, end),
        "processes" => processes(data, start, end),
        "prefetch" => prefetch(data, start, end),
        "mft" => mft(data, start, end),
        "srum" => srum(data, start, end),
        "search" => search(data, start, end),
        "rawfiles" => raw_files(data, start, end),
        "recyclebin" => recycle_bin(data, start, end),
        "shimcache" => shimcache(data, start, end),
        "shimdb" => shimdb(data, start, end),
        "shellbags" => shellbags(data, start, end),
        "shortcuts" => shortcuts(data, start, end),
        "tasks" => tasks(data, start, end),
        "userassist" => userassist(data, start, end),
        "usnjrnl" => usnjrnl(data, start, end),
        "wmipersist" => wmi(data),
        "services" => services(data, start, end),
        "jumplists" => jumplists(data, start, end),
        "eventlogs" => eventlogs(data, start, end),
        "emond" => emond(data, start, end),
        "launchd" => launchd(data, start, end),
        "outlook" => outlook(data, start, end),
        "loginitems" => loginitems(data, start, end),
        "fseventsd" => fsevents(data, start, end),
        "users-macos" => users_macos(data, start, end),
        "groups-macos" => groups_macos(data),
        "execpolicy" => execpolicy(data, start, end),
        "unifiedlogs" => unifiedlogs(data, start, end),
        "sudologs-macos" => sudo_macos(data, start, end),
        "spotlight" => spotlight(data, start, end),
        "logons" => logons(data, start, end),
        "sudologs-linux" => sudo_linux(data, start, end),
        "users-windows" => users(data, start, end),
        "connections" => network(data),
        "ext4files" => ext4_filelisting(data, start, end),
        _ => {
            warn!("Got unknown artifact: {artifact}");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::timeline_artifact_ng;
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "files", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }
        assert_eq!(data.len(), 1296);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "amcache", &None, &None));

            data.push(value);
        }

        assert_eq!(data.len(), 4);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "bits", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }

        assert_eq!(data.len(), 9);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "jumplists", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }

        assert_eq!(data.len(), 109);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "tasks", &None, &None));
            data.push(value);
        }

        assert_eq!(data.len(), 109);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "registry", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }

        assert_eq!(data.len(), 133);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "search", &None, &None));
            data.push(value);
        }

        assert_eq!(data.len(), 208);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "shortcuts", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }

        assert_eq!(data.len(), 13);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "prefetch", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }

        assert_eq!(data.len(), 325);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "shimdb", &None, &None));
            for entry in value.as_array().unwrap() {
                data.push(entry.clone())
            }
        }

        assert_eq!(data.len(), 1);
        assert_eq!(data[0].to_string().len(), 1849);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(&mut value, "spotlight", &None, &None));
            data.push(value);
        }
        assert_eq!(data.len(), 66);
        assert_eq!(format!("{data:?}").len(), 339010);
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
            let mut value = serde_json::from_str(line).unwrap();
            assert!(timeline_artifact_ng(
                &mut value,
                "unifiedlogs",
                &None,
                &None
            ));
            data.push(value);
        }

        assert_eq!(data.len(), 168);
        assert_eq!(format!("{data:?}").len(), 519542);
    }
}
