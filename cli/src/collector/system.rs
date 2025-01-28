use super::commands::CommandArgs;
use clap::{arg, Subcommand};
use core::{
    core::artemis_collection,
    structs::{
        artifacts::os::{
            files::FileOptions,
            linux::{JournalOptions, LinuxSudoOptions, LogonOptions},
            macos::{
                EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions,
                LoginitemsOptions, MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions,
                SpotlightOptions, UnifiedLogsOptions,
            },
            processes::ProcessOptions,
            windows::{
                AmcacheOptions, BitsOptions, EventLogsOptions, JumplistsOptions, MftOptions,
                OutlookOptions, PrefetchOptions, RawFilesOptions, RecycleBinOptions,
                RegistryOptions, SearchOptions, ServicesOptions, ShellbagsOptions,
                ShimcacheOptions, ShimdbOptions, ShortcutOptions, SrumOptions, TasksOptions,
                UserAssistOptions, UsnJrnlOptions, WindowsUserOptions, WmiPersistOptions,
            },
        },
        toml::{ArtemisToml, Artifacts, Output},
    },
};

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Acquire forensic artifacts
    Acquire {
        #[command(subcommand)]
        artifact: Option<CommandArgs>,
        /// Output format. JSON or JSONL or CSV.
        #[arg(long, default_value_t = String::from("JSON"))]
        format: String,
        /// Optional output directory for storing results
        #[arg(long, default_value_t = String::from("./tmp"))]
        output_dir: String,
        /// GZIP Compress results
        #[arg(long)]
        compress: bool,
    },
}

/// Run the collector and parse specified artifacts
pub(crate) fn run_collector(command: &Commands, output: Output) {
    #[cfg(target_os = "macos")]
    let system = String::from("macos");
    #[cfg(target_os = "linux")]
    let system = String::from("linux");
    #[cfg(target_os = "windows")]
    let system = String::from("windows");

    let mut collector = ArtemisToml {
        system,
        output,
        artifacts: Vec::new(),
    };
    match command {
        Commands::Acquire {
            artifact,
            format,
            output_dir,
            compress,
        } => {
            if artifact.is_none() {
                println!("No artifact provided");
                return;
            }

            let arti = artifact.as_ref().unwrap();
            collector.artifacts.push(setup_artifact(arti));
            collector.output.compress = *compress;

            if !format.is_empty() {
                collector.output.format = format.to_string().to_lowercase();
            }
            if !output_dir.is_empty() {
                collector.output.directory = output_dir.to_string();
            }

            println!(
                "[artemis] Writing output to: {}",
                collector.output.directory
            );
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
        unifiedlogs: None,
        script: None,
        emond: None,
        execpolicy: None,
        loginitems: None,
        launchd: None,
        fseventsd: None,
        users_macos: None,
        groups_macos: None,
        sudologs_macos: None,
        spotlight: None,
        journals: None,
        sudologs_linux: None,
        logons: None,
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
        users_windows: None,
        search: None,
        tasks: None,
        services: None,
        jumplists: None,
        recyclebin: None,
        wmipersist: None,
        outlook: None,
        mft: None,
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
            yara_rule,
        } => {
            let options = FileOptions {
                md5: Some(*md5),
                start_path: start_path.to_string(),
                depth: Some(*depth),
                metadata: Some(*metadata),
                sha1: Some(*sha1),
                sha256: Some(*sha256),
                regex_filter: regex_filter.clone(),
                yara: yara_rule.clone(),
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
        CommandArgs::Cron {} => collect.artifact_name = String::from("cron"),
        CommandArgs::Shellhistory {} => collect.artifact_name = String::from("shell_history"),
        CommandArgs::Systeminfo {} => collect.artifact_name = String::from("systeminfo"),
        CommandArgs::Emond { alt_path } => {
            let options = EmondOptions {
                alt_path: alt_path.clone(),
            };
            collect.emond = Some(options);
            collect.artifact_name = String::from("emond");
        }
        CommandArgs::Fsevents { alt_file } => {
            let options = FseventsOptions {
                alt_file: alt_file.clone(),
            };
            collect.fseventsd = Some(options);
            collect.artifact_name = String::from("fseventsd");
        }
        CommandArgs::Execpolicy { alt_file } => {
            let options = ExecPolicyOptions {
                alt_file: alt_file.clone(),
            };
            collect.execpolicy = Some(options);
            collect.artifact_name = String::from("execpolicy");
        }
        CommandArgs::GroupsMacos { alt_path } => {
            let options = MacosGroupsOptions {
                alt_path: alt_path.clone(),
            };
            collect.groups_macos = Some(options);
            collect.artifact_name = String::from("groups-macos");
        }
        CommandArgs::Launchd { alt_file } => {
            let options = LaunchdOptions {
                alt_file: alt_file.clone(),
            };
            collect.launchd = Some(options);
            collect.artifact_name = String::from("launchd");
        }
        CommandArgs::Loginitems { alt_file } => {
            let options = LoginitemsOptions {
                alt_file: alt_file.clone(),
            };
            collect.loginitems = Some(options);
            collect.artifact_name = String::from("loginitems");
        }
        CommandArgs::SafariDownloads {} => collect.artifact_name = String::from("safari-downloads"),
        CommandArgs::SafariHistory {} => collect.artifact_name = String::from("safari-history"),
        CommandArgs::UsersMacos { alt_path } => {
            let options = MacosUsersOptions {
                alt_path: alt_path.clone(),
            };
            collect.users_macos = Some(options);
            collect.artifact_name = String::from("users-macos");
        }
        CommandArgs::SudologsMacos { logarchive_path } => {
            let options = MacosSudoOptions {
                logarchive_path: logarchive_path.clone(),
            };
            collect.sudologs_macos = Some(options);
            collect.artifact_name = String::from("sudologs-macos");
        }
        CommandArgs::Unifiedlogs {
            sources,
            logarchive_path,
        } => {
            let options = UnifiedLogsOptions {
                sources: sources.clone(),
                logarchive_path: logarchive_path.clone(),
            };
            collect.unifiedlogs = Some(options);
            collect.artifact_name = String::from("unifiedlogs");
        }
        CommandArgs::Spotlight {
            alt_path,
            include_additional,
        } => {
            let options = SpotlightOptions {
                alt_path: alt_path.clone(),
                include_additional: Some(*include_additional),
            };
            collect.spotlight = Some(options);
            collect.artifact_name = String::from("spotlight");
        }
        CommandArgs::Journals { alt_path } => {
            let options = JournalOptions {
                alt_path: alt_path.clone(),
            };
            collect.journals = Some(options);
            collect.artifact_name = String::from("journal");
        }
        CommandArgs::Logons { alt_file } => {
            let options = LogonOptions {
                alt_file: alt_file.clone(),
            };
            collect.logons = Some(options);
            collect.artifact_name = String::from("logon");
        }
        CommandArgs::SudologsLinux { alt_path } => {
            let options = LinuxSudoOptions {
                alt_path: alt_path.clone(),
            };
            collect.sudologs_linux = Some(options);
            collect.artifact_name = String::from("sudologs-linux");
        }
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
        CommandArgs::Eventlogs {
            alt_file,
            include_templates,
            alt_dir,
            alt_template_file,
            dump_templates,
            only_templates,
        } => {
            let options = EventLogsOptions {
                alt_file: alt_file.clone(),
                alt_dir: alt_dir.clone(),
                alt_template_file: alt_template_file.clone(),
                include_templates: *include_templates,
                dump_templates: *dump_templates,
                only_templates: *only_templates,
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
        CommandArgs::UsersWindows { alt_file } => {
            let options = WindowsUserOptions {
                alt_file: alt_file.clone(),
            };
            collect.users_windows = Some(options);
            collect.artifact_name = String::from("users-windows");
        }
        CommandArgs::Usnjrnl {
            alt_drive,
            alt_path,
        } => {
            let options = UsnJrnlOptions {
                alt_drive: *alt_drive,
                alt_path: alt_path.clone(),
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
        CommandArgs::Outlook {
            alt_file,
            include_attachments,
            start_date,
            end_date,
            yara_rule_message,
            yara_rule_attachment,
        } => {
            let options = OutlookOptions {
                alt_file: alt_file.clone(),
                include_attachments: *include_attachments,
                start_date: start_date.clone(),
                end_date: end_date.clone(),
                yara_rule_message: yara_rule_message.clone(),
                yara_rule_attachment: yara_rule_attachment.clone(),
            };

            collect.outlook = Some(options);
            collect.artifact_name = String::from("outlook");
        }
        CommandArgs::Mft {
            alt_file,
            alt_drive,
        } => {
            let options = MftOptions {
                alt_drive: alt_drive.clone(),
                alt_file: alt_file.clone(),
            };
            collect.mft = Some(options);
            collect.artifact_name = String::from("mft");
        }
    }
    collect
}

#[cfg(test)]
mod tests {
    use super::{run_collector, setup_artifact, Commands};
    use crate::collector::system::CommandArgs::{
        Amcache, Bits, Chromiumdownloads, Chromiumhistory, Cron, Emond, Eventlogs, Execpolicy,
        Filelisting, Firefoxdownloads, Firefoxhistory, Fsevents, GroupsMacos, Journals, Jumplists,
        Launchd, Loginitems, Logons, Prefetch, Processes, Rawfilelisting, Recyclebin, Registry,
        SafariDownloads, SafariHistory, Services, Shellbags, Shellhistory, Shimcache, Shimdb,
        Spotlight, Srum, SudologsLinux, SudologsMacos, Systeminfo, Tasks, Unifiedlogs, UsersMacos,
        UsersWindows,
    };
    use core::structs::toml::Output;
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
            output_dir: String::from("./tmp"),
            compress: false,
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
                yara_rule: None,
            }),
            format: String::from("json"),
            compress: false,
            output_dir: String::from("./tmp"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_macos_collector_root() {
        let command = Commands::Acquire {
            artifact: Some(Chromiumdownloads {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Chromiumhistory {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Firefoxdownloads {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Firefoxhistory {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Launchd { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: true,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(UsersMacos { alt_path: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(SudologsMacos {
                logarchive_path: None,
            }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Cron {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Systeminfo {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(GroupsMacos { alt_path: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Execpolicy { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Shellhistory {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Fsevents { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Emond { alt_path: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(SafariDownloads {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(SafariHistory {}),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_logs() {
        let command = Commands::Acquire {
            artifact: Some(Unifiedlogs {
                sources: vec![String::from("Special")],
                logarchive_path: None,
            }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_spotlight() {
        let command = Commands::Acquire {
            artifact: Some(Spotlight {
                alt_path: None,
                include_additional: false,
            }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_setup_artifact() {
        let result = setup_artifact(&Loginitems { alt_file: None });
        assert_eq!(result.artifact_name, "loginitems");
    }

    #[test]
    fn test_run_linux_collector_others() {
        let command = Commands::Acquire {
            artifact: Some(Logons { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Journals {
                alt_path: Some(String::from(".")),
            }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(SudologsLinux { alt_path: None }),
            format: String::from("json"),
            compress: false,
            output_dir: String::from("./tmp"),
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
            compress: false,

            output_dir: String::from("./tmp"),
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_eventlogs() {
        let command = Commands::Acquire {
            artifact: Some(Eventlogs {
                alt_file: None,
                include_templates: false,
                dump_templates: false,
                alt_dir: None,
                alt_template_file: None,
                only_templates: false,
            }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_prefetch() {
        let command = Commands::Acquire {
            artifact: Some(Prefetch { alt_dir: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_alts() {
        let command = Commands::Acquire {
            artifact: Some(Services { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Shimcache { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Shimdb { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Recyclebin { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(UsersWindows { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Tasks { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Amcache { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
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
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_run_collector_srum() {
        let command = Commands::Acquire {
            artifact: Some(Srum { alt_file: None }),
            format: String::from("json"),
            output_dir: String::from("./tmp"),
            compress: false,
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
            output_dir: String::from("./tmp"),
            compress: false,
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
            output_dir: String::from("./tmp"),
            compress: false,
        };

        let out = output();
        run_collector(&command, out);
    }

    #[test]
    fn test_setup_artifact_windows() {
        let result = setup_artifact(&Jumplists { alt_file: None });
        assert_eq!(result.artifact_name, "jumplists");
    }
}
