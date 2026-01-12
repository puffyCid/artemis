use crate::{
    artifacts::os::{systeminfo::info::get_platform, triage::error::TriageError},
    filesystem::{acquire::acquire_file, files::get_filename},
    structs::{
        artifacts::os::triage::{TriageOptions, TriageTargets},
        toml::Output,
    },
    utils::regex_options::{create_regex, regex_check},
};
use log::{error, warn};
use std::{
    fs::{read_dir, remove_dir},
    io,
};
use walkdir::{DirEntry, WalkDir};

pub(crate) fn collect_triage(
    options: &TriageOptions,
    output: &mut Output,
) -> Result<(), TriageError> {
    for target in &options.targets {
        collect_targets(options, output, target)?;
    }
    if let Err(err) = cleanup(&output.directory) {
        panic!(
            "[triage] Could not cleanup acquisition data at {}: {err:?}",
            output.directory
        );
    }
    Ok(())
}

fn collect_targets(
    options: &TriageOptions,
    output: &mut Output,
    target: &TriageTargets,
) -> Result<(), TriageError> {
    let mut start_walk = WalkDir::new(&target.path).same_file_system(false);
    if target.recursive {
        start_walk = start_walk.max_depth(255);
    }

    for entries in start_walk.into_iter() {
        let entry = match entries {
            Ok(result) => result,
            Err(err) => {
                warn!("[triage] Failed to get file info: {err:?}");
                continue;
            }
        };

        if !entry.file_type().is_file() {
            continue;
        }

        // Copy all files no filtering
        if mask_check(&entry, &target.file_mask) {
            copy_file(options, output, &entry)?;
        }
    }

    Ok(())
}

fn mask_check(entry: &DirEntry, mask: &str) -> bool {
    if mask == "*" {
        return true;
    }
    if mask.starts_with("regex:") {
        let value = match create_regex(&mask.replace("regex:", "")) {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[triage] Bad regex mask {mask}: {err:?}. Skipping file {:?}",
                    entry.path()
                );
                return false;
            }
        };

        return regex_check(&value, &entry.path().display().to_string());
    }

    // All other masks are just globs '*.pf'
    let value = match create_regex(&mask.replace("*", ".*")) {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[triage] Bad glob mask {mask}: {err:?}. Skipping file {:?}",
                entry.path()
            );
            return false;
        }
    };

    regex_check(&value, &entry.path().display().to_string())
}

fn copy_file(
    options: &TriageOptions,
    output: &mut Output,
    file: &DirEntry,
) -> Result<(), TriageError> {
    println!("acquire the file: {}", file.path().display().to_string());

    let mut out = output.clone();
    let path = file.path().display().to_string();
    let filename = get_filename(&path);
    if options.recreate_directories {
        out.name = if get_platform().to_lowercase() == "windows" {
            format!(
                "{}\\{}",
                out.name,
                path.replace(":", "")
                    .replace(&filename, "")
                    .replace("/", "\\")
            )
        } else {
            format!("{}/{}", out.name, path.replace(&filename, ""))
        };
    }

    if out.output.to_lowercase() == "local" {
        if let Err(err) = acquire_file(&path, out) {
            error!("[triage] Failed to acquire '{path}: {err:?}");
            return Err(TriageError::CopyFile);
        }
    }

    Ok(())
}

fn cleanup(path: &str) -> io::Result<()> {
    for entry in read_dir(path)? {
        let value = entry?;
        if !value.file_type()?.is_dir() {
            continue;
        }
        cleanup(&value.path().display().to_string())?;

        let _ = remove_dir(value.path());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::triage::collect::collect_triage,
        structs::{
            artifacts::os::triage::{TriageOptions, TriageTargets},
            toml::Output,
        },
    };
    use std::path::PathBuf;

    fn output_options(
        name: &str,
        output: &str,
        directory: &str,
        compress: bool,
        port: u16,
        key: String,
    ) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(format!(
                "http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
            )),
            api_key: Some(key),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_triage() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\dfir");

        let mut options: TriageOptions = TriageOptions {
            description: String::from("A rust test"),
            author: String::from("MEEEEE!"),
            version: 1.0,
            id: String::from("my-fake-uuid"),
            recreate_directories: true,
            targets: Vec::new(),
        };
        let mut target = TriageTargets::default();
        target.path = test_location.display().to_string();
        target.recursive = true;
        options.targets.push(target);

        let mut out = output_options(
            "triage_collection_test",
            "local",
            "./tmp",
            false,
            0,
            String::from(""),
        );
        collect_triage(&options, &mut out).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_triage_prefetch() {
        use crate::filesystem::files::read_file;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\triage\\Prefetch.toml");
        let bytes = read_file(test_location.to_str().unwrap()).unwrap();
        let triage: TriageOptions = toml::from_slice(&bytes).unwrap();

        let mut out = output_options(
            "triage_collection_prefetch",
            "local",
            "./tmp",
            false,
            0,
            String::from(""),
        );
        collect_triage(&triage, &mut out).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_triage_opera() {
        use crate::filesystem::files::read_file;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\triage\\Opera.toml");
        let bytes = read_file(test_location.to_str().unwrap()).unwrap();
        let triage: TriageOptions = toml::from_slice(&bytes).unwrap();

        let mut out = output_options(
            "triage_collection_opera",
            "local",
            "./tmp",
            false,
            0,
            String::from(""),
        );
        collect_triage(&triage, &mut out).unwrap();
    }
}
