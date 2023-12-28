/**
 * macOS launchd (Daemons and Agents) can be used as persistence
 * They exist system wide and per user
 *
 * References:
 *   `https://www.sentinelone.com/blog/how-malware-persists-on-macos/`
 */
use super::error::LaunchdError;
use crate::{
    artifacts::os::macos::plist::property_list::parse_plist_file_dict,
    filesystem::{
        directory::{get_user_paths, is_directory},
        files::list_files,
    },
    structs::artifacts::os::macos::LaunchdOptions,
};
use common::macos::LaunchdPlist;
use log::{error, warn};

/// Grab `LuanchDaemons` and `LaunchAgents`
pub(crate) fn grab_launchd(options: &LaunchdOptions) -> Result<Vec<LaunchdPlist>, LaunchdError> {
    if let Some(alt_file) = &options.alt_file {
        let results = parse_plist_file_dict(alt_file);
        let launchd_data = match results {
            Ok(launchd_data_dictionary) => LaunchdPlist {
                launchd_data: launchd_data_dictionary,
                plist_path: alt_file.to_string(),
            },
            Err(err) => {
                warn!("[launchd] Failed to parse plist file {alt_file}: {err:?}");
                return Err(LaunchdError::Files);
            }
        };

        return Ok(vec![launchd_data]);
    }

    let mut launchd = grab_launchd_daemons()?;
    let mut agents = grab_launchd_agents()?;
    launchd.append(&mut agents);
    Ok(launchd)
}

/// Get and parse System and User launchd Daemons
pub(crate) fn grab_launchd_daemons() -> Result<Vec<LaunchdPlist>, LaunchdError> {
    let mut plist_files: Vec<String> = Vec::new();
    let user_launchd = user_launchd_daemons();
    match user_launchd {
        Ok(mut launchd_data) => plist_files.append(&mut launchd_data),
        Err(err) => warn!("[launchd] Failed to get user launchd daemon plist files: {err:?}"),
    }

    let system_launchd = system_launchd_daemons();
    match system_launchd {
        Ok(mut launchd_data) => plist_files.append(&mut launchd_data),
        Err(err) => warn!("[launchd] Failed to get system launchd daemon plist files: {err:?}"),
    }

    let mut launchd_plist_vec: Vec<LaunchdPlist> = Vec::new();
    for data in plist_files {
        if !data.ends_with("plist") {
            continue;
        }

        let launchd_results = parse_plist_file_dict(&data);
        match launchd_results {
            Ok(launchd_data_dictionary) => {
                let launchd_data = LaunchdPlist {
                    launchd_data: launchd_data_dictionary,
                    plist_path: data,
                };
                launchd_plist_vec.push(launchd_data);
            }
            Err(err) => warn!("[launchd] Failed to parse plist file {data}: {err:?}"),
        }
    }
    Ok(launchd_plist_vec)
}

/// Get and parse System and User launchd Agents
pub(crate) fn grab_launchd_agents() -> Result<Vec<LaunchdPlist>, LaunchdError> {
    let mut plist_files: Vec<String> = Vec::new();
    let user_launchd = user_launchd_agents();
    match user_launchd {
        Ok(mut launchd_data) => plist_files.append(&mut launchd_data),
        Err(err) => warn!("[launchd] Failed to get user launchd agent plist files: {err:?}"),
    }

    let system_launchd = system_launchd_agents();
    match system_launchd {
        Ok(mut launchd_data) => plist_files.append(&mut launchd_data),
        Err(err) => warn!("[launchd] Failed to get system launchd agent plist files: {err:?}"),
    }

    let mut launchd_plist_vec: Vec<LaunchdPlist> = Vec::new();
    for data in plist_files {
        if !data.ends_with("plist") {
            continue;
        }

        let launchd_results = parse_plist_file_dict(&data);
        match launchd_results {
            Ok(launchd_data_dictionary) => {
                let launchd_data = LaunchdPlist {
                    launchd_data: launchd_data_dictionary,
                    plist_path: data,
                };
                launchd_plist_vec.push(launchd_data);
            }
            Err(err) => warn!("[launchd] Failed to parse plist file {data}: {err:?}"),
        }
    }
    Ok(launchd_plist_vec)
}

/// Get User Launchd daemons
fn user_launchd_daemons() -> Result<Vec<String>, LaunchdError> {
    let user_daemons = "/Library/LaunchDaemons/";
    launchd_data(user_daemons)
}

/// Get System Launchd daemons
fn system_launchd_daemons() -> Result<Vec<String>, LaunchdError> {
    let system_daemons = [
        "/System/Library/LaunchDaemons/",
        "/Library/Apple/System/Library/LaunchDaemons/",
    ];
    let mut daemons: Vec<String> = Vec::new();
    for paths in system_daemons {
        let mut results = launchd_data(paths)?;
        daemons.append(&mut results);
    }
    Ok(daemons)
}

/// Get System Launchd Agents
fn system_launchd_agents() -> Result<Vec<String>, LaunchdError> {
    let system_agents = [
        "/System/Library/LaunchAgents/",
        "/Library/Apple/System/Library/LaunchAgents/",
    ];
    let mut agents: Vec<String> = Vec::new();
    for paths in system_agents {
        let mut results = launchd_data(paths)?;
        agents.append(&mut results);
    }
    Ok(agents)
}

/// Get User launchd Agents
fn user_launchd_agents() -> Result<Vec<String>, LaunchdError> {
    let user_paths_result = get_user_paths();
    let user_paths = match user_paths_result {
        Ok(result) => result,
        Err(_) => return Err(LaunchdError::UserPath),
    };

    let agents_path = "/Library/LaunchAgents/";
    let mut agent_plist_files: Vec<String> = Vec::new();

    for user_path in user_paths {
        let path = format!("{user_path}{agents_path}");
        if !is_directory(&path) {
            continue;
        }

        let mut plist_files = launchd_data(&path)?;
        agent_plist_files.append(&mut plist_files);
    }

    let mut results = launchd_data(agents_path)?;
    agent_plist_files.append(&mut results);
    Ok(agent_plist_files)
}

/// Get PLIST files from directory
fn launchd_data(path: &str) -> Result<Vec<String>, LaunchdError> {
    let files_results = list_files(path);
    let files = match files_results {
        Ok(result) => result,
        Err(err) => {
            error!("[launchd] Could not list plist files: {err:?}");
            return Err(LaunchdError::Files);
        }
    };
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::{
        grab_launchd, grab_launchd_agents, grab_launchd_daemons, system_launchd_agents,
        system_launchd_daemons, user_launchd_agents, user_launchd_daemons,
    };
    use crate::{
        artifacts::os::macos::launchd::launchdaemon::launchd_data,
        structs::artifacts::os::macos::LaunchdOptions,
    };

    #[test]
    fn test_grab_launchd() {
        let results = grab_launchd(&LaunchdOptions { alt_file: None }).unwrap();
        assert!(results.len() > 5);
    }

    #[test]
    fn test_grab_launchd_daemons() {
        let results = grab_launchd_daemons().unwrap();
        assert!(results.len() > 5);
    }

    #[test]
    fn test_grab_launchd_agents() {
        let results = grab_launchd_agents().unwrap();
        assert!(results.len() > 5);
    }

    #[test]
    fn test_user_launchd_daemons() {
        let _ = user_launchd_daemons().unwrap();
    }

    #[test]
    fn test_system_launchd_daemons() {
        let results = system_launchd_daemons().unwrap();
        assert!(results.len() > 5);
    }

    #[test]
    fn test_system_launchd_agents() {
        let results = system_launchd_agents().unwrap();
        assert!(results.len() > 5);
    }

    #[test]
    fn test_user_launchd_agents() {
        let _ = user_launchd_agents().unwrap();
    }

    #[test]
    fn test_launchd_data() {
        let results = launchd_data("/System/Library/LaunchAgents/").unwrap();
        assert!(results.len() > 5);
    }
}
