use super::{
    error::FileSystemError,
    files::{file_lines, list_files_directories},
};
use crate::{
    artifacts::os::systeminfo::info::{PlatformType, get_platform, get_platform_enum},
    utils::environment::get_env_value,
};
use log::{error, warn};
use std::path::Path;
use sysinfo::Users;

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

/// Get directories associated with users on a system
pub(crate) fn get_user_paths() -> Result<Vec<String>, FileSystemError> {
    let plat = get_platform_enum();
    match plat {
        PlatformType::Linux => linux_user_paths(),
        PlatformType::Macos | PlatformType::Windows => {
            let user_path_result = home::home_dir();
            let mut user_path = if let Some(result) = user_path_result {
                result
            } else {
                error!("[forensics] Failed get user home paths");
                return Err(FileSystemError::UserPaths);
            };

            user_path.pop();
            let user_parent = user_path.display().to_string();

            if !is_directory(&user_parent) {
                return Err(FileSystemError::NoUserParent);
            }
            list_directories(&user_parent)
        }
        PlatformType::Unknown => Ok(Vec::new()),
    }
}

/// Get directories associated with users on a system
fn linux_user_paths() -> Result<Vec<String>, FileSystemError> {
    let mut users = Users::new();
    users.refresh();

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

/// Get the path to the root user's home directory
pub(crate) fn get_root_home() -> Result<String, FileSystemError> {
    let plat = get_platform();

    let root_home = if plat == "Windows" {
        get_env_value("SystemRoot")
    } else if plat == "Darwin" {
        String::from("/var/root")
    } else {
        String::from("/root")
    };

    if !is_directory(&root_home) {
        return Err(FileSystemError::NoRootHome);
    }
    Ok(root_home)
}

/// Get the parent directory of a provided path. From: "C:\\Users\\bob\\1.txt" will return "C:\\Users\\bob"
pub(crate) fn get_parent_directory(path: &str) -> String {
    let entry_opt = if path.contains('/') {
        path.rsplit_once('/')
    } else {
        path.rsplit_once('\\')
    };

    if entry_opt.is_none() {
        warn!("[forensics] Failed to get parent directory for path: {path}");
        return path.to_string();
    }

    let (directory, _) = entry_opt.unwrap_or_default();
    directory.to_string()
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::{
        get_parent_directory, get_user_paths, is_directory, list_directories,
    };
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

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_parent_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/fsevents_tester.rs");
        let result = get_parent_directory(&test_location.display().to_string());
        assert!(result.ends_with("tests"));
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_parent_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\fsevents_tester.rs");
        let result = get_parent_directory(&test_location.display().to_string());
        assert!(result.ends_with("tests"));
    }
}
