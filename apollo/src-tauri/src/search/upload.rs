use super::index::{create_index, upload_data, upload_metadata};
use opensearch::{BulkOperation, BulkOperations, Error};
use serde_json::Value;
use timeline::timeline::{timeline_artifact, Artifacts};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};

/// Timeline and upload artemis data to OpenSearch Index name
pub(crate) async fn upload_timeline(path: &str, name: &str) -> Result<Value, Error> {
    let _create_status = create_index(name).await?;

    let file = File::open(path).await?;
    let mut reader = BufReader::new(file).lines();

    // Number of lines in JSONL to upload
    let limit = 500;
    let mut meta = Value::Null;

    let mut entries = Vec::new();
    while let Ok(Some(line)) = reader.next_line().await {
        let mut value: Value = serde_json::from_str(&line).unwrap_or_default();

        // Only need one metadata value. It contains same data for the entire collection
        if meta.is_null() {
            meta = serde_json::from_value(
                value
                    .get("collection_metadata")
                    .unwrap_or(&Value::Bool(false))
                    .clone(),
            )?;
            if meta.is_object() {
                meta["timeline_source"] = Value::String(path.to_string());
                let mut ops_meta = BulkOperations::new();
                ops_meta.push(BulkOperation::index(&meta))?;
                let _upload_status = upload_metadata(&ops_meta).await?;
            }
        }

        value["timeline_source"] = Value::String(path.to_string());

        entries.push(value.clone());

        // Upload 500 entries at a time
        if entries.len() == limit {
            let mut ops = BulkOperations::new();
            if meta.is_null() {
                ops.push(BulkOperation::index(&entries))?;
                let _upload_status = upload_data(&ops, name).await?;
                entries = Vec::new();
                continue;
            }

            let mut timeline_data = Value::Array(entries);

            let artifact = meta["artifact_name"].as_str().unwrap_or_default();
            timeline_artifact(&mut timeline_data, &artifact_name(artifact));
            let default = Vec::new();
            let values = timeline_data.as_array().unwrap_or(&default);
            for entry in values {
                let _ = ops.push(BulkOperation::index(entry));
            }
            //bulk_append(&mut ops, timeline_data.as_array().unwrap_or(&Vec::new()));

            let _upload_status = upload_data(&ops, name).await?;

            entries = Vec::new();
        }
    }

    if !entries.is_empty() {
        let mut ops = BulkOperations::new();
        if meta.is_null() {
            bulk_append(&mut ops, &entries);
            let _upload_status = upload_data(&ops, name).await?;
            return Ok(Value::Null);
        }

        let mut timeline_data = Value::Array(entries);

        let artifact = meta["artifact_name"].as_str().unwrap_or_default();
        timeline_artifact(&mut timeline_data, &artifact_name(artifact));
        bulk_append(&mut ops, timeline_data.as_array().unwrap());

        let _upload_status = upload_data(&ops, name).await?;
    }

    Ok(Value::Null)
}

fn bulk_append(op: &mut BulkOperations, values: &[Value]) {
    for entry in values {
        let _ = op.push(BulkOperation::index(entry));
    }
}

fn artifact_name(artifact: &str) -> Artifacts {
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
        "logon" => Artifacts::Logons,
        "sudologs-linux" => Artifacts::SudoLinux,
        "users-windows" => Artifacts::UsersWindows,
        _ => Artifacts::Unknown,
    }
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::search::upload::upload_timeline;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_upload_timeline_amcache() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../../timeline/tests/test_data/amcache.jsonl");

        let _result = upload_timeline(test_location.to_str().unwrap(), "test")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_timeline_bits() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../../timeline/tests/test_data/bits.jsonl");

        let _result = upload_timeline(test_location.to_str().unwrap(), "test")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_timeline_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../../timeline/tests/test_data/files.jsonl");

        let _result = upload_timeline(test_location.to_str().unwrap(), "test")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_upload_timeline_jumplists() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("../../timeline/tests/test_data/jumplist.jsonl");

        let _result = upload_timeline(test_location.to_str().unwrap(), "test")
            .await
            .unwrap();
    }
}
