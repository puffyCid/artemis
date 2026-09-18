use crate::{
    accessor::{
        access::Accessor, filesystem::ntfs::walk::NtfsWalkEntry, source::handle::SourceHandle,
    },
    artifacts::os::{files::error::FileError, windows::pe::parser::parse_pe_reader},
    filesystem::files::hash_reader,
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::{artifacts::os::files::FileOptions, toml::OutputFormat},
    utils::regex_options::regex_check,
};
use common::files::{EntryKind, FileNtfsInfo, Hashes};
use regex::Regex;
use std::mem::take;
use tracing::error;

/// Generate a filelisting for raw NTFS data
pub(crate) fn filelisting_ntfs(
    accessor: &mut Accessor,
    source: &SourceHandle,
    options: &FileOptions,
    manager: &mut OutputManager,
    path_filter: Regex,
    file_filter: Regex,
    yara_rule: String,
) -> Result<(), FileError> {
    let exclude = options
        .exclude_directories
        .clone()
        .unwrap_or_default()
        .into_iter()
        .collect();

    let depth = options.depth.unwrap_or(1);
    let max_list =
        if options.metadata.is_some_and(|b| b) || manager.config.format == OutputFormat::Timeline {
            1000
        } else {
            10000
        };

    let hashes = Hashes {
        md5: options.md5.unwrap_or_default(),
        sha1: options.sha1.unwrap_or_default(),
        sha256: options.sha256.unwrap_or_default(),
    };

    let mut batch = Vec::new();

    let mut visit = |entry: NtfsWalkEntry| {
        let mut row = entry.info;

        if options.path_regex.is_some() && !regex_check(&path_filter, &row.full_path) {
            return Ok(());
        }

        if options.filename_regex.is_some() && !regex_check(&file_filter, &row.filename) {
            return Ok(());
        }

        let Some(handle) = entry.handle.as_file() else {
            batch.push(row);
            return Ok(());
        };

        let max_size = 100 * 1024 * 1024;
        #[cfg(feature = "yarax")]
        if !yara_rule.is_empty() {
            use crate::utils::yara::scan_bytes;

            if row.kind != EntryKind::File {
                return Ok(());
            }
            if row.size > max_size {
                use tracing::info;

                info!(
                    "Skipping file {}. File size is {} vs 100MB max scans size",
                    row.display_path, row.size
                );
                return Ok(());
            }
            let bytes = match accessor.source_read_file_handle(source, handle) {
                Ok(bytes) => bytes,
                Err(err) => {
                    error!("Could not read file {}: {err:?}", handle.display_path());
                    return Ok(());
                }
            };
            match scan_bytes(&bytes, &yara_rule) {
                Ok(hits) if !hits.is_empty() => row.yara_hits = hits,
                Ok(_) => return Ok(()),
                Err(err) => {
                    use tracing::warn;

                    warn!("Failed to scan with yara: {err:?}");
                    return Ok(());
                }
            }
        }

        if options.metadata.is_some_and(|b| b)
            && row.kind == EntryKind::File
            && row.size < max_size
            && let Ok(mut reader) = accessor.source_open_reader_handle(source, handle)
        {
            row.binary_info = parse_pe_reader(&mut reader)
                .ok()
                .and_then(|info| serde_json::to_value(info).ok())
                .unwrap_or_default();
        }

        if (hashes.md5 || hashes.sha1 || hashes.sha256)
            && row.kind == EntryKind::File
            && let Ok(mut reader) = accessor.source_open_reader_handle(source, handle)
        {
            let (md5, sha1, sha256) = hash_reader(&hashes, &mut reader);
            row.md5 = md5;
            row.sha1 = sha1;
            row.sha256 = sha256;
        }
        row.evidence = source.display();

        batch.push(row);

        if batch.len() >= max_list {
            ntfs_output(take(&mut batch), manager, options);
        }
        Ok(())
    };

    accessor
        .source_walk_ntfs(source, &options.start_path, depth, &exclude, &mut visit)
        .map_err(|_| FileError::Filelisting)?;

    if !batch.is_empty() {
        ntfs_output(batch, manager, options);
    }

    Ok(())
}

/// Output batches of data for the NTFS filelisting
fn ntfs_output(entries: Vec<FileNtfsInfo>, manager: &mut OutputManager, options: &FileOptions) {
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize NTFS filelisting: {err:?}");
            return;
        }
    };
    if let Err(err) = manager.write_artifact("files_ntfs", options, &mut records) {
        error!("Failed to output NTFS filelisting: {err:?}");
    }
}
