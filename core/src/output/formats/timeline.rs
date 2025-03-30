use log::{error, warn};
use serde_json::Value;
use timeline::timeline::{Artifacts, timeline_artifact};

/// Attempt to timeline supported artifacts
pub(crate) fn timeline_data(artifact: &mut Value, artifact_name: &str) {
    let target = get_artifact(artifact_name);
    if target == Artifacts::Unknown {
        error!("[core] Unknown artifact to timeline {artifact_name}");
        return;
    }

    let status = timeline_artifact(artifact, &target);
    if status.is_none() {
        warn!("[core] Could not timeline {artifact_name}");
    }
}

/// Supported list of artifacts
fn get_artifact(artifact: &str) -> Artifacts {
    match artifact {
        "amcache" => Artifacts::Amcache,
        "bits" => Artifacts::Bits,
        "files" => Artifacts::Files,
        "journal" => Artifacts::Journal,
        "registry" => Artifacts::Registry,
        "processes" => Artifacts::Processes,
        "prefetch" => Artifacts::Prefetch,
        "mft" => Artifacts::Mft,
        "srum" => Artifacts::Srum,
        "search" => Artifacts::Search,
        "rawfiles" => Artifacts::RawFiles,
        "recyclebin" => Artifacts::RecycleBin,
        "shimcache" => Artifacts::Shimcache,
        "shimdb" => Artifacts::ShimDb,
        "shellbags" => Artifacts::Shellbags,
        "shortcuts" => Artifacts::Shortcuts,
        "tasks" => Artifacts::Tasks,
        "userassist" => Artifacts::Userassist,
        "usnjrnl" => Artifacts::UsnJrnl,
        "wmi" => Artifacts::Wmi,
        "services" => Artifacts::Services,
        "jumplist" => Artifacts::Jumplist,
        "eventlogs" => Artifacts::Eventlogs,
        "emond" => Artifacts::Emond,
        "launchd" => Artifacts::LaunchDaemon,
        "outlook" => Artifacts::Outlook,
        "loginitems" => Artifacts::LoginItems,
        "fseventsd" => Artifacts::Fsevents,
        "users-macos" => Artifacts::UsersMacos,
        "groups-macos" => Artifacts::GroupsMacos,
        "execpolicy" => Artifacts::ExecPolicy,
        "unifiedlogs" => Artifacts::UnifiedLogs,
        "sudologs-macos" => Artifacts::SudoMacos,
        "spotlight" => Artifacts::Spotlight,
        "logons" => Artifacts::Logons,
        "sudologs-linux" => Artifacts::SudoLinux,
        "users-windows" => Artifacts::UsersWindows,
        "connections" => Artifacts::Connections,
        _ => Artifacts::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::{get_artifact, timeline_data};
    use serde_json::json;
    use timeline::timeline::Artifacts;

    #[test]
    fn test_get_artifact() {
        let test = [
            "amcache",
            "bits",
            "files",
            "journal",
            "registry",
            "processes",
            "prefetch",
            "mft",
            "srum",
            "search",
            "rawfiles",
            "recyclebin",
            "shimcache",
            "shimdb",
            "shellbags",
            "shortcuts",
            "tasks",
            "userassist",
            "usnjrnl",
            "wmi",
            "services",
            "jumplist",
            "eventlogs",
            "emond",
            "launchd",
            "outlook",
            "loginitems",
            "fseventsd",
            "users-macos",
            "groups-macos",
            "execpolicy",
            "unifiedlogs",
            "sudologs-macos",
            "spotlight",
            "logons",
            "sudologs-linux",
            "users-windows",
            "connections",
        ];

        for entry in test {
            let result = get_artifact(entry);
            assert_ne!(result, Artifacts::Unknown);
        }
    }

    #[test]
    fn test_timeline_data() {
        let mut test = json![[{"full_path":"./deps/autocfg-36b1baa0a559f221.d","directory":"./deps","filename":"autocfg-36b1baa0a559f221.d","extension":"d","created":"2024-12-05T03:59:38.000Z","modified":"2024-12-05T03:59:36.000Z","changed":"2024-12-08T03:59:36.000Z","accessed":"2024-12-06T04:42:22.000Z","size":1780,"inode":4295384,"mode":33188,"uid":1000,"gid":1000,"md5":"9b5ec7c5011358706533373fdc05f59e","sha1":"","sha256":"","is_file":true,"is_directory":false,"is_symlink":false,"depth":2,"yara_hits":[],"binary_info":[]}]];
        timeline_data(&mut test, "files");

        assert_eq!(test.as_array().unwrap().len(), 4);
    }
}
