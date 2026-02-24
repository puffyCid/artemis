use crate::{
    artifacts::os::{
        systeminfo::info::{PlatformType, get_platform_enum},
        triage::{error::TriageError, reader::TriageReader},
    },
    filesystem::{
        files::get_filename,
        metadata::{GlobInfo, get_metadata, get_timestamps, glob_paths},
        ntfs::{raw_files::raw_reader, setup::setup_ntfs_parser},
    },
    structs::{
        artifacts::triage::{TriageOptions, TriageTargets},
        toml::Output,
    },
    utils::regex_options::{create_regex, regex_check},
};
use log::{error, warn};
use regex::Regex;
use serde::Serialize;
use std::{
    fs::{File, create_dir_all},
    io::BufReader,
};
use walkdir::WalkDir;
use zip::ZipWriter;

/// Triage a system by acquiring files
pub(crate) fn triage(output: &mut Output, options: &TriageOptions) -> Result<(), TriageError> {
    for target in &options.triage {
        acquire_files(target, output)?;
    }
    Ok(())
}

#[derive(Serialize, Default)]
struct TriageReport {
    created: String,
    modified: String,
    accessed: String,
    changed: String,
    full_path: String,
    filename: String,
    md5: String,
    size: u64,
}

/// Copy the targeted files
fn acquire_files(target: &TriageTargets, output: &mut Output) -> Result<(), TriageError> {
    // Combine path with file mask. Most often file mask is a simple glob
    let mut glob_string = format!("{}{}", target.path, target.file_mask);
    // If we are traversing the file system. Then apply the file mask as we traverse
    if target.recursive {
        glob_string = target.path.clone();
    }
    let mut file_pattern = None;
    // Check if file mask is using regex instead a glob
    if target.file_mask.starts_with("regex:") {
        glob_string = target.path.clone();
        let pattern = match create_regex(&target.file_mask.replace("regex:", "")) {
            Ok(result) => result,
            Err(err) => {
                error!("[triage] Could not create regex: {err:?}");
                return Err(TriageError::Regex);
            }
        };
        file_pattern = Some(pattern);
    }

    let paths = glob_paths(&glob_string).unwrap_or_default();
    let zip_output = format!("{}/{}", output.directory, output.name);
    if let Err(err) = create_dir_all(&zip_output) {
        error!("[triage] Could not create output directory: {err:?}");
        return Err(TriageError::Output);
    }
    let zip_file = match File::create(format!("{zip_output}/files.zip")) {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Could not create zip file: {err:?}");
            return Err(TriageError::Output);
        }
    };
    let zip = ZipWriter::new(zip_file);

    let mut acq = TriageReader {
        fs: None,
        zip,
        path: String::new(),
    };

    let mut report = Vec::new();
    for path in paths {
        if target.recursive {
            walk_filesystem(
                &path,
                file_pattern.as_ref(),
                &mut acq,
                &mut report,
                target.recreate_directories,
                &target.file_mask,
            )?;
            continue;
        }

        if !path.is_file {
            continue;
        }

        // If regex is being used. Then check if our filename matches
        if file_pattern
            .as_ref()
            .is_some_and(|pat| !regex_check(pat, &path.filename))
        {
            continue;
        }
        let file_report = read_file(&path.full_path, &mut acq, target.recreate_directories)?;
        report.push(file_report);
    }

    output.output_count += report.len() as u64;
    let mut bytes = serde_json::to_vec(&report).unwrap_or_default();
    acq.write_report(&mut bytes)?;

    if let Err(err) = acq.zip.finish() {
        warn!("[triage] Failed to finish zipping file: {err:?}");
    }

    Ok(())
}

/// Transverse the filesystem and acquire all files that match the provided glob or regex
fn walk_filesystem(
    glob_path: &GlobInfo,
    pattern: Option<&Regex>,
    acq: &mut TriageReader<File, File>,
    report: &mut Vec<TriageReport>,
    create_paths: bool,
    file_mask: &str,
) -> Result<(), TriageError> {
    let start_walk = WalkDir::new(&glob_path.full_path).same_file_system(false);
    for entries in start_walk {
        let entry = match entries {
            Ok(result) => result,
            Err(err) => {
                error!("[triage] Failed to walk directory: {err:?}");
                continue;
            }
        };

        // No regex was provided. Using file mask to determine if a file should be read
        if pattern.is_none() && entry.path().is_dir() {
            let file_mask_path = entry.path().join(file_mask);
            let glob_paths = match glob_paths(file_mask_path.to_str().unwrap_or_default()) {
                Ok(result) => result,
                Err(err) => {
                    error!("[triage] Failed to glob walk directory: {err:?}");
                    continue;
                }
            };

            for glob_path in glob_paths {
                if !glob_path.is_file {
                    continue;
                }

                let file_report = read_file(&glob_path.full_path, acq, create_paths)?;
                report.push(file_report);
            }
            continue;
        }

        // If we are not using regex then only acquire files that match the file mask (the glob above)
        if pattern.is_none() && entry.path().is_file() {
            continue;
        }

        // Applying Regex patterns. First make sure we are at a file
        if !entry.path().is_file() {
            continue;
        }

        // If regex is being used. Then check if our filename matches
        if pattern
            .is_some_and(|pat| !regex_check(pat, entry.file_name().to_str().unwrap_or_default()))
        {
            continue;
        }
        let path = entry.path().to_str().unwrap_or_default();
        let file_report = read_file(path, acq, create_paths)?;
        report.push(file_report);
    }

    Ok(())
}

/// Read the target file that matched the glob or regex
fn read_file(
    path: &str,
    acq: &mut TriageReader<File, File>,
    create_paths: bool,
) -> Result<TriageReport, TriageError> {
    let reader = match File::open(path) {
        Ok(result) => result,
        Err(err) => {
            if get_platform_enum() == PlatformType::Windows {
                return read_file_ntfs(path, acq, create_paths);
            }

            error!("[triage] Could not read file {path}: {err:?}");
            return Err(TriageError::ReadFile);
        }
    };
    let buf = BufReader::new(reader);
    let mut file_report = TriageReport {
        filename: get_filename(path),
        full_path: path.to_string(),
        ..Default::default()
    };

    if let Ok(meta) = get_metadata(path)
        && let Ok(time) = get_timestamps(path)
    {
        file_report.size = meta.len();
        file_report.created = time.created;
        file_report.accessed = time.accessed;
        file_report.changed = time.changed;
        file_report.modified = time.modified;
    }

    acq.fs = Some(buf);
    acq.path = path.to_string();

    // If the user does not want to preserve full paths just save the filename
    if !create_paths {
        acq.path = get_filename(path);
    }
    let hash = acq.acquire_file()?;
    file_report.md5 = hash;

    Ok(file_report)
}

/// Read the target file that matched the glob or regex by parsing the NTFS
fn read_file_ntfs(
    path: &str,
    acq: &mut TriageReader<File, File>,
    create_paths: bool,
) -> Result<TriageReport, TriageError> {
    // Check if we want to acquire an ADS attribute. Those are easier to read
    if path.contains(":$") {
        let ads_path: Vec<&str> = path.split(":$").collect();
        acq.path = ads_path[0].to_string();
        let attribute = format!("${}", ads_path[1]);
        let hash = acq.acquire_file_ntfs_ads(&attribute)?;

        let mut file_report = TriageReport {
            filename: attribute,
            full_path: path.to_string(),
            md5: hash,
            ..Default::default()
        };

        if let Ok(meta) = get_metadata(path)
            && let Ok(time) = get_timestamps(path)
        {
            file_report.size = meta.len();
            file_report.created = time.created;
            file_report.accessed = time.accessed;
            file_report.changed = time.changed;
            file_report.modified = time.modified;
        }

        // If the user does not want to preserve full paths just save the filename
        if !create_paths {
            acq.path = get_filename(path);
        }

        return Ok(file_report);
    }
    // On Windows use a NTFS reader
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Could not setup NTFS parser: {err:?}");
            return Err(TriageError::ReadFile);
        }
    };

    let reader_result = raw_reader(path, &ntfs_parser.ntfs, &mut ntfs_parser.fs);
    let ntfs_file = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Could not setup reader: {err:?}");
            return Err(TriageError::ReadFile);
        }
    };
    acq.path = path.to_string();

    let hash = match acq.acquire_file_ntfs(&ntfs_file, &mut ntfs_parser.fs) {
        Ok(result) => result,
        Err(err) => {
            error!("[triage] Could not acquire raw file: {err:?}");
            return Err(TriageError::ReadFile);
        }
    };

    let mut file_report = TriageReport {
        filename: get_filename(path),
        full_path: path.to_string(),
        md5: hash,
        ..Default::default()
    };

    if let Ok(meta) = get_metadata(path)
        && let Ok(time) = get_timestamps(path)
    {
        file_report.size = meta.len();
        file_report.created = time.created;
        file_report.accessed = time.accessed;
        file_report.changed = time.changed;
        file_report.modified = time.modified;
    }

    // If the user does not want to preserve full paths just save the filename
    if !create_paths {
        acq.path = get_filename(path);
    }

    Ok(file_report)
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::triage::{
            artifact::{acquire_files, read_file, triage, walk_filesystem},
            reader::TriageReader,
        },
        filesystem::metadata::GlobInfo,
        structs::{
            artifacts::triage::{TriageOptions, TriageTargets},
            toml::Output,
        },
        utils::regex_options::create_regex,
    };
    use std::{
        fs::{File, create_dir_all},
        path::PathBuf,
    };
    use zip::ZipWriter;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_triage() {
        let mut output = output_options("triage_test", "local", "./tmp", false);
        let options = TriageOptions {
            triage: vec![TriageTargets {
                name: String::from("Linux Journal files"),
                path: String::from("/var/log/journal/"),
                file_mask: String::from("*user*"),
                recursive: true,
                recreate_directories: true,
            }],
        };

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_triage_linux() {
        let mut output = output_options("triage_test", "local", "./tmp", false);
        let options = TriageOptions {
            triage: vec![TriageTargets {
                name: String::from("Linux Journal files"),
                path: String::from("/var/log/journal/"),
                file_mask: String::from("*user*"),
                recursive: false,
                recreate_directories: true,
            }],
        };

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_triage_linux_recursive() {
        let mut output = output_options("triage_test_recursive", "local", "./tmp", false);
        let options = TriageOptions {
            triage: vec![TriageTargets {
                name: String::from("Linux Journal files"),
                path: String::from("/var/log/journal/"),
                file_mask: String::from("*user*"),
                recursive: true,
                recreate_directories: false,
            }],
        };

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_acquire_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/");

        let target = TriageTargets {
            name: String::from("test"),
            recursive: false,
            file_mask: String::from("*.toml"),
            path: test_location.display().to_string(),
            recreate_directories: false,
        };

        let mut out = output_options("acquire_files", "local", "./tmp", false);
        acquire_files(&target, &mut out).unwrap();
    }

    #[test]
    fn test_walk_filesystem() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/triage/malformed");
        let glob_path = GlobInfo {
            full_path: test_location.display().to_string(),
            filename: String::new(),
            is_file: true,
            is_directory: false,
            is_symlink: false,
        };
        let out = output_options("walk_filesystem", "local", "./tmp", false);

        let zip_output = format!("{}/{}", out.directory, out.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{zip_output}/files.zip")).unwrap();

        let zip = ZipWriter::new(zip_file);
        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };
        let mut report = Vec::new();

        walk_filesystem(&glob_path, None, &mut acq, &mut report, true, "bad.toml").unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].md5, "695aacf9c82357da7564cb875604fd62");
    }

    #[test]
    fn test_walk_filesystem_regex() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/triage/malformed");
        let glob_path = GlobInfo {
            full_path: test_location.display().to_string(),
            filename: String::new(),
            is_file: true,
            is_directory: false,
            is_symlink: false,
        };
        let out = output_options("walk_filesystem", "local", "./tmp", false);

        let zip_output = format!("{}/{}", out.directory, out.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{zip_output}/files.zip")).unwrap();

        let zip = ZipWriter::new(zip_file);
        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };
        let mut report = Vec::new();
        let patter = create_regex("bad.*").unwrap();

        walk_filesystem(&glob_path, Some(&patter), &mut acq, &mut report, true, "").unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].md5, "695aacf9c82357da7564cb875604fd62");
    }

    #[test]
    fn test_read_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/files.toml");

        let out = output_options("read_file", "local", "./tmp", false);

        let zip_output = format!("{}/{}", out.directory, out.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{zip_output}/files.zip")).unwrap();

        let zip = ZipWriter::new(zip_file);
        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };

        let report = read_file(test_location.to_str().unwrap(), &mut acq, true).unwrap();
        assert_eq!(report.md5, "cbed8a94f6a32edc5266206f83985386");
        assert_eq!(report.size, 606);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs() {
        use crate::artifacts::os::triage::artifact::read_file_ntfs;

        let out = output_options("read_file_ntfs", "local", "./tmp", false);
        let path = "C:\\Windows\\System32\\config\\SOFTWARE";

        let zip_output = format!("{}/{}", out.directory, out.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{zip_output}/files.zip")).unwrap();
        let zip = ZipWriter::new(zip_file);

        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };

        let report = read_file_ntfs(path, &mut acq, true).unwrap();
        assert!(!report.md5.is_empty())
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs_ads() {
        use crate::artifacts::os::triage::artifact::read_file_ntfs;

        let out = output_options("read_file_ntfs_ads", "local", "./tmp", false);
        let path = "C:\\$Secure:$SDS";

        let zip_output = format!("{}/{}", out.directory, out.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{zip_output}/files.zip")).unwrap();
        let zip = ZipWriter::new(zip_file);

        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };

        let report = read_file_ntfs(path, &mut acq, true).unwrap();
        assert!(!report.md5.is_empty());
    }
}
