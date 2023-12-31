use super::{error::FileSystemError, files::list_files_directories};
use std::path::Path;

/// Check if path is a directory
pub(crate) fn is_directory(path: &str) -> bool {
    let dir = Path::new(path);
    if dir.is_dir() {
        return true;
    }
    false
}

/// Get a list of all directories in a provided directory. Use `list_files` to get only files. Use `list_files_directories` to get both files and directories
pub(crate) fn list_directories(path: &str) -> Result<Vec<String>, FileSystemError> {
    let data = list_files_directories(path)?;
    let mut dirs: Vec<String> = Vec::new();

    for entry in data {
        if !is_directory(&entry) {
            continue;
        }
        dirs.push(entry);
    }
    Ok(dirs)
}

#[cfg(any(target_os = "windows", target_os = "macos"))]
/// Get directories associated with users on a system
pub(crate) fn get_user_paths() -> Result<Vec<String>, FileSystemError> {
    use log::error;

    let user_path_result = home::home_dir();
    let mut user_path = if let Some(result) = user_path_result {
        result
    } else {
        error!("[artemis-core] Failed get user home paths");
        return Err(FileSystemError::UserPaths);
    };

    user_path.pop();
    let user_parent = user_path.display().to_string();

    if !is_directory(&user_parent) {
        return Err(FileSystemError::NoUserParent);
    }

    list_directories(&user_parent)
}

#[cfg(target_os = "linux")]
/// Get directories associated with users on a system
pub(crate) fn get_user_paths() -> Result<Vec<String>, FileSystemError> {
    use crate::filesystem::files::file_lines;
    use sysinfo::Users;

    let mut users = Users::new();
    users.refresh_list();

    let passwd_lines = file_lines("/etc/passwd")?;
    let mut user_list: Vec<String> = Vec::new();

    for line_entry in passwd_lines {
        let entry = match line_entry {
            Ok(result) => result,
            Err(_) => continue,
        };
        if entry.contains("nologin") {
            continue;
        }

        for user in users.list() {
            if !entry.contains(&format!("/{}", user.name())) {
                continue;
            }

            let line_split = entry.split(':');
            for (key, split) in line_split.enumerate() {
                let home = 5;
                if key != home {
                    continue;
                }
                if split.contains(&format!("/{}", user.name())) {
                    user_list.push(split.to_string());
                    break;
                }
            }
        }
    }
    Ok(user_list)
}

#[cfg(target_family = "unix")]
/// Get the path to the root user's home directory
pub(crate) fn get_root_home() -> Result<String, FileSystemError> {
    #[cfg(target_os = "macos")]
    let root_home = "/var/root";
    #[cfg(target_os = "linux")]
    let root_home = "/root";

    if !is_directory(root_home) {
        return Err(FileSystemError::NoRootHome);
    }
    Ok(root_home.to_string())
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::{get_user_paths, is_directory, list_directories};
    use std::path::PathBuf;

    #[test]
    fn test_is_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");
        let result = is_directory(&test_location.display().to_string());
        assert!(result);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_user_paths() {
        let result = get_user_paths().unwrap();
        assert!(result.len() >= 4);

        let mut default = false;
        let mut public = false;
        for entry in result {
            if entry.ends_with("Public") {
                public = true;
            } else if entry.ends_with("Default") {
                default = true;
            }
        }
        assert!(default);
        assert!(public);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_user_paths() {
        let result = get_user_paths().unwrap();

        let mut shared = false;
        for entry in result {
            if entry.ends_with("Shared") {
                shared = true;
            }
        }
        assert!(shared);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_user_paths() {
        let result = get_user_paths().unwrap();

        assert!(!result.is_empty());
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_root_home() {
        use crate::filesystem::directory::get_root_home;

        let result = get_root_home().unwrap();
        assert!(result.contains("root"));
    }

    #[test]
    fn test_list_directories() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests");
        let result = list_directories(&test_location.display().to_string()).unwrap();

        assert_eq!(result.len(), 1);
        let mut test_data = false;
        for entry in result {
            if entry.ends_with("test_data") {
                test_data = true;
            }
        }

        assert!(test_data);
    }
}
