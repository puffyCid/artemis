use super::{
    dbstr::meta::{SpotlightMeta, get_spotlight_meta},
    error::SpotlightError,
    store::db::{parse_store, parse_store_blocks},
};
use crate::{
    accessor::{access::Accessor, entry::handle::EntryKind},
    artifacts::os::macos::spotlight::store::db::get_blocks,
    output::manager::OutputManager,
    structs::artifacts::os::macos::SpotlightOptions,
};
use common::macos::SpotlightEntries;
use serde::{Deserialize, Serialize};
use tracing::error;

/// Parse the Spotlight database and output results
pub(crate) fn parse_spotlight(
    glob_path: &str,
    manager: &mut OutputManager,
    options: &SpotlightOptions,
    accessor: &mut Accessor,
) -> Result<(), SpotlightError> {
    let paths = match accessor.globfs(glob_path) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not glob {glob_path}: {err:?}");
            return Err(SpotlightError::Glob);
        }
    };

    let meta = get_spotlight_meta(&paths, accessor)?;
    for path in paths {
        if path.meta.kind != EntryKind::File {
            continue;
        }

        let Some(file_handle) = path.handle.as_file() else {
            continue;
        };

        if !file_handle.display_path().ends_with("store.db") {
            continue;
        }

        let mut store_reader = match accessor.open_reader_handle(file_handle) {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "Could not create reader for store.db {}: {err:?}",
                    file_handle.display_path()
                );
                return Err(SpotlightError::ReadFile);
            }
        };

        let result = parse_store(&mut store_reader, &meta, manager, options);
        if result.is_err() {
            error!(
                "Could not parse the spotlight store at: {}",
                file_handle.display_path()
            );
        }

        break;
    }

    Ok(())
}

/// Create a file reader and read the Spotlight database in blocks. This allows for a **little** more flexible JS scripting.
pub(crate) fn parse_spotlight_reader(
    store_file: &str,
    meta: &SpotlightMeta,
    blocks: &[u32],
    offset: u32,
) -> Result<Vec<SpotlightEntries>, SpotlightError> {
    let reader_result = Accessor::with_defaults().open_reader(store_file);
    let mut store_reader = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("Could not create reader for store.db {store_file}: {err:?}",);
            return Err(SpotlightError::ReadFile);
        }
    };

    let entries = parse_store_blocks(&mut store_reader, meta, blocks, offset, store_file)?;
    Ok(entries)
}

#[derive(Deserialize, Serialize)]
pub(crate) struct StoreMeta {
    pub(crate) meta: SpotlightMeta,
    pub(crate) blocks: Vec<u32>,
}

/// Setup Spotlight reader by getting the minimum amount of metadata to stream the Spotlight database
pub(crate) fn setup_spotlight_reader(glob_path: &str) -> Result<StoreMeta, SpotlightError> {
    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(glob_path) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not glob {glob_path}: {err:?}");
            return Err(SpotlightError::Glob);
        }
    };
    let meta = get_spotlight_meta(&paths, &mut accessor)?;
    let mut blocks = Vec::new();
    for path in paths {
        if path.meta.kind != EntryKind::File {
            continue;
        }

        let Some(file_handle) = path.handle.as_file() else {
            continue;
        };

        if !file_handle.display_path().contains("store.db") {
            continue;
        }

        let mut store_reader = match accessor.open_reader_handle(file_handle) {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "Could not create reader for store.db {}: {err:?}",
                    file_handle.display_path()
                );
                return Err(SpotlightError::ReadFile);
            }
        };
        let (results, _) = get_blocks(&mut store_reader)?;
        blocks = results;
        break;
    }

    let store_meta = StoreMeta { meta, blocks };

    Ok(store_meta)
}

#[cfg(test)]
mod tests {
    use super::{parse_spotlight, parse_spotlight_reader, setup_spotlight_reader};
    use crate::accessor::access::Accessor;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{output::manager::OutputManager, structs::artifacts::os::macos::SpotlightOptions};
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputConfig {
        OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    #[test]
    fn test_parse_spotlight() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*");
        let output = output_options("spotlight_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();
        let mut accessor = Accessor::with_defaults();

        parse_spotlight(
            test_location.to_str().unwrap(),
            &mut manage,
            &SpotlightOptions {
                alt_dir: None,
                include_additional: Some(false),
            },
            &mut accessor,
        )
        .unwrap();
    }

    #[test]
    fn test_setup_spotlight_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*");
        let result = setup_spotlight_reader(&test_location.to_str().unwrap()).unwrap();

        assert_eq!(result.meta.categories.len(), 4708);
    }

    #[test]
    fn test_parse_spotlight_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*");
        let result = setup_spotlight_reader(&test_location.to_str().unwrap()).unwrap();

        test_location.pop();
        test_location.push("store.db");

        let entries = parse_spotlight_reader(
            test_location.to_str().unwrap(),
            &result.meta,
            &result.blocks,
            0,
        )
        .unwrap();
        assert_eq!(entries.len(), 1022);
        assert_eq!(entries[10].inode, 12884902012);
    }
}
