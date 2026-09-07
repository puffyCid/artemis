use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, FileHandle, Timestamp},
        source::handle::SourceHandle,
        walk::WalkAccessor,
    },
    artifacts::os::{
        systeminfo::info::{PlatformType, get_platform_enum},
        triage::{
            error::TriageError,
            reader::{TriageReader, grab_file, write_report},
        },
    },
    filesystem::{
        files::get_filename,
        metadata::{GlobInfo, get_metadata, get_timestamps, glob_paths},
        ntfs::{raw_files::raw_reader, setup::setup_ntfs_parser},
    },
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::triage::TriageOptions,
    utils::regex_options::{create_regex, regex_check},
};
use glob::Pattern;
use regex::Regex;
use serde::Serialize;
use std::{
    fs::{File, create_dir_all},
    io::BufReader,
    path::PathBuf,
};
use tracing::{error, info, warn};
use walkdir::WalkDir;
use zip::ZipWriter;

pub(crate) fn triage(
    manager: &mut OutputManager,
    options: &Vec<TriageOptions>,
) -> Result<(), TriageError> {
    let full_path = manager.config.directory.join(&manager.config.name);
    let zip_output = full_path.to_str().unwrap_or_default();
    if let Err(err) = create_dir_all(zip_output) {
        error!("Could not create output directory: {err:?}");
        return Err(TriageError::Output);
    }
    let zip_file = match File::create(format!("{zip_output}/files.zip")) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not create zip file: {err:?}");
            return Err(TriageError::Output);
        }
    };
    let mut zip = ZipWriter::new(zip_file);

    let mut report = Vec::new();
    let mut accessor = Accessor::with_defaults();
    let source = match accessor.open_source("host:") {
        Ok(result) => result,
        Err(err) => {
            error!("Could not open host source: {err:?}");
            return Err(TriageError::NoReader);
        }
    };
    // Loop through all triage targets
    for target in options {
        acquire_files_v2(target, &mut report, &mut accessor, &source, &mut zip)?;
    }
    let mut bytes = serde_json::to_vec(&report).unwrap_or_default();
    write_report(&mut zip, &mut bytes)?;

    if let Err(err) = zip.finish() {
        warn!("Failed to finish zipping file: {err:?}");
    }

    let mut records = match serialize_records_to_stream(report) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not serialize triage report: {err:?}");
            return Err(TriageError::Output);
        }
    };
    let artifact_name = "triage";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Could not write triage report: {err:?}");
        return Err(TriageError::Output);
    }

    Ok(())
}

/// Triage a system by acquiring files
pub(crate) fn triage_old(
    manager: &mut OutputManager,
    options: &Vec<TriageOptions>,
) -> Result<(), TriageError> {
    let full_path = manager.config.directory.join(&manager.config.name);
    let zip_output = full_path.to_str().unwrap_or_default();
    if let Err(err) = create_dir_all(zip_output) {
        error!("Could not create output directory: {err:?}");
        return Err(TriageError::Output);
    }
    let zip_file = match File::create(format!("{zip_output}/files.zip")) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not create zip file: {err:?}");
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
    // Loop through all triage targets
    for target in options {
        acquire_files(target, &mut acq, &mut report)?;
    }
    let mut bytes = serde_json::to_vec(&report).unwrap_or_default();
    acq.write_report(&mut bytes)?;

    if let Err(err) = acq.zip.finish() {
        warn!("Failed to finish zipping file: {err:?}");
    }

    let mut records = match serialize_records_to_stream(report) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not serialize triage report: {err:?}");
            return Err(TriageError::Output);
        }
    };
    let artifact_name = "triage";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Could not write triage report: {err:?}");
        return Err(TriageError::Output);
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
fn acquire_files(
    target: &TriageOptions,
    acq: &mut TriageReader<File, File>,
    report: &mut Vec<TriageReport>,
) -> Result<(), TriageError> {
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
                error!("Could not create regex: {err:?}");
                return Err(TriageError::Regex);
            }
        };
        file_pattern = Some(pattern);
    }

    let paths = glob_paths(&glob_string).unwrap_or_default();

    for path in paths {
        if target.recursive {
            walk_filesystem(
                &path,
                file_pattern.as_ref(),
                acq,
                report,
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
        if let Ok(file_report) = read_file(&path.full_path, acq, target.recreate_directories) {
            report.push(file_report);
        }
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
                error!("Failed to walk directory: {err:?}");
                continue;
            }
        };

        // No regex was provided. Using file mask to determine if a file should be read
        if pattern.is_none() && entry.path().is_dir() {
            let file_mask_path = entry.path().join(file_mask);
            let glob_paths = match glob_paths(file_mask_path.to_str().unwrap_or_default()) {
                Ok(result) => result,
                Err(err) => {
                    error!("Failed to glob walk directory: {err:?}");
                    continue;
                }
            };

            for glob_path in glob_paths {
                if !glob_path.is_file {
                    continue;
                }
                if let Ok(file_report) = read_file(&glob_path.full_path, acq, create_paths) {
                    report.push(file_report);
                }
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
        if let Ok(file_report) = read_file(path, acq, create_paths) {
            report.push(file_report);
        }
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
            // If the file is locked, try reading raw NTFS
            if get_platform_enum() == PlatformType::Windows {
                return read_file_ntfs(path, acq, create_paths);
            }

            error!("Could not read file {path}: {err:?}");
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
        let zip_entry_path = get_ntfs_ads_zip_path(ads_path[0], &attribute, create_paths);
        let hash = acq.acquire_file_ntfs_ads(&zip_entry_path, &attribute)?;

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

        return Ok(file_report);
    }
    // On Windows use a NTFS reader
    let ntfs_parser_result = setup_ntfs_parser(path.chars().next().unwrap_or('C'));
    let mut ntfs_parser = match ntfs_parser_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not setup NTFS parser: {err:?}");
            return Err(TriageError::ReadFile);
        }
    };

    let reader_result = raw_reader(path, &ntfs_parser.ntfs, &mut ntfs_parser.fs);
    let ntfs_file = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not setup NTFS reader: {err:?}");
            return Err(TriageError::ReadFile);
        }
    };
    acq.path = path.to_string();

    let hash = match acq.acquire_file_ntfs(&ntfs_file, &mut ntfs_parser.fs) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not acquire raw file: {err:?}");
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

fn get_ntfs_ads_zip_path(path: &str, attribute: &str, create_paths: bool) -> String {
    let base_path = if create_paths {
        path.to_string()
    } else {
        get_filename(path)
    };
    format!("{base_path}_{attribute}")
}

fn acquire_files_v2(
    target: &TriageOptions,
    report: &mut Vec<TriageReport>,
    accessor: &mut Accessor,
    source: &SourceHandle,
    zip: &mut ZipWriter<File>,
) -> Result<(), TriageError> {
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
                error!("Could not create regex: {err:?}");
                return Err(TriageError::Regex);
            }
        };
        file_pattern = Some(pattern);
    }

    info!("Applying glob on '{glob_string}'");

    let paths = match accessor.globfs(&glob_string) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob '{glob_string}': {err:?}");
            return Err(TriageError::ReadFile);
        }
    };

    for path in paths {
        if let Some(handle) = path.handle.as_directory()
            && target.recursive
        {
            info!("Walking the directory: '{}'", handle.full_path());
            let walk = match WalkAccessor::new(source, &handle.full_path()) {
                Ok(results) => results,
                Err(err) => {
                    warn!("Could not start walk for {}: {err:?}", handle.full_path());
                    continue;
                }
            };

            walking(
                walk,
                file_pattern.as_ref(),
                report,
                &target.file_mask,
                zip,
                accessor,
                source,
            )?;
            continue;
        }

        let Some(handle) = path.handle.as_file() else {
            continue;
        };
        // If regex is being used. Then check if our filename matches
        if file_pattern
            .as_ref()
            .is_some_and(|pat| !regex_check(pat, &path.meta.filename))
        {
            continue;
        }

        if let Ok(file_report) = read_file_v2(handle, accessor, source, zip) {
            report.push(file_report);
        }
    }

    Ok(())
}

fn walking(
    mut walk: WalkAccessor,
    pattern: Option<&Regex>,
    report: &mut Vec<TriageReport>,
    file_mask: &str,
    zip: &mut ZipWriter<File>,
    accessor: &mut Accessor,
    source: &SourceHandle,
) -> Result<(), TriageError> {
    while let Some(value) = walk.next(accessor) {
        let entry = match value {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not walk: {err:?}");
                continue;
            }
        };

        // No regex was provided. Using file mask to determine if a file should be read
        if pattern.is_none()
            && entry.entry.is_file()
            && let Ok(glob_pattern) = Pattern::new(file_mask)
        {
            if !glob_pattern.matches(&entry.entry.meta.filename) {
                continue;
            }

            let Some(handle) = entry.entry.handle.as_file() else {
                continue;
            };

            if let Ok(file_report) = read_file_v2(handle, accessor, source, zip) {
                report.push(file_report);
            }

            continue;
        }

        // If we are not using regex then only acquire files that match the file mask (the glob above)
        if (pattern.is_none() && entry.entry.is_file()) || !entry.entry.is_file() {
            continue;
        }

        // If regex is being used. Then check if our filename matches
        if pattern.is_some_and(|pat| !regex_check(pat, &entry.entry.meta.filename)) {
            continue;
        }

        let Some(handle) = entry.entry.handle.as_file() else {
            continue;
        };

        if let Ok(file_report) = read_file_v2(handle, accessor, source, zip) {
            report.push(file_report);
        }
    }

    Ok(())
}

fn read_file_v2(
    handle: &FileHandle,
    accessor: &mut Accessor,
    source: &SourceHandle,
    zip: &mut ZipWriter<File>,
) -> Result<TriageReport, TriageError> {
    let mut reader = match accessor.open_reader_handle(handle) {
        Ok(result) => result,
        Err(err) => {
            error!("Could open reader for {}: {err:?}", handle.display_path());
            return Err(TriageError::ReadFile);
        }
    };

    let mut file_report = TriageReport {
        filename: handle.filename(),
        full_path: handle.full_path(),
        ..Default::default()
    };

    if let Ok(meta) = accessor.source_stat_handle(source, handle) {
        file_report.size = meta.meta.size;
        for time in meta.times {
            match time {
                Timestamp::Created(value) => file_report.created = value,
                Timestamp::Accessed(value) => file_report.accessed = value,
                Timestamp::Modified(value) => file_report.modified = value,
                Timestamp::Changed(value) => file_report.changed = value,
                _ => continue,
            }
        }
    }

    let hash = grab_file(&mut reader, zip)?;
    file_report.md5 = hash;

    Ok(file_report)
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::triage::{
            artifact::{acquire_files, get_ntfs_ads_zip_path, read_file, triage, walk_filesystem},
            reader::TriageReader,
        },
        filesystem::metadata::GlobInfo,
        output::manager::OutputManager,
        structs::artifacts::triage::TriageOptions,
        utils::regex_options::create_regex,
    };
    use std::{
        fs::{File, create_dir_all},
        path::PathBuf,
    };
    use zip::ZipWriter;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_triage() {
        let mut output = output_options("triage_test", "./tmp", false);
        let options = vec![TriageOptions {
            name: String::from("Linux Journal files"),
            path: String::from("/var/log/journal/"),
            file_mask: String::from("*user*"),
            recursive: true,
            recreate_directories: true,
        }];

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_triage_linux() {
        let mut output = output_options("triage_test", "./tmp", false);
        let options = vec![TriageOptions {
            name: String::from("Linux Journal files"),
            path: String::from("/var/log/journal/"),
            file_mask: String::from("*user*"),
            recursive: false,
            recreate_directories: true,
        }];

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_triage_linux_recursive() {
        let mut output = output_options("triage_test_recursive", "./tmp", false);
        let options = vec![TriageOptions {
            name: String::from("Linux Journal files"),
            path: String::from("/var/log/journal/"),
            file_mask: String::from("*user*"),
            recursive: true,
            recreate_directories: false,
        }];

        triage(&mut output, &options).unwrap();
    }

    #[test]
    fn test_acquire_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/");

        let target = TriageOptions {
            name: String::from("test"),
            recursive: false,
            file_mask: String::from("*.toml"),
            path: test_location.display().to_string(),
            recreate_directories: false,
        };

        let out = output_options("acquire_files", "./tmp", false);
        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();

        let zip = ZipWriter::new(zip_file);
        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };
        let mut report = Vec::new();
        acquire_files(&target, &mut acq, &mut report).unwrap();
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
        let out = output_options("walk_filesystem", "./tmp", false);

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();

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
        let out = output_options("walk_filesystem", "./tmp", false);

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();

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

        let out = output_options("read_file", "./tmp", false);

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();

        let zip = ZipWriter::new(zip_file);
        let mut acq = TriageReader {
            fs: None,
            zip,
            path: String::new(),
        };

        let report = read_file(test_location.to_str().unwrap(), &mut acq, true).unwrap();
        assert_eq!(report.md5, "7bf0a4b133b9e4d8aa8d279474ab3367");
        assert_eq!(report.size, 611);
    }

    #[test]
    fn test_get_ntfs_ads_zip_path() {
        assert_eq!(
            get_ntfs_ads_zip_path("C:\\Windows\\System32\\config\\SOFTWARE", "$SDS", true),
            "C:\\Windows\\System32\\config\\SOFTWARE_$SDS"
        );
        assert_eq!(
            get_ntfs_ads_zip_path("C:\\Windows\\System32\\config\\SOFTWARE", "$SDS", false),
            "SOFTWARE_$SDS"
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs() {
        use crate::artifacts::os::triage::artifact::read_file_ntfs;

        let out = output_options("read_file_ntfs", "./tmp", false);
        let path = "C:\\Windows\\System32\\config\\SOFTWARE";

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();
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

        let out = output_options("read_file_ntfs_ads", "./tmp", false);
        let path = "C:\\$Secure:$SDS";

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();
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
