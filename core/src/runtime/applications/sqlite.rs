use crate::utils::{encoding::base64_encode_standard, strings::extract_ascii_utf16_string};
use deno_core::{error::AnyError, op2};
use log::error;
use rusqlite::{
    types::{FromSql, FromSqlError, ValueRef},
    Connection, OpenFlags,
};
use serde_json::json;

#[op2]
#[string]
/// Query a sqlite file
pub(crate) fn query_sqlite(
    #[string] path: String,
    #[string] query: String,
) -> Result<String, AnyError> {
    // Bypass SQLITE file lock
    let sqlite_file = format!("file:{path}?immutable=1");
    let connection = Connection::open_with_flags(
        sqlite_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[runtime] Failed to open sqlite file {path}: {err:?}");
            return Err(err.into());
        }
    };

    let statement = conn.prepare(&query);
    let mut stmt = match statement {
        Ok(query) => query,
        Err(err) => {
            error!("[runtime] Failed to compose query {err:?}");
            return Err(err.into());
        }
    };
    let columns = stmt.column_count();

    // Execute user query
    let query_result = stmt.query(());
    let mut query_data = match query_result {
        Ok(result) => result,
        Err(err) => {
            error!("[runtime] Failed to query sqlite {path} {err:?}");
            return Err(err.into());
        }
    };

    let mut data = Vec::new();
    // Loop through all results
    while let Some(row) = query_data.next()? {
        let mut json_data = serde_json::map::Map::new();
        for column in 0..columns {
            let column_name = row
                .as_ref()
                .column_name(column)
                .unwrap_or_default()
                .to_string();

            let column_value_result = row.get_ref(column);
            let column_data = match column_value_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[runtime] Could not get value for column {column_name} for {path}: {err:?}");
                    continue;
                }
            };

            // Need to extract strings and blobs. Everything else can be serialized directly
            match column_data {
                ValueRef::Text(value) => {
                    let result = extract_ascii_utf16_string(value);
                    let column_value = json!(result);
                    // add to json. Column name is key, column value is value
                    json_data.insert(column_name, column_value);
                }
                ValueRef::Blob(value) => {
                    let encoded_data = base64_encode_standard(value);
                    let column_value = json!(encoded_data);
                    // add to json. Column name is key, column value is value
                    json_data.insert(column_name, column_value);
                }
                _ => {
                    let value_result: Result<serde_json::Value, FromSqlError> =
                        FromSql::column_result(column_data);
                    let column_value = match value_result {
                        Ok(result) => result,
                        Err(err) => {
                            error!("[runtime] Could not serialize data from column {column_name} from {path}: {err:?}");
                            continue;
                        }
                    };
                    json_data.insert(column_name, column_value);
                }
            }
        }
        data.push(json_data);
    }

    let results = serde_json::to_string(&data)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
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
    fn test_query_sqlite() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9hcHBsaWNhdGlvbnMvZXJyb3JzLnRzCnZhciBBcHBsaWNhdGlvbkVycm9yID0gY2xhc3MgZXh0ZW5kcyBFcnJvckJhc2Ugewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvYXBwbGljYXRpb25zL3NxbGl0ZS50cwpmdW5jdGlvbiBxdWVyeVNxbGl0ZShwYXRoLCBxdWVyeSkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0gRGVuby5jb3JlLm9wcy5xdWVyeV9zcWxpdGUocGF0aCwgcXVlcnkpOwogICAgY29uc3QgcmVzdWx0cyA9IEpTT04ucGFyc2UoZGF0YSk7CiAgICByZXR1cm4gcmVzdWx0czsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgQXBwbGljYXRpb25FcnJvcigKICAgICAgIlNRTElURSIsCiAgICAgIGBmYWlsZWQgdG8gZXhlY3V0ZSBxdWVyeSAke2Vycn1gCiAgICApOwogIH0KfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHJlc3VsdHMgPSBxdWVyeVNxbGl0ZSgiL0xpYnJhcnkvQXBwbGljYXRpb24gU3VwcG9ydC9jb20uYXBwbGUuVENDL1RDQy5kYiIsICJzZWxlY3QgKiBmcm9tIGFjY2VzcyIpOwp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("sqlite_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
