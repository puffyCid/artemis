use super::error::ExecPolicyError;
use crate::{filesystem::files::is_file, structs::artifacts::os::macos::ExecPolicyOptions};
use common::macos::ExecPolicy;
use log::error;
use rusqlite::{Connection, OpenFlags};

/// Query `ExecPolicy` database
pub(crate) fn grab_execpolicy(
    options: &ExecPolicyOptions,
) -> Result<Vec<ExecPolicy>, ExecPolicyError> {
    let path = if let Some(alt_file) = &options.alt_file {
        alt_file
    } else {
        "/var/db/SystemPolicyConfiguration/ExecPolicy"
    };

    if !is_file(path) {
        return Err(ExecPolicyError::PathError);
    }

    // Bypass SQLITE file lock
    let history_file = format!("file:{path}?immutable=1");
    let connection = Connection::open_with_flags(
        history_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );

    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[execpolicy] Failed to read ExecPolicy SQLITE file {err:?}");
            return Err(ExecPolicyError::SQLITEParseError);
        }
    };

    let  statement = conn.prepare("select is_signed,file_identifier,bundle_identifier,bundle_version,executable_measurements_v2.team_identifier as team_identifier,executable_measurements_v2.signing_identifier as signing_identifier,executable_measurements_v2.cdhash as cdhash,main_executable_hash,executable_timestamp,file_size,is_library,is_used,responsible_file_identifier,is_valid,is_quarantined,executable_measurements_v2.timestamp as executable_measurements_v2_timestamp,reported_timestamp,pk,volume_uuid,object_id,fs_type_name,bundle_id,policy_match,malware_result,flags,mod_time,policy_scan_cache.timestamp as policy_scan_cache_timestamp,revocation_check_time,scan_version,top_policy_match from executable_measurements_v2 left join policy_scan_cache on policy_scan_cache.cdhash = executable_measurements_v2.cdhash;");
    let mut stmt = match statement {
        Ok(query) => query,
        Err(err) => {
            panic!("[execpolicy] Failed to compose ExecPolicy SQL query {err:?}");
            return Err(ExecPolicyError::BadSQL);
        }
    };

    let policy_data = stmt.query_map([], |row| {
        Ok(ExecPolicy {
            is_signed: row.get("is_signed").unwrap_or_default(),
            file_identifier: row.get("file_identifier").unwrap_or_default(),
            bundle_identifier: row.get("bundle_identifier").unwrap_or_default(),
            bundle_version: row.get("bundle_version").unwrap_or_default(),
            team_identifier: row.get("team_identifier").unwrap_or_default(),
            signing_identifier: row.get("signing_identifier").unwrap_or_default(),
            cdhash: row.get("cdhash").unwrap_or_default(),
            main_executable_hash: row.get("main_executable_hash").unwrap_or_default(),
            executable_timestamp: row.get("executable_timestamp").unwrap_or_default(),
            file_size: row.get("file_size").unwrap_or_default(),
            is_library: row.get("is_library").unwrap_or_default(),
            is_used: row.get("is_used").unwrap_or_default(),
            responsible_file_identifier: row.get("responsible_file_identifier").unwrap_or_default(),
            is_valid: row.get("is_valid").unwrap_or_default(),
            is_quarantined: row.get("is_quarantined").unwrap_or_default(),
            executable_measurements_v2_timestamp: row
                .get("executable_measurements_v2_timestamp")
                .unwrap_or_default(),
            reported_timstamp: row.get("reported_timstamp").unwrap_or_default(),
            pk: row.get("pk").unwrap_or_default(),
            volume_uuid: row.get("volume_uuid").unwrap_or_default(),
            object_id: row.get("object_id").unwrap_or_default(),
            fs_type_name: row.get("fs_type_name").unwrap_or_default(),
            bundle_id: row.get("bundle_id").unwrap_or_default(),
            policy_match: row.get("policy_match").unwrap_or_default(),
            malware_result: row.get("malware_result").unwrap_or_default(),
            flags: row.get("flags").unwrap_or_default(),
            mod_time: row.get("mod_time").unwrap_or_default(),
            policy_scan_cache_timestamp: row.get("policy_scan_cache_timestamp").unwrap_or_default(),
            revocation_check_time: row.get("revocation_check_time").unwrap_or_default(),
            scan_version: row.get("scan_version").unwrap_or_default(),
            top_policy_match: row.get("top_policy_match").unwrap_or_default(),
        })
    });

    match policy_data {
        Ok(policy_iter) => {
            let mut policy_vec: Vec<ExecPolicy> = Vec::new();

            for policy in policy_iter {
                match policy {
                    Ok(exec_data) => {
                        policy_vec.push(exec_data);
                    }
                    Err(err) => {
                        error!("[execpolicy] Failed to iterate ExecPolicy data: {err:?}");
                    }
                }
            }

            Ok(policy_vec)
        }
        Err(err) => {
            error!("[execpolicy] Failed to get ExecPolicy data: {err:?}");
            Err(ExecPolicyError::SQLITEParseError)
        }
    }
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::{
        artifacts::os::macos::execpolicy::policy::grab_execpolicy,
        structs::artifacts::os::macos::ExecPolicyOptions,
    };

    #[test]
    fn test_get_execpolicy() {
        let _ = grab_execpolicy(&ExecPolicyOptions { alt_file: None }).unwrap();
    }
}
