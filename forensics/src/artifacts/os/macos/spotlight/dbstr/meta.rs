use super::{
    data::{DataProperties, parse_categories_data, parse_dbstr_data, parse_properties_data},
    header::get_header,
    offsets::get_offsets,
};
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, FileHandle, GlobMatch},
    },
    artifacts::os::macos::spotlight::error::SpotlightError,
    filesystem::files::get_filename,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;

#[derive(Deserialize, Serialize)]
pub(crate) struct SpotlightMeta {
    pub(crate) props: HashMap<usize, DataProperties>,
    pub(crate) categories: HashMap<usize, String>,
    pub(crate) indexes1: HashMap<usize, Vec<u32>>,
    pub(crate) indexes2: HashMap<usize, Vec<u32>>,
}

/// Grab all metadata needed to parse Spotlight entries
pub(crate) fn get_spotlight_meta(
    paths: &[GlobMatch],
    accessor: &mut Accessor,
) -> Result<SpotlightMeta, SpotlightError> {
    let mut meta = SpotlightMeta {
        props: HashMap::new(),
        categories: HashMap::new(),
        indexes1: HashMap::new(),
        indexes2: HashMap::new(),
    };

    let mut meta_maps = HashMap::new();

    for path in paths {
        if path.meta.kind != EntryKind::File {
            continue;
        }
        let Some(file_handle) = path.handle.as_file() else {
            continue;
        };

        meta_maps.insert(get_filename(&file_handle.display_path()), file_handle);
    }

    if let Some(header_handle) = meta_maps.get("dbStr-1.map.header")
        && let Some(props_handle) = meta_maps.get("dbStr-1.map.data")
        && let Some(offsets_handle) = meta_maps.get("dbStr-1.map.offsets")
    {
        let offsets = parse_spotlight_header(header_handle, offsets_handle, accessor)?;
        let props_data = read_dbstr(props_handle, accessor)?;
        let props = match parse_properties_data(&props_data, &offsets) {
            Ok((_, results)) => results,
            Err(_err) => {
                error!(
                    "Could not parse dbstr property: '{}'",
                    props_handle.display_path()
                );
                return Err(SpotlightError::Property);
            }
        };

        meta.props = props;
    }

    if let Some(header_handle) = meta_maps.get("dbStr-2.map.header")
        && let Some(category_handle) = meta_maps.get("dbStr-2.map.data")
        && let Some(offsets_handle) = meta_maps.get("dbStr-2.map.offsets")
    {
        let offsets = parse_spotlight_header(header_handle, offsets_handle, accessor)?;
        let category_data = read_dbstr(category_handle, accessor)?;
        let categories = match parse_categories_data(&category_data, &offsets) {
            Ok((_, results)) => results,
            Err(_err) => {
                error!(
                    "Could not parse dbstr category: {}",
                    category_handle.display_path()
                );
                return Err(SpotlightError::Category);
            }
        };

        meta.categories = categories;
    }

    if let Some(header_handle) = meta_maps.get("dbStr-4.map.header")
        && let Some(indexes_handle) = meta_maps.get("dbStr-4.map.data")
        && let Some(offsets_handle) = meta_maps.get("dbStr-4.map.offsets")
    {
        let offsets = parse_spotlight_header(header_handle, offsets_handle, accessor)?;
        let category_data = read_dbstr(indexes_handle, accessor)?;
        let indexes = match parse_dbstr_data(&category_data, &offsets, false) {
            Ok((_, results)) => results,
            Err(_err) => {
                error!(
                    "Could not parse dbstr indexes1: {}",
                    indexes_handle.display_path()
                );
                return Err(SpotlightError::Indexes1);
            }
        };

        meta.indexes1 = indexes;
    }

    if let Some(header_handle) = meta_maps.get("dbStr-5.map.header")
        && let Some(indexes_handle) = meta_maps.get("dbStr-5.map.data")
        && let Some(offsets_handle) = meta_maps.get("dbStr-5.map.offsets")
    {
        let offsets = parse_spotlight_header(header_handle, offsets_handle, accessor)?;
        let category_data = read_dbstr(indexes_handle, accessor)?;
        let indexes = match parse_dbstr_data(&category_data, &offsets, true) {
            Ok((_, results)) => results,
            Err(_err) => {
                error!(
                    "Could not parse dbstr indexes2: {}",
                    indexes_handle.display_path()
                );
                return Err(SpotlightError::Indexes2);
            }
        };

        meta.indexes2 = indexes;
    }

    Ok(meta)
}

/// Parse common header info for `Spotlight` data
fn parse_spotlight_header(
    header_handle: &FileHandle,
    offsets_handle: &FileHandle,
    accessor: &mut Accessor,
) -> Result<Vec<u32>, SpotlightError> {
    let header_data = read_dbstr(header_handle, accessor)?;
    let header = get_header(&header_data)?;
    let offset_data = read_dbstr(offsets_handle, accessor)?;
    let offsets = get_offsets(&offset_data, header.offset_entries)?;

    Ok(offsets)
}

/// Read the Dbstr files
fn read_dbstr(handle: &FileHandle, accessor: &mut Accessor) -> Result<Vec<u8>, SpotlightError> {
    let data_results = accessor.read_file_handle(handle);
    let data = match data_results {
        Ok(results) => results,
        Err(err) => {
            error!(
                "Could not read dbstr file '{}': {err:?}",
                handle.display_path()
            );
            return Err(SpotlightError::ReadFile);
        }
    };

    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::{get_spotlight_meta, read_dbstr};
    use crate::{
        accessor::{access::Accessor, entry::handle::FileHandle},
        artifacts::os::macos::spotlight::dbstr::meta::parse_spotlight_header,
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_spotlight_meta() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/*");
        let paths = Accessor::with_defaults()
            .globfs(test_location.to_str().unwrap())
            .unwrap();

        let meta = get_spotlight_meta(&paths, &mut Accessor::with_defaults()).unwrap();
        assert_eq!(meta.props.len(), 109);
    }

    #[test]
    fn test_read_dbstr() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/dbStr-1.map.header");
        let handle = FileHandle::host(test_location);
        let results = read_dbstr(&handle, &mut Accessor::with_defaults()).unwrap();
        assert_eq!(results.len(), 56);
    }

    #[test]
    fn test_parse_spotlight_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/spotlight/bigsur/dbStr-1.map.header");
        let handle = FileHandle::host(&test_location);
        test_location.pop();
        test_location.push("dbStr-1.map.offsets");

        let handle2 = FileHandle::host(test_location);
        let offsets =
            parse_spotlight_header(&handle, &handle2, &mut Accessor::with_defaults()).unwrap();
        assert_eq!(offsets.len(), 110);
    }
}
