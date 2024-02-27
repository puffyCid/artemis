use super::{
    data::{parse_categories_data, parse_dbstr_data, parse_properties_data, DataProperties},
    header::get_header,
    offsets::get_offsets,
};
use crate::{
    artifacts::os::macos::spotlight::error::SpotlightError,
    filesystem::{files::read_file, metadata::GlobInfo},
};
use log::error;
use std::collections::HashMap;

pub(crate) struct SpotlightMeta {
    pub(crate) props: HashMap<usize, DataProperties>,
    pub(crate) categories: HashMap<usize, String>,
    pub(crate) indexes1: HashMap<usize, Vec<u32>>,
    pub(crate) indexes2: HashMap<usize, Vec<u32>>,
}

/// Grab all metadata needed to parse Spotlight entries
pub(crate) fn get_spotlight_meta(paths: &[GlobInfo]) -> Result<SpotlightMeta, SpotlightError> {
    let mut meta = SpotlightMeta {
        props: HashMap::new(),
        categories: HashMap::new(),
        indexes1: HashMap::new(),
        indexes2: HashMap::new(),
    };

    for path in paths {
        if !path.full_path.contains(".header") || path.full_path.contains('3') {
            continue;
        }

        let header_data = read_dbstr(&path.full_path)?;
        let header = get_header(&header_data)?;

        let offsets_path = path.full_path.replace("header", "offsets");
        let offset_data = read_dbstr(&offsets_path)?;
        let offsets = get_offsets(&offset_data, &header.offset_entries)?;

        if path.full_path.contains("dbStr-1") {
            let props_path = path.full_path.replace("header", "data");
            let props_data = read_dbstr(&props_path)?;
            let prop_results = parse_properties_data(&props_data, &offsets);
            match prop_results {
                Ok((_, results)) => meta.props = results,
                Err(_err) => {
                    error!(
                        "[spotlight] Could not parse dbstr property: {}",
                        path.full_path
                    );
                    return Err(SpotlightError::Property);
                }
            }
        } else if path.full_path.contains("dbStr-2") {
            let category_path = path.full_path.replace("header", "data");
            let category_data = read_dbstr(&category_path)?;
            let cat_results = parse_categories_data(&category_data, &offsets);
            match cat_results {
                Ok((_, results)) => meta.categories = results,
                Err(_err) => {
                    error!(
                        "[spotlight] Could not parse dbstr category: {}",
                        path.full_path
                    );
                    return Err(SpotlightError::Category);
                }
            }
        } else if path.full_path.contains("dbStr-4") {
            let indexes_path = path.full_path.replace("header", "data");
            let indexes_data = read_dbstr(&indexes_path)?;
            let indexes_results = parse_dbstr_data(&indexes_data, &offsets, &false);
            match indexes_results {
                Ok((_, results)) => meta.indexes1 = results,
                Err(_err) => {
                    error!(
                        "[spotlight] Could not parse dbstr indexes1: {}",
                        path.full_path
                    );
                    return Err(SpotlightError::Indexes1);
                }
            }
        } else if path.full_path.contains("dbStr-5") {
            let indexes_path = path.full_path.replace("header", "data");
            let indexes_data = read_dbstr(&indexes_path)?;
            let indexes_results = parse_dbstr_data(&indexes_data, &offsets, &true);
            match indexes_results {
                Ok((_, results)) => meta.indexes2 = results,
                Err(_err) => {
                    error!(
                        "[spotlight] Could not parse dbstr indexes2: {}",
                        path.full_path
                    );
                    return Err(SpotlightError::Indexes2);
                }
            }
        }
    }

    Ok(meta)
}

/// Read the Dbstr files
fn read_dbstr(path: &str) -> Result<Vec<u8>, SpotlightError> {
    let data_results = read_file(path);
    let data = match data_results {
        Ok(results) => results,
        Err(err) => {
            error!("[spotlight] Could not read dbstr file {path}: {err:?}");
            return Err(SpotlightError::ReadFile);
        }
    };

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::{get_spotlight_meta, read_dbstr};
    use crate::filesystem::metadata::glob_paths;
    use std::path::PathBuf;

    #[test]
    fn test_get_spotlight_meta() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*.header");
        let paths = glob_paths(test_location.to_str().unwrap()).unwrap();
        let meta = get_spotlight_meta(&paths).unwrap();
        assert_eq!(meta.props.len(), 109);
    }

    #[test]
    fn test_read_dbstr() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/dbStr-1.map.header");
        let results = read_dbstr(test_location.to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 56);
    }
}
