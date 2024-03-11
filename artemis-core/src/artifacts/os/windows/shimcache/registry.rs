use super::error::ShimcacheError;
use crate::{
    artifacts::os::windows::registry::helper::get_registry_keys, utils::regex_options::create_regex,
};
use log::error;

#[derive(Debug)]
pub(crate) struct ShimcacheReg {
    pub(crate) key_path: String,
    pub(crate) shim_data: String,
}

/// Get `shimcache` entries for all `ControlSet` values
pub(crate) fn get_shimcache_data(path: &str) -> Result<Vec<ShimcacheReg>, ShimcacheError> {
    let start_path = "";
    let regex_value =
        create_regex(r"controlset\d*\\control\\session manager\\appcompatcache").unwrap(); // Always valid

    let encoded_result = get_registry_keys(start_path, &regex_value, path);
    let shim_matches = match encoded_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimcache] Could not get shimcache data from Registry: {err:?}");
            return Err(ShimcacheError::RegistryFile);
        }
    };

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

    #[test]
    fn test_get_shimcache_data() {
        let result = get_shimcache_data("C:\\Windows\\System32\\config\\SYSTEM").unwrap();
        assert!(result.len() > 0);
    }
}
