use std::{
    fs::{OpenOptions, create_dir_all},
    io::Write,
};

use crate::{
    filesystem::files::{hash_file_data, is_file, read_file},
    structs::toml::{Artifacts, Marker},
    utils::time::{time_now, unixepoch_to_iso},
};
use common::files::Hashes;
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct ArtifactRuns {
    hash: String,
    name: String,
    last_run: String,
    unixepoch: u64,
}

/// Check for a marker file and determine if the provided artifact should be skipped
pub(crate) fn skip_artifact(marker: &Marker, artifact: &Artifacts) -> bool {
    let full_path = format!("{}/{}", marker.path, marker.name);

    if !is_file(&full_path) {
        return false;
    }

    let marker_bytes = match read_file(&full_path) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to read marker file: {err:?}");
            return false;
        }
    };

    let runs: Vec<ArtifactRuns> = match serde_json::from_slice(&marker_bytes) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to deserialize marker file: {err:?}");
            return false;
        }
    };

    let artifact_bytes = match serde_json::to_vec(artifact) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[forensics] Failed to serialize artifact {}: {err:?}",
                artifact.artifact_name
            );
            return false;
        }
    };

    let hashes = Hashes {
        md5: true,
        sha1: false,
        sha256: false,
    };
    let (md5, _, _) = hash_file_data(&hashes, &artifact_bytes);

    let seconds = 60;
    let next_run = marker.age * seconds;
    let now = time_now();

    // Check our marker to see if we should collect the artifact again
    for run in runs {
        if run.hash != md5 {
            continue;
        }

        // The current time is greater than previous run plus the age
        // Example: Collected processes on 2026-01-01. Age is 3 days (in minutes). Next process collection would be 2026-01-04.
        //   Any attempt before that date will be skipped if marker file is checked
        if (run.unixepoch + next_run) < now {
            return true;
        }
        break;
    }

    false
}

pub(crate) fn update_marker(marker: &Marker, artifact: &Artifacts) {
    let mut runs: Vec<ArtifactRuns> = Vec::new();
    let full_path = format!("{}/{}", marker.path, marker.name);

    // Return array of previous artifact runs
    if is_file(&full_path)
        && let Ok(bytes) = read_file(&full_path)
        && let Ok(values) = serde_json::from_slice(&bytes)
    {
        runs = values;
    }

    let artifact_bytes = match serde_json::to_vec(artifact) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[forensics] Failed to serialize artifact {}: {err:?}",
                artifact.artifact_name
            );
            return;
        }
    };

    let hashes = Hashes {
        md5: true,
        sha1: false,
        sha256: false,
    };
    let (md5, _, _) = hash_file_data(&hashes, &artifact_bytes);

    let mut existing_run = false;
    // Check if we are running an existing artifact with same options
    for run in &mut runs {
        if run.hash != md5 {
            continue;
        }

        // Update the last run
        run.unixepoch = time_now();
        run.last_run = unixepoch_to_iso(time_now() as i64);
        existing_run = true;
    }

    // If its a new artifact, append to our marker
    if !existing_run {
        let new_run = ArtifactRuns {
            unixepoch: time_now(),
            last_run: unixepoch_to_iso(time_now() as i64),
            name: artifact.artifact_name.clone(),
            hash: md5,
        };
        runs.push(new_run);
    }

    if let Err(err) = create_dir_all(&marker.path) {
        error!(
            "[forensics] Failed to create marker directory {}: {err:?}",
            marker.path
        );
    }

    let mut fs = match OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(&full_path)
    {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[forensics] Could not open provided marker file {}: {err:?}",
                marker.path
            );
            return;
        }
    };

    let runs_bytes = match serde_json::to_vec(&runs) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[forensics] Failed to serialize runs {}: {err:?}",
                artifact.artifact_name
            );
            return;
        }
    };

    if let Err(err) = fs.write_all(&runs_bytes) {
        error!(
            "[forensics] Failed to write marker file {}: {err:?}",
            marker.path
        );
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        filesystem::files::read_file,
        structs::{
            artifacts::os::{processes::ProcessOptions, windows::AmcacheOptions},
            toml::{Artifacts, Marker},
        },
        utils::marker::{ArtifactRuns, skip_artifact, update_marker},
    };
    use std::path::PathBuf;

    #[test]
    fn test_skip_artifact() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/marker");

        let marker = Marker {
            path: test_location.display().to_string(),
            name: String::from("test.json"),
            age: 300,
        };
        let art = Artifacts {
            artifact_name: String::from("amcache"),
            amcache: Some(AmcacheOptions {
                alt_file: Some(String::from("test")),
            }),
            filter: None,
            processes: None,
            files: None,
            unifiedlogs: None,
            users_macos: None,
            groups_macos: None,
            emond: None,
            execpolicy: None,
            launchd: None,
            loginitems: None,
            fseventsd: None,
            sudologs_macos: None,
            spotlight: None,
            journals: None,
            sudologs_linux: None,
            logons: None,
            rawfiles_ext4: None,
            eventlogs: None,
            prefetch: None,
            rawfiles: None,
            shimdb: None,
            registry: None,
            userassist: None,
            shimcache: None,
            shellbags: None,
            shortcuts: None,
            usnjrnl: None,
            bits: None,
            srum: None,
            users_windows: None,
            search: None,
            tasks: None,
            services: None,
            jumplists: None,
            recyclebin: None,
            wmipersist: None,
            outlook: None,
            mft: None,
            connections: None,
            script: None,
        };

        assert!(skip_artifact(&marker, &art));
    }

    #[test]
    fn test_skip_artifact_no() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/marker");

        let marker = Marker {
            path: test_location.display().to_string(),
            name: String::from("test.json"),
            age: 300,
        };
        let art = Artifacts {
            artifact_name: String::from("processes"),
            amcache: None,
            filter: None,
            processes: Some(ProcessOptions {
                md5: true,
                sha1: true,
                sha256: false,
                metadata: true,
            }),
            files: None,
            unifiedlogs: None,
            users_macos: None,
            groups_macos: None,
            emond: None,
            execpolicy: None,
            launchd: None,
            loginitems: None,
            fseventsd: None,
            sudologs_macos: None,
            spotlight: None,
            journals: None,
            sudologs_linux: None,
            logons: None,
            rawfiles_ext4: None,
            eventlogs: None,
            prefetch: None,
            rawfiles: None,
            shimdb: None,
            registry: None,
            userassist: None,
            shimcache: None,
            shellbags: None,
            shortcuts: None,
            usnjrnl: None,
            bits: None,
            srum: None,
            users_windows: None,
            search: None,
            tasks: None,
            services: None,
            jumplists: None,
            recyclebin: None,
            wmipersist: None,
            outlook: None,
            mft: None,
            connections: None,
            script: None,
        };

        assert!(!skip_artifact(&marker, &art));
    }

    #[test]
    fn test_update_marker() {
        let mark = Marker {
            path: String::from("./tmp"),
            age: 600,
            name: String::from("marker.json"),
        };

        let art = Artifacts {
            artifact_name: String::from("processes"),
            amcache: None,
            filter: None,
            processes: Some(ProcessOptions {
                md5: true,
                sha1: true,
                sha256: false,
                metadata: true,
            }),
            files: None,
            unifiedlogs: None,
            users_macos: None,
            groups_macos: None,
            emond: None,
            execpolicy: None,
            launchd: None,
            loginitems: None,
            fseventsd: None,
            sudologs_macos: None,
            spotlight: None,
            journals: None,
            sudologs_linux: None,
            logons: None,
            rawfiles_ext4: None,
            eventlogs: None,
            prefetch: None,
            rawfiles: None,
            shimdb: None,
            registry: None,
            userassist: None,
            shimcache: None,
            shellbags: None,
            shortcuts: None,
            usnjrnl: None,
            bits: None,
            srum: None,
            users_windows: None,
            search: None,
            tasks: None,
            services: None,
            jumplists: None,
            recyclebin: None,
            wmipersist: None,
            outlook: None,
            mft: None,
            connections: None,
            script: None,
        };

        update_marker(&mark, &art);

        let art = Artifacts {
            artifact_name: String::from("processes"),
            amcache: None,
            filter: None,
            processes: Some(ProcessOptions {
                md5: false,
                sha1: false,
                sha256: false,
                metadata: true,
            }),
            files: None,
            unifiedlogs: None,
            users_macos: None,
            groups_macos: None,
            emond: None,
            execpolicy: None,
            launchd: None,
            loginitems: None,
            fseventsd: None,
            sudologs_macos: None,
            spotlight: None,
            journals: None,
            sudologs_linux: None,
            logons: None,
            rawfiles_ext4: None,
            eventlogs: None,
            prefetch: None,
            rawfiles: None,
            shimdb: None,
            registry: None,
            userassist: None,
            shimcache: None,
            shellbags: None,
            shortcuts: None,
            usnjrnl: None,
            bits: None,
            srum: None,
            users_windows: None,
            search: None,
            tasks: None,
            services: None,
            jumplists: None,
            recyclebin: None,
            wmipersist: None,
            outlook: None,
            mft: None,
            connections: None,
            script: None,
        };

        update_marker(&mark, &art);

        let bytes = read_file("./tmp/marker.json").unwrap();
        let runs: Vec<ArtifactRuns> = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(runs.len(), 2);
    }
}
