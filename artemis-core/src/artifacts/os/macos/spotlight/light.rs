use super::{dbstr::meta::get_spotlight_meta, error::SpotlightError, store::db::parse_store_db};
use crate::{
    filesystem::{files::read_file, metadata::glob_paths},
    structs::toml::Output,
};
use common::macos::SpotlightEntries;
use log::error;

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
        let store_bytes_result = read_file(&path.full_path);
        let store_bytes = match store_bytes_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[spotlight] Could not read store.db {}: {err:?}",
                    path.full_path
                );
                return Err(SpotlightError::ReadFile);
            }
        };

        println!("path: {}", path.full_path);

        let result = parse_store_db(&store_bytes, &meta, output, start_time, filter);
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
