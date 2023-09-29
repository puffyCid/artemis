use super::artifacts::Options;
use artemis_core::{
    core::artemis_collection,
    structs::{
        artifacts::os::{files::FileOptions, macos::UnifiedLogsOptions, processes::ProcessOptions},
        toml::{ArtemisToml, Artifacts, Output},
    },
};
use clap::{arg, Subcommand};

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Acquire forensic artifacts
    Acquire {
        /// Collect processes
        #[arg(long)]
        processes: bool,
        /// Pull filelisting
        #[arg(long)]
        files: bool,
        /// Parse Unfied Logs
        #[arg(long)]
        unifiedlogs: bool,
        /// Parse LoginItems
        #[arg(long)]
        loginitems: bool,
        /// Parse Emond
        #[arg(long)]
        emond: bool,
        /// Parse FsEvents
        #[arg(long)]
        fsevents: bool,
        /// Parse Launchd
        #[arg(long)]
        launchd: bool,
        /// Parse Users
        #[arg(long)]
        users: bool,
        /// Parse Groups
        #[arg(long)]
        groups: bool,
        /// Get systeminfo
        #[arg(long)]
        systeminfo: bool,
        /// Parse ExecPolicy
        #[arg(long)]
        execpolicy: bool,
        /// Parse Safari History and Downloads
        #[arg(long)]
        safari: bool,
        /// Parse Firefox History and Downloads
        #[arg(long)]
        firefox: bool,
        /// Parse Chromium History and Downloads
        #[arg(long)]
        chromium: bool,
        /// Parse Shellhistory
        #[arg(long)]
        shellhistory: bool,
        /// Parse Cron Jobs
        #[arg(long)]
        cron: bool,
        /// Grab Sudo logs
        #[arg(long)]
        sudologs: bool,
        /// Output format. JSON or JSON.
        #[arg(long, default_value_t = String::from("json"))]
        format: String,
    },
}

/// Run the macOS collector and parse specified artifacts
pub(crate) fn run_collector(command: &Commands, output: Output) {
    let mut collector = ArtemisToml {
        system: String::from("macos"),
        output,
        artifacts: Vec::new(),
    };
    println!(
        "[artemis] Writing output to: {}",
        collector.output.directory
    );

    match command {
        Commands::Acquire {
            processes,
            files,
            unifiedlogs,
            emond,
            loginitems,
            launchd,
            safari,
            firefox,
            chromium,
            users,
            groups,
            fsevents,
            systeminfo,
            shellhistory,
            execpolicy,
            cron,
            sudologs,
            format,
        } => {
            if *processes {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Processes, "processes"))
            }
            if *files {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Files, "files"))
            }
            if *unifiedlogs {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Unifiedlogs, "unifiedlogs"))
            }
            if *emond {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "emond"))
            }
            if *loginitems {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "loginitems"))
            }
            if *safari {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "safari-downloads"));
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "safari-history"))
            }
            if *launchd {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "launchd"))
            }
            if *firefox {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "firefox-downloads"));
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "firefox-history"))
            }
            if *chromium {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "chromium-downloads"));
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "chromium-history"))
            }
            if *users {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "users"))
            }
            if *groups {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "groups"))
            }
            if *fsevents {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "fseventsd"))
            }
            if *systeminfo {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "systeminfo"))
            }
            if *shellhistory {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "shell_history"))
            }
            if *execpolicy {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "execpolicy"))
            }
            if *cron {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "cron"))
            }
            if *sudologs {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "sudologs"))
            }
            if !format.is_empty() {
                collector.output.format = format.to_string();
            }
        }
    }

    artemis_collection(&mut collector).unwrap();
}

/// Setup any artifact options. Only a few have options on macOS
fn setup_artifact(artifact: Options, name: &str) -> Artifacts {
    let mut collect = Artifacts {
        artifact_name: name.to_string(),
        filter: None,
        processes: None,
        files: None,
        unifiedlogs: None,
        script: None,
    };
    match artifact {
        Options::Processes => {
            let options = ProcessOptions {
                md5: true,
                sha1: false,
                sha256: false,
                metadata: false,
            };
            collect.processes = Some(options);
        }
        Options::Unifiedlogs => {
            let options = UnifiedLogsOptions {
                sources: vec![
                    String::from("Persist"),
                    String::from("Special"),
                    String::from("HighVolume"),
                    String::from("Signpost"),
                ],
            };
            collect.unifiedlogs = Some(options);
        }
        Options::Files => {
            let options = FileOptions {
                start_path: String::from("/"),
                depth: Some(100),
                metadata: None,
                md5: Some(true),
                sha1: None,
                sha256: None,
                regex_filter: None,
            };
            collect.files = Some(options);
        }
        _ => {}
    }
    collect
}

#[cfg(test)]
mod tests {
    use super::{run_collector, setup_artifact, Commands};
    use crate::collector::artifacts::Options;
    use artemis_core::structs::toml::Output;

    #[test]
    fn test_run_collector() {
        let command = Commands::Acquire {
            processes: true,
            files: false,
            unifiedlogs: false,
            loginitems: true,
            emond: true,
            fsevents: false,
            launchd: true,
            users: false,
            groups: false,
            systeminfo: true,
            execpolicy: false,
            safari: true,
            firefox: true,
            chromium: true,
            shellhistory: true,
            cron: true,
            sudologs: false,
            format: String::from("json"),
        };

        let out = Output {
            name: String::from("local_collector"),
            endpoint_id: String::from("local"),
            collection_id: 0,
            directory: String::from("./tmp"),
            output: String::from("local"),
            format: String::from("json"),
            compress: false,
            filter_name: None,
            filter_script: None,
            url: None,
            api_key: None,
            logging: None,
        };

        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_root() {
        let command = Commands::Acquire {
            processes: false,
            files: false,
            unifiedlogs: false,
            loginitems: false,
            emond: false,
            fsevents: true,
            launchd: false,
            users: true,
            groups: true,
            systeminfo: false,
            execpolicy: true,
            safari: false,
            firefox: false,
            chromium: false,
            shellhistory: false,
            cron: false,
            sudologs: false,
            format: String::from("jsonl"),
        };

        let out = Output {
            name: String::from("root_local_collector"),
            endpoint_id: String::from("local"),
            collection_id: 0,
            directory: String::from("./tmp"),
            output: String::from("local"),
            format: String::from("json"),
            compress: false,
            filter_name: None,
            filter_script: None,
            url: None,
            api_key: None,
            logging: None,
        };

        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_sudo() {
        let command = Commands::Acquire {
            processes: false,
            files: false,
            unifiedlogs: false,
            loginitems: false,
            emond: false,
            fsevents: false,
            launchd: false,
            users: false,
            groups: false,
            systeminfo: false,
            execpolicy: false,
            safari: false,
            firefox: false,
            chromium: false,
            shellhistory: false,
            cron: false,
            sudologs: true,
            format: String::from("json"),
        };

        let out = Output {
            name: String::from("sudo_local_collector"),
            endpoint_id: String::from("local"),
            collection_id: 0,
            directory: String::from("./tmp"),
            output: String::from("local"),
            format: String::from("json"),
            compress: false,
            filter_name: None,
            filter_script: None,
            url: None,
            api_key: None,
            logging: None,
        };

        run_collector(&command, out);
    }

    #[test]
    fn test_setup_artifact() {
        let result = setup_artifact(Options::None, "loginitems");
        assert_eq!(result.artifact_name, "loginitems");
    }
}
