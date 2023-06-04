use super::{error::FileSystemError, files::list_files_directories};
use log::error;
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

/// Get directories associated with users on a system
pub(crate) fn get_user_paths() -> Result<Vec<String>, FileSystemError> {
    let user_path_result = home::home_dir();
    let mut user_path = if let Some(result) = user_path_result {
        result
    } else {
        error!("[artemis-core] Failed get user home paths");
        return Err(FileSystemError::UserPaths);
    };

    let user_parent = if user_path.has_root() {
        #[cfg(target_os = "windows")]
        {
            format!("{}Users", &user_path.display().to_string()[0..3])
        }

        #[cfg(target_os = "macos")]
        {
            String::from("/Users")
        }

        #[cfg(target_os = "linux")]
        {
            user_path.pop();
            user_path.display().to_string()
        }
    } else {
        error!("[artemis-core] Failed get user base paths");
        return Err(FileSystemError::NoUserParent);
    };

    if !is_directory(&user_parent) {
        return Err(FileSystemError::NoUserParent);
    }

    list_directories(&user_parent)
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
        assert_eq!(result, true);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_user_win_paths() {
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
        assert_eq!(default, true);
        assert_eq!(public, true);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_user_macos_paths() {
        let result = get_user_paths().unwrap();

        let mut shared = false;
        for entry in result {
            if entry.ends_with("Shared") {
                shared = true;
            }
        }
        assert_eq!(shared, true);
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_get_root_home() {
        use crate::filesystem::directory::get_root_home;

        let result = get_root_home().unwrap();
        assert_eq!(result.contains("root"), true);
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

        assert_eq!(test_data, true);
    }
}
