use super::error::ServicesError;
use crate::{
    accessor::entry::handle::FileHandle,
    artifacts::os::windows::registry::helper::get_registry_keys_handle,
    utils::regex_options::create_regex,
};
use common::windows::RegistryData;
use tracing::error;

/// Parse provided Registry file (SYSTEM) and get Services information
pub(crate) fn get_services_data(handle: &FileHandle) -> Result<Vec<RegistryData>, ServicesError> {
    let start_path = String::new();
    let regex = create_regex(r".*\\controlset([0-9]+)\\services\\.*").unwrap(); // always valid

    let entries_result = get_registry_keys_handle(start_path, regex, handle);
    let entries = match entries_result {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to parse Registry: {err:?}");
            return Err(ServicesError::RegistryFiles);
        }
    };

    Ok(entries)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::get_services_data;
    use crate::{accessor::access::Accessor, utils::environment::get_systemdrive};

    #[test]
    fn test_get_services_data() {
        let drive = get_systemdrive().unwrap();
        let path = format!("ntfs:{drive}:\\Windows\\System32\\config\\SYSTEM");
        let handle = Accessor::with_defaults().globfs(&path).unwrap();
        let results = get_services_data(handle[0].handle.as_file().unwrap()).unwrap();

        assert!(results.len() > 10);
    }
}
