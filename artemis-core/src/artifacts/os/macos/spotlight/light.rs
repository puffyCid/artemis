use super::{
    dbstr::meta::{get_spotlight_meta, SpotlightMeta},
    error::SpotlightError,
    store::db::{parse_store, parse_store_blocks},
};
use crate::{
    artifacts::os::macos::spotlight::store::db::get_blocks,
    filesystem::{files::file_reader, metadata::glob_paths},
    structs::toml::Output,
};
use common::macos::SpotlightEntries;
use log::error;

/// Parse the Spotlight database and output results
pub(crate) fn parse_spotlight(
    glob_path: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), SpotlightError> {
    let paths_result = glob_paths(glob_path);
    let paths = match paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[spotlight] Could not glob {glob_path}: {err:?}");
            return Err(SpotlightError::Glob);
        }
    };

    let meta = get_spotlight_meta(&paths)?;
    for path in paths {
        if path.filename != "store.db" {
            continue;
        }
        let reader_result = file_reader(&path.full_path);
        let mut store_reader = match reader_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[spotlight] Could not create reader for store.db {}: {err:?}",
                    path.full_path
                );
                return Err(SpotlightError::ReadFile);
            }
        };

        let result = parse_store(&mut store_reader, &meta, output, start_time, filter);
        if result.is_err() {
            error!(
                "[spotlight] Could not parse the spotlight store at: {}",
                path.full_path
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
    offset: &u32,
) -> Result<Vec<SpotlightEntries>, SpotlightError> {
    let reader_result = file_reader(store_file);
    let mut store_reader = match reader_result {
        Ok(result) => result,
        Err(err) => {
            error!("[spotlight] Could not create reader for store.db {store_file}: {err:?}",);
            return Err(SpotlightError::ReadFile);
        }
    };

    let entries = parse_store_blocks(&mut store_reader, meta, blocks, offset, store_file)?;
    Ok(entries)
}

pub(crate) struct StoreMeta {
    meta: SpotlightMeta,
    blocks: Vec<u32>,
}

/// Setup Spotlight reader by getting the minimum amount of metadata to stream the Spotlight database
pub(crate) fn setup_spotlight_reader(glob_path: &str) -> Result<StoreMeta, SpotlightError> {
    let paths_result = glob_paths(glob_path);
    let paths = match paths_result {
        Ok(result) => result,
        Err(err) => {
            error!("[spotlight] Could not glob {glob_path}: {err:?}");
            return Err(SpotlightError::Glob);
        }
    };
    let meta = get_spotlight_meta(&paths)?;
    let mut blocks = Vec::new();
    for path in paths {
        if path.filename != "store.db" {
            continue;
        }
        let reader_result = file_reader(&path.full_path);
        let mut store_reader = match reader_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[spotlight] Could not create reader for store.db {}: {err:?}",
                    path.full_path
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
    use crate::structs::toml::Output;
    use std::path::PathBuf;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_parse_spotlight() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*");
        let mut output = output_options("spotlight_test", "local", "./tmp", false);

        parse_spotlight(test_location.to_str().unwrap(), &mut output, &0, &false).unwrap();
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
            &0,
        )
        .unwrap();
        assert_eq!(entries.len(), 1022);
        assert_eq!(entries[10].inode, 12884902012);
    }
}
