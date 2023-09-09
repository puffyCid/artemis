use super::{
    error::DbError,
    tables::{add_table_data, lookup_table_data},
};
use crate::{
    artifacts::{
        enrollment::{EndpointDb, EndpointInfo},
        sockets::{Heartbeat, Pulse},
    },
    utils::time::time_now,
};
use log::error;
use redb::Database;

/// Enroll endpoint into the `EndpointDB`
pub(crate) fn enroll_endpointdb(
    endpoint: &EndpointInfo,
    id: &str,
    db: &Database,
) -> Result<(), DbError> {
    let data = EndpointDb {
        hostname: endpoint.hostname.clone(),
        platform: endpoint.platform.clone(),
        tags: Vec::new(),
        notes: Vec::new(),
        checkin: time_now(),
    };

    let serde_result = serde_json::to_vec(&data);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize endpoint data into DB table: {err:?}");
            return Err(DbError::Serialize);
        }
    };

    let result = add_table_data(db, id, &value, "endpoints");
    if result.is_err() {
        return Err(DbError::EndpointDb);
    }

    Ok(())
}

/// Update heartbeat information table
pub(crate) fn update_heartbeat(
    heartbeat: &Heartbeat,
    id: &str,
    db: &Database,
) -> Result<(), DbError> {
    let serde_result = serde_json::to_vec(heartbeat);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize heartbeat data into DB table: {err:?}");
            return Err(DbError::Serialize);
        }
    };

    let result = add_table_data(db, id, &value, "heartbeat");
    if result.is_err() {
        return Err(DbError::EndpointDb);
    }

    Ok(())
}

/// Update pulse information table
pub(crate) fn update_pulse(pulse: &Pulse, id: &str, db: &Database) -> Result<(), DbError> {
    let serde_result = serde_json::to_vec(pulse);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize pulse data into DB table: {err:?}");
            return Err(DbError::Serialize);
        }
    };

    let result = add_table_data(db, id, &value, "pulse");
    if result.is_err() {
        return Err(DbError::EndpointDb);
    }

    Ok(())
}

/// Lookup an endpoint ID in the `EndpointDB`
pub(crate) fn lookup_endpoint(db: &Database, id: &str) -> Result<(bool, EndpointDb), DbError> {
    let value = lookup_table_data("endpoints", id, db)?;
    if value.is_empty() {
        let empty = EndpointDb {
            hostname: String::new(),
            platform: String::new(),
            tags: Vec::new(),
            notes: Vec::new(),
            checkin: 0,
        };

        return Ok((false, empty));
    }

    let serde_value = serde_json::from_slice(&value);
    let db_value: EndpointDb = match serde_value {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize endpoint data: {err:?}");
            return Err(DbError::Deserialize);
        }
    };

    Ok((true, db_value))
}

#[cfg(test)]
mod tests {
    use super::{enroll_endpointdb, update_heartbeat, update_pulse};
    use crate::{
        artifacts::{
            enrollment::EndpointInfo,
            sockets::{Heartbeat, Pulse},
            systeminfo::Memory,
        },
        db::{endpoints::lookup_endpoint, tables::setup_db},
        utils::filesystem::create_dirs,
    };
    use std::path::PathBuf;

    #[test]
    fn test_enroll_endpointdb() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/endpoints.redb";

        let id = "arandomkey";
        let data = EndpointInfo {
            boot_time: 1111,
            hostname: String::from("hello"),
            os_version: String::from("12.1"),
            uptime: 100,
            kernel_version: String::from("12.11"),
            platform: String::from("linux"),
            cpu: Vec::new(),
            disks: Vec::new(),
            memory: Memory {
                available_memory: 111,
                free_memory: 111,
                free_swap: 111,
                total_memory: 111,
                total_swap: 111,
                used_memory: 111,
                used_swap: 111,
            },
        };

        let db = setup_db(path).unwrap();

        enroll_endpointdb(&data, &id, &db).unwrap();
    }

    #[test]
    fn test_update_heartbeat() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/endpointsbeat.redb";
        let id = "arandomkey";
        let data = Heartbeat {
            endpoint_id: id.to_string(),
            heartbeat: true,
            jobs_running: 0,
            timestamp: 10,
            boot_time: 1111,
            hostname: String::from("hello"),
            os_version: String::from("12.1"),
            uptime: 100,
            kernel_version: String::from("12.11"),
            platform: String::from("linux"),
            cpu: Vec::new(),
            disks: Vec::new(),
            memory: Memory {
                available_memory: 111,
                free_memory: 111,
                free_swap: 111,
                total_memory: 111,
                total_swap: 111,
                used_memory: 111,
                used_swap: 111,
            },
        };

        let db = setup_db(path).unwrap();

        update_heartbeat(&data, &id, &db).unwrap();
    }

    #[test]
    fn test_update_pulse() {
        create_dirs("./tmp").unwrap();
        let path = "./tmp/endpointspulse.redb";
        let id = "arandomkey";
        let data = Pulse {
            endpoint_id: id.to_string(),
            pulse: true,
            jobs_running: 0,
            timestamp: 10,
        };

        let db = setup_db(path).unwrap();

        update_pulse(&data, &id, &db).unwrap();
    }

    #[test]
    fn test_lookup_endpoint() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/endpoints.redb");
        let path = test_location.display().to_string();

        let id = "3482136c-3176-4272-9bd7-b79f025307d6";
        let db = setup_db(&path).unwrap();

        let (found, value) = lookup_endpoint(&db, id).unwrap();
        assert!(found);

        assert_eq!(value.hostname, "aStudio.lan");
        assert_eq!(value.platform, "Darwin");
        assert_eq!(value.checkin, 1693968058);
    }
}
