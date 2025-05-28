use std::{collections::HashMap, env::vars_os};

/// Get a specific environment variable value
pub(crate) fn get_env_value(value: &str) -> String {
    let envs = get_env();
    if let Some(env) = envs.get(value) {
        return env.to_string();
    }
    String::new()
}

/// Get all environment variables associated with artemis process
fn get_env() -> HashMap<String, String> {
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
    fn test_get_env_value() {
        let result = get_env_value("ProgramData");
        assert_eq!(result, "C:\\ProgramData")
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_env_value() {
        let result = get_env_value("PATH");
        assert!(!result.is_empty())
    }
}
