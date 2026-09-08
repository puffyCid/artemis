use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{FileHandle, Timestamp},
        io::reader::AccessorReader,
        source::handle::SourceHandle,
        walk::WalkAccessor,
    },
    artifacts::os::{
        systeminfo::info::{PlatformType, get_platform_enum},
        triage::{
            error::TriageError,
            reader::{grab_file, write_report},
        },
    },
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::triage::TriageOptions,
    utils::regex_options::{create_regex, regex_check},
};
use glob::Pattern;
use regex::Regex;
use serde::Serialize;
use std::fs::{File, create_dir_all};
use tracing::{error, info, warn};
use zip::ZipWriter;

/// Triage a system by acquiring files
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

    // For now this Triage artifact is inspired by KAPE
    // Which supports running on a live system
    // Maybe in future the Triage artifact can support other accessor types (ex: disk images)
    let source = match accessor.open_source("host:") {
        Ok(result) => result,
        Err(err) => {
            error!("Could not open host source: {err:?}");
            return Err(TriageError::NoReader);
        }
    };
    // Loop through all triage targets
    for target in options {
        acquire_files(target, &mut report, &mut accessor, &source, &mut zip)?;
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
        let pattern =
            match create_regex(target.file_mask.strip_prefix("regex:").unwrap_or_default()) {
                Ok(result) => result,
                Err(err) => {
                    error!("Could not create regex: {err:?}");
                    return Err(TriageError::Regex);
                }
            };
        file_pattern = Some(pattern);
    }

    info!("Applying glob on '{glob_string}'");

    let paths = accessor
        .source_globfs(source, &glob_string)
        .unwrap_or_default();
    let file_mask = if !target.file_mask.starts_with("regex:")
        && let Ok(value) = Pattern::new(&target.file_mask)
    {
        value
    } else {
        Pattern::default()
    };

    for path in paths {
        if let Some(handle) = path.handle.as_directory()
            && target.recursive
        {
            info!("Walking the directory: '{}'", handle.full_path());
            let mut walk = match WalkAccessor::new(source, &handle.full_path()) {
                Ok(results) => results,
                Err(err) => {
                    warn!("Could not start walk for {}: {err:?}", handle.full_path());
                    continue;
                }
            };

            let depth = 1000;
            walk = walk.max_depth(depth);

            walking(
                walk,
                file_pattern.as_ref(),
                report,
                &file_mask,
                zip,
                accessor,
                source,
            )?;
            continue;
        }

        let Some(handle) = path.handle.as_file() else {
            continue;
        };

        if file_pattern.is_none() && !file_mask.matches(&path.meta.filename) {
            continue;
        }

        // If regex is being used. Then check if our filename matches
        if file_pattern
            .as_ref()
            .is_some_and(|pat| !regex_check(pat, &path.meta.filename))
        {
            continue;
        }

        if let Ok(file_report) = read_file(handle, accessor, source, zip) {
            report.push(file_report);
        }
    }

    Ok(())
}

/// Transverse the filesystem and acquire all files that match the provided glob or regex
fn walking(
    mut walk: WalkAccessor,
    pattern: Option<&Regex>,
    report: &mut Vec<TriageReport>,
    file_mask: &Pattern,
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
            && file_mask.matches(&entry.entry.meta.filename)
        {
            let Some(handle) = entry.entry.handle.as_file() else {
                continue;
            };

            if let Ok(file_report) = read_file(handle, accessor, source, zip) {
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

        if let Ok(file_report) = read_file(handle, accessor, source, zip) {
            report.push(file_report);
        }
    }

    Ok(())
}

/// Read the target file that matched the glob or regex
fn read_file(
    handle: &FileHandle,
    accessor: &mut Accessor,
    source: &SourceHandle,
    zip: &mut ZipWriter<File>,
) -> Result<TriageReport, TriageError> {
    let mut reader = match accessor.source_open_reader_handle(source, handle) {
        Ok(result) => result,
        Err(err) => {
            warn!(
                "Could not open host reader for {}: {err:?}",
                handle.display_path()
            );
            // On Windows we try the NTFS accessor if a file is locked
            if get_platform_enum() == PlatformType::Windows
                && handle.display_path().starts_with("host:")
            {
                read_file_locked(accessor, &handle.full_path())?
            } else {
                return Err(TriageError::ReadFile);
            }
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
                _ => {}
            }
        }
    }

    let hash = grab_file(&mut reader, zip)?;
    file_report.md5 = hash;

    Ok(file_report)
}

/// Acquire a file by parsing the NTFS filesystem. Will bypass locked files
fn read_file_locked(accessor: &mut Accessor, path: &str) -> Result<AccessorReader, TriageError> {
    let ntfs = format!("ntfs:{path}");
    let reader = match accessor.open_reader(&ntfs) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to open ntfs reader for locked file '{path}': {err:?}");
            return Err(TriageError::NoReader);
        }
    };

    Ok(reader)
}

#[cfg(test)]
mod tests {
    use crate::accessor::access::Accessor;
    use crate::accessor::entry::handle::FileHandle;
    use crate::accessor::walk::WalkAccessor;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::triage::artifact::{acquire_files, read_file, triage, walking},
        output::manager::OutputManager,
        structs::artifacts::triage::TriageOptions,
        utils::regex_options::create_regex,
    };
    use glob::Pattern;
    use std::fs::{self, remove_dir_all};
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

        let mut zip = ZipWriter::new(zip_file);

        let mut report = Vec::new();
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();
        acquire_files(&target, &mut report, &mut accessor, &source, &mut zip).unwrap();
    }

    #[test]
    fn test_walking() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/triage/malformed");

        let out = output_options("walk_filesystem", "./tmp", false);

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();

        let mut zip = ZipWriter::new(zip_file);
        let mut report = Vec::new();
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let walk = WalkAccessor::new(&source, test_location.to_str().unwrap()).unwrap();

        walking(
            walk,
            None,
            &mut report,
            &Pattern::new("bad.toml").unwrap(),
            &mut zip,
            &mut accessor,
            &source,
        )
        .unwrap();
        assert_eq!(report.len(), 1);
        assert_eq!(report[0].md5, "695aacf9c82357da7564cb875604fd62");
    }

    #[test]
    fn test_walk_filesystem_regex() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/triage/malformed");

        let out = output_options("walk_filesystem", "./tmp", false);

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();

        let mut zip = ZipWriter::new(zip_file);
        let mut report = Vec::new();
        let patter = create_regex("bad.*").unwrap();

        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let walk = WalkAccessor::new(&source, test_location.to_str().unwrap()).unwrap();
        walking(
            walk,
            Some(&patter),
            &mut report,
            &Pattern::new("").unwrap(),
            &mut zip,
            &mut accessor,
            &source,
        )
        .unwrap();

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

        let mut zip = ZipWriter::new(zip_file);
        let handle = FileHandle::host(test_location);
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let report = read_file(&handle, &mut accessor, &source, &mut zip).unwrap();
        assert_eq!(report.md5, "7bf0a4b133b9e4d8aa8d279474ab3367");
        assert_eq!(report.size, 611);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs() {
        let out = output_options("read_file_ntfs", "./tmp", false);
        let path = "C:\\Windows\\System32\\config\\SOFTWARE";

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();
        let mut zip = ZipWriter::new(zip_file);

        let handle = FileHandle::host(path);
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let report = read_file(&handle, &mut accessor, &source, &mut zip).unwrap();
        assert!(!report.md5.is_empty())
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs_ads() {
        let out = output_options("read_file_ntfs_ads", "./tmp", false);
        let path = "C:\\$Secure:$SDS";

        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();
        let zip_file = File::create(format!("{}/files.zip", zip_output.to_str().unwrap())).unwrap();
        let mut zip = ZipWriter::new(zip_file);

        let handle = FileHandle::host(path);
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let report = read_file(&handle, &mut accessor, &source, &mut zip).unwrap();
        assert!(!report.md5.is_empty());
    }

    fn setup_zip(name: &str) -> (ZipWriter<File>, PathBuf) {
        let out = output_options(name, "./tmp", false);
        let zip_output = out.config.directory.join(&out.config.name);
        create_dir_all(&zip_output).unwrap();

        let zip_path = zip_output.join("files.zip");
        let zip = ZipWriter::new(File::create(&zip_path).unwrap());

        (zip, zip_path)
    }

    fn setup_tree(name: &str) -> PathBuf {
        let dir = PathBuf::from("./tmp").join(name);
        let _ = remove_dir_all(&dir);
        create_dir_all(dir.join("a/b")).unwrap();

        fs::write(dir.join("keep.log"), b"a").unwrap();
        fs::write(dir.join("skip.txt"), b"b").unwrap();
        fs::write(dir.join("a/b/keep.log"), b"c").unwrap();
        fs::write(dir.join("a/b/skip.txt"), b"d").unwrap();
        dir
    }

    #[test]
    fn test_acquire_files_recursive_mask_and_depth() {
        let dir = setup_tree("triage_mask");
        let target = TriageOptions {
            name: String::from("mask"),
            path: format!("{}/", dir.display()),
            file_mask: String::from("*.log"),
            recursive: true,
            recreate_directories: true,
        };

        let (mut zip, _) = setup_zip("triage_mask");
        let mut report = Vec::new();
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        acquire_files(&target, &mut report, &mut accessor, &source, &mut zip).unwrap();
        let mut names: Vec<_> = report.iter().map(|r| r.filename.clone()).collect();
        names.sort();

        assert_eq!(names, vec!["keep.log", "keep.log"]);

        assert!(report.iter().any(
            |r| r.full_path.contains("a/b/keep.log") || r.full_path.contains("a\\b\\keep.log")
        ));

        assert!(!report.iter().any(|r| r.filename == "skip.txt"));
    }

    #[test]
    fn test_acquire_files_regex_prefix() {
        let dir = setup_tree("triage_regex");
        let target = TriageOptions {
            name: String::from("regex"),
            path: format!("{}/", dir.display()),
            file_mask: String::from(r"regex:keep\..*"),
            recursive: true,
            recreate_directories: true,
        };

        let (mut zip, _) = setup_zip("triage_regex");
        let mut report = Vec::new();
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        acquire_files(&target, &mut report, &mut accessor, &source, &mut zip).unwrap();

        let mut names: Vec<_> = report.iter().map(|r| r.filename.clone()).collect();
        names.sort();

        assert_eq!(names, vec!["keep.log", "keep.log"]);
        assert!(!report.iter().any(|r| r.filename == "skip.txt"));
    }

    #[test]
    fn test_acquire_files_missing_path_ok() {
        let dir = setup_tree("triage_missing");
        let missing = TriageOptions {
            name: String::from("missing"),
            path: String::from("/no/such/triage_dir/"),
            file_mask: String::from("*.log"),
            recursive: true,
            recreate_directories: true,
        };

        let present = TriageOptions {
            name: String::from("present"),
            path: format!("{}/", dir.display()),
            file_mask: String::from("*.log"),
            recursive: true,
            recreate_directories: true,
        };

        let (mut zip, _) = setup_zip("triage_missing");
        let mut report = Vec::new();
        let mut accessor = Accessor::with_defaults();

        let source = accessor.open_source("host:").unwrap();

        acquire_files(&missing, &mut report, &mut accessor, &source, &mut zip).unwrap();
        assert!(report.is_empty());

        acquire_files(&present, &mut report, &mut accessor, &source, &mut zip).unwrap();
        assert!(!report.is_empty());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs_zip_entry_name() {
        let (mut zip, zip_path) = setup_zip("read_file_ntfs_zip_name");
        let handle = FileHandle::host("C:\\Windows\\System32\\config\\SOFTWARE");
        let mut accessor = Accessor::with_defaults();

        let source = accessor.open_source("host:").unwrap();
        let report = read_file(&handle, &mut accessor, &source, &mut zip).unwrap();
        assert!(!report.md5.is_empty());

        zip.finish().unwrap();
        let archive = ZipArchive::new(File::open(zip_path).unwrap()).unwrap();
        let name = archive.name_for_index(0).unwrap();
        assert!(!name.contains(':'));

        assert!(
            name.contains("C_\\Windows\\System32\\config\\SOFTWARE")
                || name.contains("C_/Windows/System32/config/SOFTWARE")
        );
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_read_file_ntfs_ads_zip_entry_name() {
        let (mut zip, zip_path) = setup_zip("read_file_ntfs_ads_zip_name");
        let handle = FileHandle::host("C:\\$Secure:$SDS");
        let mut accessor = Accessor::with_defaults();
        let source = accessor.open_source("host:").unwrap();

        let report = read_file(&handle, &mut accessor, &source, &mut zip).unwrap();
        assert!(!report.md5.is_empty());
        zip.finish().unwrap();

        let archive = ZipArchive::new(File::open(zip_path).unwrap()).unwrap();
        let name = archive.name_for_index(0).unwrap();

        assert!(!name.contains(':'));
        assert!(name.contains("$Secure_$SDS") || name.contains("$SDS"));
    }
}
