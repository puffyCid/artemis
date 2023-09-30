use super::artifacts::Options;
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
                UsnJrnlOptions,
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
        /// Collect processes
        #[arg(long)]
        processes: bool,
        /// Pull filelisting
        #[arg(long)]
        files: bool,
        /// Parse Prefetch
        #[arg(long)]
        prefetch: bool,
        /// Parse EventLogs
        #[arg(long)]
        eventlogs: bool,
        /// Parse ShimDatabase
        #[arg(long)]
        shimdb: bool,
        /// Parse Registry
        #[arg(long)]
        registry: bool,
        /// Parse Userassist
        #[arg(long)]
        userassist: bool,
        /// Parse Users
        #[arg(long)]
        users: bool,
        /// Parse Shimcache
        #[arg(long)]
        shimcache: bool,
        /// Get systeminfo
        #[arg(long)]
        systeminfo: bool,
        /// Parse Shortcuts. Must provide target directory
        #[arg(long, default_value_t = String::new())]
        shortcuts: String,
        /// Parse Shellbags
        #[arg(long)]
        shellbags: bool,
        /// Parse Amcache
        #[arg(long)]
        amcache: bool,
        /// Parse Firefox History and Downloads
        #[arg(long)]
        firefox: bool,
        /// Parse Chromium History and Downloads
        #[arg(long)]
        chromium: bool,
        /// Parse UsnJrnl
        #[arg(long)]
        usnjrnl: bool,
        /// Parse BITS
        #[arg(long)]
        bits: bool,
        /// Parse SRUM
        #[arg(long)]
        srum: bool,
        /// Parse Windows Search
        #[arg(long)]
        search: bool,
        /// Parse Windows Tasks
        #[arg(long)]
        tasks: bool,
        /// Parse Windows Services
        #[arg(long)]
        services: bool,
        /// Parse Jumplists
        #[arg(long)]
        jumplists: bool,
        /// Parse RecycleBin
        #[arg(long)]
        recyclebin: bool,
        /// Parse NTFS to get filelisting
        #[arg(long)]
        rawfiles: bool,
        /// Output format. JSON or JSON.
        #[arg(long, default_value_t = String::from("json"))]
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
        Commands::Acquire {
            processes,
            files,
            format,
            prefetch,
            eventlogs,
            shimdb,
            registry,
            userassist,
            users,
            shimcache,
            systeminfo,
            shellbags,
            amcache,
            firefox,
            chromium,
            usnjrnl,
            bits,
            srum,
            search,
            tasks,
            services,
            jumplists,
            recyclebin,
            shortcuts,
            rawfiles,
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
                    .push(setup_artifact(Options::Users, "users"))
            }
            if *prefetch {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Prefetch, "prefetch"))
            }
            if *eventlogs {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Eventlogs, "eventlogs"))
            }
            if *shimdb {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Shimdb, "shimdb"))
            }
            if *registry {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Registry, "registry"))
            }
            if *systeminfo {
                collector
                    .artifacts
                    .push(setup_artifact(Options::None, "systeminfo"))
            }
            if !shortcuts.is_empty() {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Shortcuts, shortcuts))
            }
            if *userassist {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Userassist, "userassist"))
            }
            if *shimcache {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Shimcache, "shimcache"))
            }
            if *shellbags {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Shellbags, "shellbags"))
            }
            if *amcache {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Amcache, "amcache"))
            }
            if *usnjrnl {
                collector
                    .artifacts
                    .push(setup_artifact(Options::UsnJrnl, "usnjrnl"))
            }
            if *bits {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Bits, "bits"))
            }
            if *srum {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Srum, "srum"))
            }
            if *search {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Search, "search"))
            }
            if *tasks {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Tasks, "tasks"))
            }
            if *services {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Services, "services"))
            }
            if *jumplists {
                collector
                    .artifacts
                    .push(setup_artifact(Options::Jumplists, "jumplists"))
            }
            if *recyclebin {
                collector
                    .artifacts
                    .push(setup_artifact(Options::RecycleBin, "recyclebin"))
            }
            if *rawfiles {
                collector
                    .artifacts
                    .push(setup_artifact(Options::RawFiles, "rawfiles"))
            }
            if !format.is_empty() {
                collector.output.format = format.to_string();
            }
        }
    }
    artemis_collection(&mut collector).unwrap();
}

/// Setup any artifact options
fn setup_artifact(artifact: Options, info: &str) -> Artifacts {
    let name = if artifact == Options::Shortcuts {
        "shortcuts"
    } else {
        info
    };
    let mut collect = Artifacts {
        artifact_name: name.to_string(),
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
        Options::Registry => {
            let options = RegistryOptions {
                user_hives: true,
                system_hives: true,
                path_regex: None,
                alt_drive: None,
            };
            collect.registry = Some(options);
        }
        Options::Bits => {
            let options = BitsOptions {
                carve: true,
                alt_path: None,
            };
            collect.bits = Some(options);
        }
        Options::Shellbags => {
            let options = ShellbagsOptions {
                resolve_guids: true,
                alt_drive: None,
            };
            collect.shellbags = Some(options);
        }
        Options::Shortcuts => {
            let options = ShortcutOptions {
                path: info.to_string(),
            };
            collect.shortcuts = Some(options);
        }
        Options::Srum => {
            let options = SrumOptions { alt_path: None };
            collect.srum = Some(options);
        }
        Options::Eventlogs => {
            let options = EventLogsOptions { alt_drive: None };
            collect.eventlogs = Some(options);
        }
        Options::Prefetch => {
            let options = PrefetchOptions { alt_drive: None };
            collect.prefetch = Some(options);
        }
        Options::Shimdb => {
            let options = ShimdbOptions { alt_drive: None };
            collect.shimdb = Some(options);
        }
        Options::Shimcache => {
            let options = ShimcacheOptions { alt_drive: None };
            collect.shimcache = Some(options);
        }
        Options::Amcache => {
            let options = AmcacheOptions { alt_drive: None };
            collect.amcache = Some(options);
        }
        Options::UsnJrnl => {
            let options = UsnJrnlOptions { alt_drive: None };
            collect.usnjrnl = Some(options);
        }
        Options::Users => {
            let options = UserOptions { alt_drive: None };
            collect.users = Some(options);
        }
        Options::Search => {
            let options = SearchOptions { alt_path: None };
            collect.search = Some(options);
        }
        Options::Tasks => {
            let options = TasksOptions { alt_drive: None };
            collect.tasks = Some(options);
        }
        Options::Services => {
            let options = ServicesOptions { alt_drive: None };
            collect.services = Some(options);
        }
        Options::Jumplists => {
            let options = JumplistsOptions { alt_drive: None };
            collect.jumplists = Some(options);
        }
        Options::RecycleBin => {
            let options = RecycleBinOptions { alt_drive: None };
            collect.recyclebin = Some(options);
        }
        Options::Userassist => {
            let options = UserAssistOptions { alt_drive: None };
            collect.userassist = Some(options);
        }
        Options::RawFiles => {
            let options = RawFilesOptions {
                start_path: String::from("C:\\"),
                drive_letter: 'C',
                depth: 100,
                recover_indx: true,
                md5: Some(true),
                sha1: None,
                sha256: None,
                metadata: None,
                path_regex: None,
                filename_regex: None,
            };
            collect.rawfiles = Some(options);
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
            users: false,
            systeminfo: true,
            firefox: true,
            chromium: true,
            format: String::from("json"),
            prefetch: true,
            eventlogs: true,
            shimdb: true,
            registry: false,
            userassist: false,
            shimcache: false,
            shortcuts: String::from(
                "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs\\Startup",
            ),
            shellbags: false,
            amcache: false,
            usnjrnl: false,
            bits: true,
            srum: true,
            search: true,
            tasks: true,
            services: true,
            jumplists: true,
            recyclebin: true,
            rawfiles: false,
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
    fn test_run_collector_registry() {
        let command = Commands::Acquire {
            processes: false,
            files: false,
            users: false,
            systeminfo: false,
            firefox: false,
            chromium: false,
            format: String::from("json"),
            prefetch: false,
            eventlogs: false,
            shimdb: false,
            registry: true,
            userassist: true,
            shimcache: true,
            shortcuts: String::new(),
            shellbags: true,
            amcache: true,
            usnjrnl: false,
            bits: false,
            srum: false,
            search: false,
            tasks: false,
            services: false,
            jumplists: false,
            recyclebin: false,
            rawfiles: false,
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
    fn test_setup_artifact() {
        let result = setup_artifact(Options::Users, "users");
        assert_eq!(result.artifact_name, "users");
    }
}
