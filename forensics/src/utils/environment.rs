use super::error::ArtemisError;
use super::regex_options::create_regex;
use crate::artifacts::os::windows::registry::helper::get_registry_keys;
use log::error;
use std::collections::HashMap;
use std::env::vars_os;

/// Get the `SystemDrive` for Windows
pub(crate) fn get_systemdrive() -> Result<char, ArtemisError> {
    let envs = get_env();
    let mut update_env = HashMap::new();
    // ENV keys are insensitive so we lower case all env keys. Ex: %systemroot% == %SystemRoot%
    for (key, value) in envs {
        update_env.insert(key.to_lowercase(), value);
    }

    if let Some(value) = update_env.get("systemdrive")
        && let Some(drive) = value.chars().next()
    {
        return Ok(drive);
    }
    error!("[forensics] Empty systemdrive value");
    Err(ArtemisError::Env)
}

/// Get Folder descriptions that map CLSIDs to a directory name
pub(crate) fn get_folder_descriptions() -> Result<HashMap<String, String>, ArtemisError> {
    let systemdrive = get_systemdrive()?;
    let path = format!("{systemdrive}:\\Windows\\System32\\config\\SOFTWARE");
    let reg_start = "";
    let path_regex =
        create_regex(r".*\\microsoft\\windows\\currentversion\\explorer\\folderdescriptions.*}$")?;
    let reg_results = get_registry_keys(reg_start, &path_regex, &path);
    let reg_values = match reg_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Could not get folder descriptions: {err:?}",);
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

/// Get Folder descriptions that map CLSIDs to a directory name
pub(crate) fn get_clsids() -> Result<HashMap<String, String>, ArtemisError> {
    let systemdrive = get_systemdrive()?;
    let path = format!("{systemdrive}:\\Windows\\System32\\config\\SOFTWARE");
    let reg_start = "";
    let path_regex = create_regex(r".*\\classes\\clsid\\.*}$")?;
    let reg_results = get_registry_keys(reg_start, &path_regex, &path);
    let reg_values = match reg_results {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Could not get CLSIDs: {err:?}");
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
        return env.clone();
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
    #[cfg(target_family = "unix")]
    fn test_get_env_value() {
        let result = get_env_value("PATH");
        assert!(!result.is_empty())
    }
}
