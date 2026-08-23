use super::error::ShimcacheError;
use crate::{
    accessor::entry::handle::FileHandle,
    artifacts::os::windows::registry::helper::get_registry_keys_handle,
    utils::regex_options::create_regex,
};
use tracing::{debug, error, info};

#[derive(Debug)]
pub(crate) struct ShimcacheReg {
    pub(crate) key_path: String,
    pub(crate) shim_data: String,
}

/// Get `shimcache` entries for all `ControlSet` values
pub(crate) fn get_shimcache_data(handle: &FileHandle) -> Result<Vec<ShimcacheReg>, ShimcacheError> {
    info!("Reading Shimcache file: {}", handle.display_path());

    let start_path = String::new();
    let pattern = r"controlset\d*\\control\\session manager\\appcompatcache";
    let regex_value = create_regex(pattern).unwrap(); // Always valid

    let encoded_result = get_registry_keys_handle(start_path, regex_value, handle);
    let shim_matches = match encoded_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not get shimcache data from Registry: {err:?}");
            return Err(ShimcacheError::RegistryFile);
        }
    };
    debug!(
        "Got {} Shimcache Registry values from regex '{pattern}'",
        shim_matches.len()
    );

    let mut shim_vec: Vec<ShimcacheReg> = Vec::new();
    for entry in shim_matches {
        for value in entry.values {
            if value.value != "AppCompatCache" {
                continue;
            }
            let shim_value = ShimcacheReg {
                key_path: entry.path.clone(),
                shim_data: value.data,
            };
            shim_vec.push(shim_value);
        }
    }
    Ok(shim_vec)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::get_shimcache_data;
    use crate::accessor::access::Accessor;

    #[test]
    fn test_get_shimcache_data() {
        let handle = Accessor::with_defaults()
            .globfs("ntfs:C:\\Windows\\System32\\config\\SYSTEM")
            .unwrap();
        let result = get_shimcache_data(handle[0].handle.as_file().unwrap()).unwrap();
        assert!(result.len() > 0);
    }
}
