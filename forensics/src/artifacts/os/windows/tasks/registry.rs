use crate::{
    artifacts::os::windows::{registry::helper::get_registry_keys, tasks::error::TaskError},
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{
            Endian, nom_signed_four_bytes, nom_unsigned_eight_bytes, nom_unsigned_four_bytes,
        },
        regex_options::create_regex,
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use common::windows::TaskCache;
use log::error;
use std::collections::HashMap;

#[derive(Debug)]
struct TaskTree {
    id: String,
    path: String,
    registry_path: String,
}

pub(crate) fn cache_info(drive: char) -> Result<HashMap<String, TaskCache>, TaskError> {
    let patter =
        create_regex(r"microsoft\\windows nt\\currentversion\\schedule\\taskcache\\(tree|tasks)")
            .unwrap();
    let registry_file = format!("{drive}:\\Windows\\System32\\config\\SOFTWARE");
    let reg_paths = match get_registry_keys("ROOT", &patter, &registry_file) {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Could not read Registry {err:?}");
            return Err(TaskError::Registry);
        }
    };

    /*
     * TODO:
     * 10. create hashmap that uses the tree uri path as key and taskcache as value
     */

    let tree_key = "ROOT\\Microsoft\\Windows NT\\CurrentVersion\\Schedule\\TaskCache\\Tree";
    let cache = "ROOT\\Microsoft\\Windows NT\\CurrentVersion\\Schedule\\TaskCache\\Tasks";
    let mut tree_info = Vec::new();
    let mut cache_map = HashMap::new();
    // Loop through the Registry data and get the Tasks cache and Tree values
    for entry in reg_paths {
        if entry.path.starts_with(tree_key) && entry.values.len() != 1 {
            // Find the ID value in the Registry Value data
            for value in entry.values {
                if value.value != "Id" {
                    continue;
                }
                let info = TaskTree {
                    id: value.data,
                    path: entry.path.replace(tree_key, ""),
                    registry_path: entry.path,
                };
                tree_info.push(info);
                break;
            }
        } else if entry.path.starts_with(cache) && entry.values.len() > 1 {
            let mut cache = TaskCache {
                registry_task_path: entry.path,
                registry_file: registry_file.clone(),
                id: entry.name.clone(),
                ..Default::default()
            };
            for value in entry.values {
                // Contains timestamps associated Task execution
                if value.value == "DynamicInfo" {
                    let info = extract_dynamic_info(&value.data)?;
                    cache.created = info.created;
                    cache.last_error_code = info.last_error_code;
                    cache.last_run = info.last_run;
                    cache.last_successful_run = info.last_successful_run;
                } else if value.value == "SecurityDescriptor" {
                    cache.security_description = value.data;
                }
            }
            cache_map.insert(entry.name, cache);
        }
    }

    let mut full_cache = HashMap::new();
    // Now loop through our Tree info and complete our TaskCache info
    for entry in tree_info {
        if let Some(value) = cache_map.get_mut(&entry.id) {
            value.registry_tree_path = entry.registry_path;
            full_cache.insert(entry.path.to_lowercase(), value.clone());
        }
    }

    Ok(full_cache)
}

struct DynamicInfo {
    created: String,
    last_run: String,
    last_successful_run: String,
    last_error_code: i32,
}
fn extract_dynamic_info(value: &str) -> Result<DynamicInfo, TaskError> {
    let bytes = match base64_decode_standard(value) {
        Ok(result) => result,
        Err(err) => {
            error!("[tasks] Failed to decode dynamic info: {err:?}");
            return Err(TaskError::Registry);
        }
    };

    let info = match parse_dynamic_info(&bytes) {
        Ok((_, result)) => result,
        Err(err) => {
            error!("[tasks] Failed to parse dynamic info: {err:?}");
            return Err(TaskError::Registry);
        }
    };

    Ok(info)
}

fn parse_dynamic_info(data: &[u8]) -> nom::IResult<&[u8], DynamicInfo> {
    // https://cyber.wtf/2022/06/01/windows-registry-analysis-todays-episode-tasks
    let (input, _version) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, created_bytes) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, last_run_bytes) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, _state) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, last_error_code) = nom_signed_four_bytes(input, Endian::Le)?;
    let (input, last_successful_run_bytes) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let created = unixepoch_to_iso(filetime_to_unixepoch(created_bytes));
    let last_run = unixepoch_to_iso(filetime_to_unixepoch(last_run_bytes));
    let last_successful_run = unixepoch_to_iso(filetime_to_unixepoch(last_successful_run_bytes));
    let info = DynamicInfo {
        created,
        last_run,
        last_successful_run,
        last_error_code,
    };

    Ok((input, info))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::tasks::registry::cache_info;

    #[test]
    fn test_cache_info() {
        let result = cache_info('C').unwrap();
        panic!("stop!");
    }
}
