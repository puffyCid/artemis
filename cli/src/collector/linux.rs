use super::commands::CommandArgs;
use artemis_core::{
    core::artemis_collection,
    structs::{
        artifacts::os::{files::FileOptions, processes::ProcessOptions},
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

/// Run the linux collector and parse specified artifacts
pub(crate) fn run_collector(command: &Commands, output: Output) {
    let mut collector = ArtemisToml {
        system: String::from("linux"),
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
                collector.output.format = format.to_string();
            }
        }
    }

    artemis_collection(&mut collector).unwrap();
}

/// Setup any artifact options. Only a few have options on linux
fn setup_artifact(artifact: &CommandArgs) -> Artifacts {
    let mut collect = Artifacts {
        artifact_name: String::new(),
        filter: None,
        processes: None,
        files: None,
        script: None,
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
        CommandArgs::Cron {} => collect.artifact_name = String::from("cron"),
        CommandArgs::Journals {} => collect.artifact_name = String::from("journal"),
        CommandArgs::Logons {} => collect.artifact_name = String::from("logon"),
        CommandArgs::Sudologs {} => collect.artifact_name = String::from("sudologs"),
        CommandArgs::Shellhistory {} => collect.artifact_name = String::from("shell_history"),
        CommandArgs::Systeminfo {} => collect.artifact_name = String::from("systeminfo"),
    }
    collect
}

#[cfg(test)]
mod tests {
    use super::{run_collector, setup_artifact, Commands};
    use crate::collector::linux::CommandArgs::{
        Chromiumdownloads, Chromiumhistory, Cron, Filelisting, Firefoxdownloads, Firefoxhistory,
        Journals, Logons, Processes, Shellhistory, Sudologs, Systeminfo,
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
    fn test_run_collector_others() {
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
            artifact: Some(Logons {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Journals {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Sudologs {}),
            format: String::from("json"),
        };

        let out = output();
        run_collector(&command, out);

        let command = Commands::Acquire {
            artifact: Some(Cron {}),
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
    fn test_setup_artifact() {
        let result = setup_artifact(&Shellhistory {});
        assert_eq!(result.artifact_name, "shell_history");
    }
}
