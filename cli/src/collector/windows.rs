use super::commands::CommandArgs;
use artemis_core::{
    core::artemis_collection,
    structs::{
        artifacts::os::{
            files::FileOptions,
            processes::ProcessOptions,
            windows::{
                AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, PrefetchOptions,
                RawFilesOptions, RecycleBinOptions, RegistryOptions, SearchOptions,
                ServicesOptions, ShellbagsOptions, ShimcacheOptions, ShimdbOptions,
                ShortcutOptions, SrumOptions, TasksOptions, UserAssistOptions, UserOptions,
                UsnJrnlOptions, WmiPersistOptions,
            },
        },
        toml::{ArtemisToml, Artifacts, Output},
    },
};
use clap::{arg, Subcommand};

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Acquire forensic artifacts
    Acquire {
        #[command(subcommand)]
        artifact: Option<CommandArgs>,
        /// Output format. JSON or JSONL.
        #[arg(long, default_value_t = String::from("JSON"))]
        format: String,
    },
}

/// Run the Windows collector and parse specified artifacts
pub(crate) fn run_collector(command: &Commands, output: Output) {
    let mut collector = ArtemisToml {
        system: String::from("windows"),
        output,
        artifacts: Vec::new(),
    };
    println!(
        "[artemis] Writing output to: {}",
        collector.output.directory
    );

    match command {
        Commands::Acquire { artifact, format } => {
            if artifact.is_none() {
                println!("No artifact provided");
                return;
            }

            let arti = artifact.as_ref().unwrap();
            collector.artifacts.push(setup_artifact(arti));

            if !format.is_empty() {
                collector.output.format = format.to_string().to_lowercase();
            }
        }
    }
    artemis_collection(&mut collector).unwrap();
}

/// Setup any artifact options
fn setup_artifact(artifact: &CommandArgs) -> Artifacts {
    let mut collect = Artifacts {
        artifact_name: String::new(),
        filter: None,
        processes: None,
        files: None,
        script: None,
        eventlogs: None,
        prefetch: None,
        rawfiles: None,
        shimdb: None,
        registry: None,
        userassist: None,
        shimcache: None,
        shellbags: None,
        amcache: None,
        shortcuts: None,
        usnjrnl: None,
        bits: None,
        srum: None,
        users: None,
        search: None,
        tasks: None,
        services: None,
        jumplists: None,
        recyclebin: None,
        wmipersist: None,
    };
    match artifact {
        CommandArgs::Processes {
            md5,
            sha1,
            sha256,
            metadata,
        } => {
            let options = ProcessOptions {
                md5: *md5,
                sha1: *sha1,
                sha256: *sha256,
                metadata: *metadata,
            };
            collect.processes = Some(options);
            collect.artifact_name = String::from("processes");
        }
        CommandArgs::Filelisting {
            md5,
            sha1,
            sha256,
            metadata,
            start_path,
            depth,
            regex_filter,
        } => {
            let options = FileOptions {
                md5: Some(*md5),
                start_path: start_path.to_string(),
                depth: Some(*depth),
                metadata: Some(*metadata),
                sha1: Some(*sha1),
                sha256: Some(*sha256),
                regex_filter: regex_filter.clone(),
            };
            collect.files = Some(options);
            collect.artifact_name = String::from("files");
        }
        CommandArgs::Chromiumhistory {} => collect.artifact_name = String::from("chromium-history"),
        CommandArgs::Chromiumdownloads {} => {
            collect.artifact_name = String::from("chromium-downloads")
        }
        CommandArgs::Firefoxdownloads {} => {
            collect.artifact_name = String::from("firefox-downloads")
        }
        CommandArgs::Firefoxhistory {} => collect.artifact_name = String::from("firefox-history"),
        CommandArgs::Systeminfo {} => collect.artifact_name = String::from("systeminfo"),
        CommandArgs::Amcache { alt_file } => {
            let options = AmcacheOptions {
                alt_file: alt_file.clone(),
            };
            collect.amcache = Some(options);
            collect.artifact_name = String::from("amcache");
        }
        CommandArgs::Bits { carve, alt_file } => {
            let options = BitsOptions {
                carve: *carve,
                alt_file: alt_file.clone(),
            };
            collect.bits = Some(options);
            collect.artifact_name = String::from("bits");
        }
        CommandArgs::Eventlogs { alt_file } => {
            let options = EventLogsOptions {
                alt_file: alt_file.clone(),
            };
            collect.eventlogs = Some(options);
            collect.artifact_name = String::from("eventlogs");
        }
        CommandArgs::Jumplists { alt_file } => {
            let options = JumplistsOptions {
                alt_file: alt_file.clone(),
            };
            collect.jumplists = Some(options);
            collect.artifact_name = String::from("jumplists");
        }
        CommandArgs::Prefetch { alt_dir } => {
            let options = PrefetchOptions {
                alt_dir: alt_dir.clone(),
            };
            collect.prefetch = Some(options);
            collect.artifact_name = String::from("prefetch");
        }
        CommandArgs::Rawfilelisting {
            drive_letter,
            start_path,
            depth,
            recover_indx,
            md5,
            sha1,
            sha256,
            metadata,
            path_regex,
            filename_regex,
        } => {
            let options = RawFilesOptions {
                drive_letter: *drive_letter,
                start_path: start_path.clone(),
                depth: *depth,
                recover_indx: *recover_indx,
                md5: Some(*md5),
                sha1: Some(*sha1),
                sha256: Some(*sha256),
                metadata: Some(*metadata),
                path_regex: path_regex.clone(),
                filename_regex: filename_regex.clone(),
            };
            collect.rawfiles = Some(options);
            collect.artifact_name = String::from("rawfiles");
        }
        CommandArgs::Recyclebin { alt_file } => {
            let options = RecycleBinOptions {
                alt_file: alt_file.clone(),
            };
            collect.recyclebin = Some(options);
            collect.artifact_name = String::from("recyclebin");
        }
        CommandArgs::Registry {
            user_hives,
            system_hives,
            alt_file,
            path_regex,
        } => {
            let options = RegistryOptions {
                user_hives: *user_hives,
                system_hives: *system_hives,
                alt_file: alt_file.clone(),
                path_regex: path_regex.clone(),
            };
            collect.registry = Some(options);
            collect.artifact_name = String::from("registry");
        }
        CommandArgs::Search { alt_file } => {
            let options = SearchOptions {
                alt_file: alt_file.clone(),
            };
            collect.search = Some(options);
            collect.artifact_name = String::from("search");
        }
        CommandArgs::Services { alt_file } => {
            let options = ServicesOptions {
                alt_file: alt_file.clone(),
            };
            collect.services = Some(options);
            collect.artifact_name = String::from("services");
        }
        CommandArgs::Shellbags {
            resolve_guids,
            alt_file,
        } => {
            let options = ShellbagsOptions {
                resolve_guids: *resolve_guids,
                alt_file: alt_file.clone(),
            };
            collect.shellbags = Some(options);
            collect.artifact_name = String::from("shellbags");
        }
        CommandArgs::Shimcache { alt_file } => {
            let options = ShimcacheOptions {
                alt_file: alt_file.clone(),
            };
            collect.shimcache = Some(options);
            collect.artifact_name = String::from("shimcache");
        }
        CommandArgs::Shimdb { alt_file } => {
            let options = ShimdbOptions {
                alt_file: alt_file.clone(),
            };
            collect.shimdb = Some(options);
            collect.artifact_name = String::from("shimdb");
        }
        CommandArgs::Shortcuts { path } => {
            let options = ShortcutOptions { path: path.clone() };
            collect.shortcuts = Some(options);
            collect.artifact_name = String::from("shortcuts");
        }
        CommandArgs::Srum { alt_file } => {
            let options = SrumOptions {
                alt_file: alt_file.clone(),
            };
            collect.srum = Some(options);
            collect.artifact_name = String::from("srum");
        }
        CommandArgs::Tasks { alt_file } => {
            let options = TasksOptions {
                alt_file: alt_file.clone(),
            };
            collect.tasks = Some(options);
            collect.artifact_name = String::from("tasks");
        }
        CommandArgs::Userassist {
            alt_file,
            resolve_descriptions,
        } => {
            let options = UserAssistOptions {
                alt_file: alt_file.clone(),
                resolve_descriptions: *resolve_descriptions,
            };
            collect.userassist = Some(options);
            collect.artifact_name = String::from("userassist");
        }
        CommandArgs::Users { alt_file } => {
            let options = UserOptions {
                alt_file: alt_file.clone(),
            };
            collect.users = Some(options);
            collect.artifact_name = String::from("users");
        }
        CommandArgs::Usnjrnl { alt_drive } => {
            let options = UsnJrnlOptions {
                alt_drive: *alt_drive,
            };
            collect.usnjrnl = Some(options);
            collect.artifact_name = String::from("usnjrnl");
        }
        CommandArgs::Wmipersist { alt_dir } => {
            let options = WmiPersistOptions {
                alt_dir: alt_dir.clone(),
            };
            collect.wmipersist = Some(options);
            collect.artifact_name = String::from("wmipersist");
        }
    }
    collect
}

#[cfg(test)]
mod tests {
    use super::{run_collector, setup_artifact, Commands};
    use crate::collector::windows::CommandArgs::{
        Amcache, Bits, Chromiumdownloads, Chromiumhistory, Eventlogs, Filelisting,
        Firefoxdownloads, Firefoxhistory, Jumplists, Prefetch, Processes, Rawfilelisting,
        Recyclebin, Registry, Services, Shellbags, Shimcache, Shimdb, Srum, Systeminfo, Tasks,
        Users,
    };
    use artemis_core::structs::toml::Output;

    fn output() -> Output {
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

        out
    }

    #[test]
    fn test_run_collector_proc() {
        let command = Commands::Acquire {
            artifact: Some(Processes {
                md5: true,
                sha1: false,
                sha256: false,
                metadata: false,
            }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_files() {
        let command = Commands::Acquire {
            artifact: Some(Filelisting {
                md5: true,
                sha1: false,
                sha256: false,
                metadata: false,
                start_path: String::from("/"),
                depth: 1,
                regex_filter: None,
            }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_quick() {
        let command = Commands::Acquire {
            artifact: Some(Chromiumdownloads {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Chromiumhistory {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Firefoxdownloads {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Firefoxhistory {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Systeminfo {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_reg() {
        let command = Commands::Acquire {
            artifact: Some(Registry {
                user_hives: true,
                system_hives: false,
                alt_file: None,
                path_regex: None,
            }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_eventlogs() {
        let command = Commands::Acquire {
            artifact: Some(Eventlogs { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_prefetch() {
        let command = Commands::Acquire {
            artifact: Some(Prefetch { alt_dir: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_alts() {
        let command = Commands::Acquire {
            artifact: Some(Services { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Shimcache { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Shimdb { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Recyclebin { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Users { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Tasks { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Amcache { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_shellbags() {
        let command = Commands::Acquire {
            artifact: Some(Shellbags {
                resolve_guids: false,
                alt_file: None,
            }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_srum() {
        let command = Commands::Acquire {
            artifact: Some(Srum { alt_file: None }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_bits() {
        let command = Commands::Acquire {
            artifact: Some(Bits {
                carve: false,
                alt_file: None,
            }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_rawfiles() {
        let command = Commands::Acquire {
            artifact: Some(Rawfilelisting {
                drive_letter: 'C',
                start_path: String::from("C:\\"),
                depth: 1,
                recover_indx: false,
                md5: false,
                sha1: false,
                sha256: false,
                metadata: false,
                path_regex: None,
                filename_regex: None,
            }),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_setup_artifact() {
        let result = setup_artifact(&Jumplists { alt_file: None });
        assert_eq!(result.artifact_name, "jumplists");
    }
}
