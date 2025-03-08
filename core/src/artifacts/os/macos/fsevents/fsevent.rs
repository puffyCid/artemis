use crate::{
    filesystem::metadata::get_timestamps,
    utils::{
        nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
        strings::extract_utf8_string,
    },
};
use common::macos::FsEvents;
use log::warn;
use nom::bytes::complete::{take, take_while};
use std::mem::size_of;

#[derive(Debug)]
struct FsEventsHeader {
    /**File signature DLS1 or DLS2 */
    signature: u32,
    /**Size of stream of `FsEvent` records, includes the header size */
    stream_size: u32,
}

/// Parse provided `FsEvent` data
pub(crate) fn fsevents_data<'a>(
    data: &'a [u8],
    path: &str,
) -> nom::IResult<&'a [u8], Vec<FsEvents>> {
    let mut total_fsevents: Vec<FsEvents> = Vec::new();
    let mut input = data;
    let disk_loggerv1 = 0x444c5331; // DLS1
    let disk_loggerv2 = 0x444c5332; // DLS2
    let disk_loggerv3 = 0x444c5333; // DLS3
    let versions = [disk_loggerv1, disk_loggerv2, disk_loggerv3];
    // Loop through all the `FsEvent` data
    while !input.is_empty() {
        // Parse header to get `FsEvent` stream size
        let (fsevents_data, fsevents_header) = fsevents_header(input)?;
        if !versions.contains(&fsevents_header.signature) {
            warn!(
                "[fsevents] Got unknown header: {}",
                fsevents_header.signature
            );
            break;
        }

        let header_size = 12;
        let (stream_input, fsevent_data) =
            take(fsevents_header.stream_size - header_size)(fsevents_data)?;

        // Parse `FsEvent` stream data
        let (_result, mut fsevents) = get_fsevent(fsevent_data, fsevents_header.signature, path)?;
        total_fsevents.append(&mut fsevents);
        input = stream_input;
    }

    Ok((input, total_fsevents))
}

/// Begin parsing `FsEvent` stream
fn get_fsevent<'a>(data: &'a [u8], sig: u32, path: &str) -> nom::IResult<&'a [u8], Vec<FsEvents>> {
    let mut input_results = data;
    let mut fsevents_array: Vec<FsEvents> = Vec::new();

    // Parse `FsEvent` stream and get each `FsEvent` record
    while !input_results.is_empty() {
        let (input_data, fsevent_results) = get_fsevent_data(input_results, &sig, path)?;
        input_results = input_data;
        fsevents_array.push(fsevent_results);
    }

    Ok((input_results, fsevents_array))
}

/// Parse `FsEvent` header
fn fsevents_header(data: &[u8]) -> nom::IResult<&[u8], FsEventsHeader> {
    let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, stream_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let fsevent = FsEventsHeader {
        signature,
        stream_size,
    };

    Ok((input, fsevent))
}

/// Parse `FsEvent` stream entry
fn get_fsevent_data<'a>(data: &'a [u8], sig: &u32, path: &str) -> nom::IResult<&'a [u8], FsEvents> {
    let mut fsevent_data = FsEvents {
        flags: Vec::new(),
        path: String::new(),
        node: 0,
        event_id: 0,
        source: path.to_string(),
        source_created: String::from("1601-01-01T00:00:00Z"),
        source_modified: String::from("1601-01-01T00:00:00Z"),
        source_changed: String::from("1601-01-01T00:00:00Z"),
        source_accessed: String::from("1601-01-01T00:00:00Z"),
    };

    let meta_result = get_timestamps(path);
    match meta_result {
        Ok(result) => {
            fsevent_data.source_accessed = result.accessed;
            fsevent_data.source_changed = result.changed;
            fsevent_data.source_created = result.created;
            fsevent_data.source_modified = result.modified;
        }
        Err(err) => warn!("[fsvents] Could not get timestamps {err:?}"),
    }

    // Read path until end-of-string character
    let (input, path) = take_while(|b: u8| b != 0)(data)?;
    // Nom end-of-string character
    let (input, _) = take(size_of::<u8>())(input)?;

    let (input, fsevent_id) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, fsevent_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let flag_list = match_flags(&fsevent_flags);

    fsevent_data.flags = flag_list;
    fsevent_data.event_id = fsevent_id;

    // Ensure every path has root slash
    fsevent_data.path = format!("/{}", extract_utf8_string(path));

    // Strip any paths that have duplicative root slashes
    if fsevent_data.path.starts_with("//") {
        fsevent_data.path = (fsevent_data.path[1..]).to_string();
    }

    let disk_loggerv1 = 0x444c5331;
    if sig != &disk_loggerv1 {
        let (input, fsevent_node) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        fsevent_data.node = fsevent_node;

        // Version 3 has another 4 bytes. Seems to alway be 0
        let disk_loggvrv3 = 0x444c5333;
        if sig == &disk_loggvrv3 {
            let (input, _) = nom_unsigned_four_bytes(input, Endian::Le)?;
            return Ok((input, fsevent_data));
        }
        return Ok((input, fsevent_data));
    }

    Ok((input, fsevent_data))
}

/// Identify Event flags in `FsEvent` entry
fn match_flags(flags: &u32) -> Vec<String> {
    let mut flag_list: Vec<String> = Vec::new();
    if (flags & 0x01) != 0 {
        flag_list.push("Created".to_string());
    }
    if (flags & 0x02) != 0 {
        flag_list.push("Removed".to_string());
    }
    if (flags & 0x04) != 0 {
        flag_list.push("InodeMetadataModified".to_string());
    }
    if (flags & 0x08) != 0 {
        flag_list.push("Renamed".to_string());
    }
    if (flags & 0x10) != 0 {
        flag_list.push("Modified".to_string());
    }
    if (flags & 0x20) != 0 {
        flag_list.push("Exchange".to_string());
    }
    if (flags & 0x40) != 0 {
        flag_list.push("FinderInfoModified".to_string());
    }
    if (flags & 0x80) != 0 {
        flag_list.push("DirectoryCreated".to_string());
    }
    if (flags & 0x100) != 0 {
        flag_list.push("PermissionChanged".to_string());
    }
    if (flags & 0x200) != 0 {
        flag_list.push("ExtendedAttributeModified".to_string());
    }
    if (flags & 0x400) != 0 {
        flag_list.push("ExtendedAttributeRemoved".to_string());
    }
    if (flags & 0x800) != 0 {
        flag_list.push("DocumentCreated".to_string());
    }
    if (flags & 0x1000) != 0 {
        flag_list.push("DocumentRevision".to_string());
    }
    if (flags & 0x2000) != 0 {
        flag_list.push("UnmountPending".to_string());
    }
    if (flags & 0x4000) != 0 {
        flag_list.push("ItemCloned".to_string());
    }
    if (flags & 0x10000) != 0 {
        flag_list.push("NotificationClone".to_string());
    }
    if (flags & 0x20000) != 0 {
        flag_list.push("ItemTruncated".to_string());
    }
    if (flags & 0x40000) != 0 {
        flag_list.push("DirectoryEvent".to_string());
    }
    if (flags & 0x80000) != 0 {
        flag_list.push("LastHardLinkRemoved".to_string());
    }
    if (flags & 0x100000) != 0 {
        flag_list.push("IsHardLink".to_string());
    }
    if (flags & 0x400000) != 0 {
        flag_list.push("IsSymbolicLink".to_string());
    }
    if (flags & 0x800000) != 0 {
        flag_list.push("IsFile".to_string());
    }
    if (flags & 0x1000000) != 0 {
        flag_list.push("IsDirectory".to_string());
    }
    if (flags & 0x2000000) != 0 {
        flag_list.push("Mount".to_string());
    }
    if (flags & 0x4000000) != 0 {
        flag_list.push("Unmount".to_string());
    }
    if (flags & 0x20000000) != 0 {
        flag_list.push("EndOfTransaction".to_string());
    }
    flag_list
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::fsevents::fsevent::{
            fsevents_data, fsevents_header, get_fsevent, get_fsevent_data, match_flags,
        },
        utils::compression::decompress::decompress_gzip,
    };
    use std::{fs, path::PathBuf};

    #[test]
    fn test_match_flags() {
        let data: u32 = 11;
        let results = match_flags(&data);
        assert_eq!(results[0], "Created");
        assert_eq!(results[1], "Removed");
        assert_eq!(results[2], "Renamed");
    }

    #[test]
    fn test_fsevents_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/0000000000027d79");
        let test_path: &str = &test_location.display().to_string();
        let files = decompress_gzip(test_path).unwrap();
        let (results, data) = fsevents_data(&files, test_path).unwrap();
        assert_eq!(results.len(), 0);
        assert_eq!(data.len(), 736);
    }

    #[test]
    fn test_fsevents_headers() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/Headers/dls2header");
        let buffer = fs::read(test_location).unwrap();
        let (_, header) = fsevents_header(&buffer).unwrap();
        assert_eq!(header.signature, 1145852722);
        assert_eq!(header.stream_size, 78970);
    }

    #[test]
    fn test_get_fsevent_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/Uncompressed/0000000000027d79");
        let buffer = fs::read(test_location.clone()).unwrap();
        let (input, header) = fsevents_header(&buffer).unwrap();

        let (_, results) =
            get_fsevent_data(input, &header.signature, test_location.to_str().unwrap()).unwrap();

        assert_eq!(results.event_id, 163140);
        assert_eq!(results.path, "/Volumes/Preboot");
        assert_eq!(results.node, 0);
        assert_eq!(
            results.flags,
            ["Removed", "IsDirectory", "Mount", "Unmount"]
        );
    }

    #[test]
    fn test_get_fsevent() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/Uncompressed/0000000000027d79");
        let buffer = fs::read(test_location.clone()).unwrap();
        let (input, header) = fsevents_header(&buffer).unwrap();

        let (input, results) =
            get_fsevent(input, header.signature, test_location.to_str().unwrap()).unwrap();
        assert_eq!(results.len(), 736);
        assert_eq!(input.len(), 0);
    }

    #[test]
    fn test_fsevents_data_version3() {
        let test = [
            51, 83, 76, 68, 149, 147, 40, 64, 64, 0, 0, 0, 46, 68, 111, 99, 117, 109, 101, 110,
            116, 82, 101, 118, 105, 115, 105, 111, 110, 115, 45, 86, 49, 48, 48, 47, 46, 99, 115,
            0, 95, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 1, 234, 121, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (results, data) = fsevents_data(&test, "test").unwrap();
        assert_eq!(results.len(), 0);
        assert_eq!(data.len(), 1);
    }
}
