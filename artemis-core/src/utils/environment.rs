use std::collections::HashMap;
use std::env::vars_os;

#[cfg(target_os = "windows")]
use super::error::ArtemisError;
#[cfg(target_os = "windows")]
use log::error;

#[cfg(target_os = "windows")]
/// Get the `SystemDrive` for Windows
pub(crate) fn get_systemdrive() -> Result<char, ArtemisError> {
    let sys_drive = get_env_value("SystemDrive");

    if sys_drive.is_empty() {
        error!("[artemis-core] Empty systemdrive value");
        return Err(ArtemisError::Env);
    }
    // unwrap should be safe since we check for at least one value in string
    Ok(sys_drive_result.chars().next().unwrap())
}

#[cfg(target_os = "windows")]
/// Get Folder descriptions that map CLSIDs to a directory name
pub(crate) fn get_folder_descriptions() -> Result<HashMap<String, String>, ArtemisError> {
    use crate::artifacts::os::windows::registry::helper::get_registry_keys;
    use crate::utils::regex_options::create_regex;

    let systemdrive = get_systemdrive()?;
    let path = format!("{systemdrive}:\\Windows\\System32\\config\\SOFTWARE");
    let reg_start = "";
    let path_regex =
        create_regex(r".*\\microsoft\\windows\\currentversion\\explorer\\folderdescriptions.*}$")?;
    let reg_results = get_registry_keys(reg_start, &path_regex, &path);
    let reg_values = match reg_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Could not get folder descriptions: {err:?}",);
            return Err(ArtemisError::Env);
        }
    };

    let mut folder_descriptions = HashMap::new();
    for entry in reg_values {
        if entry.name.ends_with('}') {
            for value in entry.values {
                if value.value == "Name" {
                    folder_descriptions.insert(entry.name.clone(), value.data);
                }
            }
        }
    }
    Ok(folder_descriptions)
}

#[cfg(target_os = "windows")]
/// Get Folder descriptions that map CLSIDs to a directory name
pub(crate) fn get_clsids() -> Result<HashMap<String, String>, ArtemisError> {
    use crate::artifacts::os::windows::registry::helper::get_registry_keys;
    use crate::utils::regex_options::create_regex;

    let systemdrive = get_systemdrive()?;
    let path = format!("{systemdrive}:\\Windows\\System32\\config\\SOFTWARE");
    let reg_start = "";
    let path_regex = create_regex(r".*\\classes\\clsid\\.*}$")?;
    let reg_results = get_registry_keys(reg_start, &path_regex, &path);
    let reg_values = match reg_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Could not get CLSIDs: {err:?}");
            return Err(ArtemisError::Env);
        }
    };

    let mut clsids = HashMap::new();
    for entry in reg_values {
        if entry.name.ends_with('}') {
            for value in entry.values {
                if value.value == "(default)" {
                    clsids.insert(entry.name.clone(), value.data);
                }
            }
        }
    }
    Ok(clsids)
}

/// Get a specific environment variable value
pub(crate) fn get_env_value(value: &str) -> String {
    let envs = get_env();
    if let Some(env) = envs.get(value) {
        return env.to_string();
    }
    String::new()
}

/// Get all environment variables associated with artemis process
pub(crate) fn get_env() -> HashMap<String, String> {
    let envs = vars_os();
    let mut environment = HashMap::new();
    for (key, value) in envs {
        environment.insert(
            key.into_string().unwrap_or_default(),
            value.into_string().unwrap_or_default(),
        );
    }
    environment
}

#[cfg(test)]
mod tests {
    use super::get_env_value;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_systemdrive() {
        use super::get_systemdrive;

        let result = get_systemdrive().unwrap();
        assert_eq!(result, 'C')
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_folder_descriptions() {
        use super::get_folder_descriptions;

        let result = get_folder_descriptions().unwrap();
        assert!(result.len() > 40)
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_clsids() {
        use super::get_clsids;

        let result = get_clsids().unwrap();
        assert!(result.len() > 40);
        assert_eq!(
            result
                .get(&"{20d04fe0-3aea-1069-a2d8-08002b30309d}".to_uppercase())
                .unwrap(),
            "This PC"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_env_value() {
        let result = get_env_value("PUBLIC");
        assert_eq!(result, "C:\\Users\\Public")
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_env_value() {
        let result = get_env_value("PATH");
        assert!(!result.is_empty())
    }
}
