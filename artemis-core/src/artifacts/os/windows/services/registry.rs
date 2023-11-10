use super::error::ServicesError;
use crate::{
    artifacts::os::windows::registry::helper::get_registry_keys, utils::regex_options::create_regex,
};
use common::windows::RegistryEntry;
use log::error;

/// Parse provided Registry file (SYSTEM) and get Services information
pub(crate) fn get_services_data(path: &str) -> Result<Vec<RegistryEntry>, ServicesError> {
    let start_path = "";
    let regex = create_regex(r".*\\controlset([0-9]+)\\services\\.*").unwrap(); // always valid

    let entries_result = get_registry_keys(start_path, &regex, path);
    let entries = match entries_result {
        Ok(result) => result,
        Err(err) => {
            error!("[services] Failed to parse Registry: {err:?}");
            return Err(ServicesError::RegistryFiles);
        }
    };

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::get_services_data;
    use crate::utils::environment::get_systemdrive;

    #[test]
    fn test_get_services_data() {
        let drive = get_systemdrive().unwrap();
        let path = format!("{drive}:\\Windows\\System32\\config\\SYSTEM");
        let results = get_services_data(&path).unwrap();

        assert!(results.len() > 10);
    }
}
