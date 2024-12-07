use crate::artifacts::{
    files::files,
    linux::{journal, logons, sudo_linux},
    processes::processes,
    windows::{amcache, bits, eventlogs, jumplists, users},
};
use serde_json::Value;

pub enum Artifacts {
    Processes,
    Files,
    Journal,
    Logons,
    SudoLinux,
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
        Artifacts::UsersWindows => users(data),
        Artifacts::Amcache => amcache(data),
        Artifacts::Bits => bits(data),
        Artifacts::Eventlogs => eventlogs(data),
        Artifacts::Jumplist => jumplists(data),
        Artifacts::RawFiles => todo!(),
        Artifacts::Outlook => todo!(),
        Artifacts::Prefetch => todo!(),
        Artifacts::RecycleBin => todo!(),
        Artifacts::Registry => todo!(),
        Artifacts::Search => todo!(),
        Artifacts::Services => todo!(),
        Artifacts::Shellbags => todo!(),
        Artifacts::Shimcache => todo!(),
        Artifacts::ShimDb => todo!(),
        Artifacts::Shortcuts => todo!(),
        Artifacts::Srum => todo!(),
        Artifacts::Tasks => todo!(),
        Artifacts::Userassist => todo!(),
        Artifacts::UsnJrnl => todo!(),
        Artifacts::Wmi => todo!(),

        Artifacts::Unknown => todo!(),
    }
}

#[cfg(test)]
mod tests {
    use super::{timeline_artifact, Artifacts};
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
        assert_eq!(result.as_array().unwrap().len(), 82);
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
}
