/**
 * macOS `Bookmarks` are binary data that can be used to point to another file on disk
 * They are very similar to Windows Shortcut files. `Bookmarks` are used by `Safari Downloads` and `LoginItems`
 *
 * References:  
 *   `https://mac-alias.readthedocs.io/en/latest/bookmark_fmt.html`
 *   `http://michaellynn.github.io/2015/10/24/apples-bookmarkdata-exposed/`
 *
 * Other Parsers:
 *   `https://github.com/dmgbuild/mac_alias`
 */
use super::{bookmark::BookmarkData, error::BookmarkError};
use log::error;

/// Parse provided bookmark data
pub(crate) fn parse_bookmark(data: &[u8]) -> Result<BookmarkData, BookmarkError> {
    let header_size = 48;
    if data.len() < header_size {
        error!("[bookmarks] Data size less than bookmark header size");
        return Err(BookmarkError::BadHeader);
    }

    // Read first 48 bytes of bookmark header
    let header_results = BookmarkData::parse_bookmark_header(data);
    let (bookmark_data, header) = match header_results {
        Ok((bookmark_data, header)) => (bookmark_data, header),
        Err(err) => {
            error!("[bookmarks] failed to get bookmark header: {err:?}");
            return Err(BookmarkError::BadHeader);
        }
    };
    let book_sig: u32 = 1802465122;
    let book_data_offset: u32 = 48;

    // Check for bookmark signature and expected offset
    if header.signature != book_sig || header.bookmark_data_offset != book_data_offset {
        error!("[bookmarks] Data is not a bookmark got incorrect signature/offset");
        return Err(BookmarkError::BadHeader);
    }

    let data_results = BookmarkData::parse_bookmark_data(bookmark_data);
    match data_results {
        Ok((_, bookmark_results)) => Ok(bookmark_results),
        Err(err) => {
            error!("[bookmarks] Failed to get bookmark data: {err:?}");
            Err(BookmarkError::BadBookmarkData)
        }
    }
}

#[test]
fn test_parse_bookmark() {
    let data = [
        98, 111, 111, 107, 204, 2, 0, 0, 0, 0, 4, 16, 48, 0, 0, 0, 217, 10, 110, 155, 143, 43, 6,
        0, 139, 200, 168, 230, 42, 214, 22, 102, 103, 228, 112, 159, 141, 163, 20, 27, 36, 83, 233,
        178, 57, 208, 89, 105, 200, 1, 0, 0, 4, 0, 0, 0, 3, 3, 0, 0, 0, 24, 0, 40, 5, 0, 0, 0, 1,
        1, 0, 0, 85, 115, 101, 114, 115, 0, 0, 0, 8, 0, 0, 0, 1, 1, 0, 0, 112, 117, 102, 102, 121,
        99, 105, 100, 9, 0, 0, 0, 1, 1, 0, 0, 68, 111, 119, 110, 108, 111, 97, 100, 115, 0, 0, 0,
        28, 0, 0, 0, 1, 1, 0, 0, 112, 111, 119, 101, 114, 115, 104, 101, 108, 108, 45, 55, 46, 50,
        46, 52, 45, 111, 115, 120, 45, 120, 54, 52, 46, 112, 107, 103, 16, 0, 0, 0, 1, 6, 0, 0, 16,
        0, 0, 0, 32, 0, 0, 0, 48, 0, 0, 0, 68, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 79, 83, 0, 0, 0, 0,
        0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 11, 128, 5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 62, 128,
        5, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 216, 194, 61, 2, 0, 0, 0, 0, 16, 0, 0, 0, 1, 6,
        0, 0, 128, 0, 0, 0, 144, 0, 0, 0, 160, 0, 0, 0, 176, 0, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65,
        196, 48, 15, 162, 9, 145, 58, 24, 0, 0, 0, 1, 2, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 15, 0, 0, 0,
        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 0, 0, 0, 4, 3, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 4, 0,
        0, 0, 3, 3, 0, 0, 245, 1, 0, 0, 8, 0, 0, 0, 1, 9, 0, 0, 102, 105, 108, 101, 58, 47, 47, 47,
        12, 0, 0, 0, 1, 1, 0, 0, 77, 97, 99, 105, 110, 116, 111, 115, 104, 32, 72, 68, 8, 0, 0, 0,
        4, 3, 0, 0, 0, 112, 196, 208, 209, 1, 0, 0, 8, 0, 0, 0, 0, 4, 0, 0, 65, 195, 229, 4, 81,
        128, 0, 0, 36, 0, 0, 0, 1, 1, 0, 0, 57, 54, 70, 66, 52, 49, 67, 48, 45, 54, 67, 69, 57, 45,
        52, 68, 65, 50, 45, 56, 52, 51, 53, 45, 51, 53, 66, 67, 49, 57, 67, 55, 51, 53, 65, 51, 24,
        0, 0, 0, 1, 2, 0, 0, 129, 0, 0, 0, 1, 0, 0, 0, 239, 19, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        0, 0, 1, 0, 0, 0, 1, 1, 0, 0, 47, 0, 0, 0, 0, 0, 0, 0, 1, 5, 0, 0, 204, 0, 0, 0, 254, 255,
        255, 255, 1, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 4, 16, 0, 0, 104, 0, 0, 0, 0, 0, 0, 0, 5,
        16, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 16, 16, 0, 0, 232, 0, 0, 0, 0, 0, 0, 0, 64, 16, 0, 0,
        216, 0, 0, 0, 0, 0, 0, 0, 2, 32, 0, 0, 180, 1, 0, 0, 0, 0, 0, 0, 5, 32, 0, 0, 36, 1, 0, 0,
        0, 0, 0, 0, 16, 32, 0, 0, 52, 1, 0, 0, 0, 0, 0, 0, 17, 32, 0, 0, 104, 1, 0, 0, 0, 0, 0, 0,
        18, 32, 0, 0, 72, 1, 0, 0, 0, 0, 0, 0, 19, 32, 0, 0, 88, 1, 0, 0, 0, 0, 0, 0, 32, 32, 0, 0,
        148, 1, 0, 0, 0, 0, 0, 0, 48, 32, 0, 0, 192, 1, 0, 0, 0, 0, 0, 0, 1, 192, 0, 0, 8, 1, 0, 0,
        0, 0, 0, 0, 17, 192, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 18, 192, 0, 0, 24, 1, 0, 0, 0, 0, 0, 0,
        16, 208, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0,
    ];

    let bookmark = parse_bookmark(&data).unwrap();
    let app_path_len = 4;
    let app_path = [
        "Users",
        "puffycid",
        "Downloads",
        "powershell-7.2.4-osx-x64.pkg",
    ];
    let cnid_path = [21327, 360459, 360510, 37602008];
    let volume_path = "/";
    let volume_url = "file:///";
    let volume_name = "Macintosh HD";
    let volume_uuid = "96FB41C0-6CE9-4DA2-8435-35BC19C735A3";
    let volume_size = 2000662327296;
    let volume_flag = [4294967425, 4294972399, 0];
    let volume_root = true;
    let localized_name = String::new();
    let target_flags = [1, 15, 0];
    let username = "puffycid";
    let folder_index = 2;
    let uid = 501;
    let creation_options = 671094784;
    let security_extension = String::new();

    let cnid_path_len = 4;
    let target_creation = 1655695300;
    let volume_creation = 1645859107;
    let target_flags_len = 3;

    assert_eq!(bookmark.path.len(), app_path_len);
    assert_eq!(bookmark.cnid_path.len(), cnid_path_len);
    assert_eq!(bookmark.created, target_creation);
    assert_eq!(bookmark.volume_created, volume_creation);
    assert_eq!(bookmark.target_flags.len(), target_flags_len);

    assert_eq!(bookmark.path, app_path);
    assert_eq!(bookmark.cnid_path, cnid_path);
    assert_eq!(bookmark.volume_path, volume_path);
    assert_eq!(bookmark.volume_url, volume_url);
    assert_eq!(bookmark.volume_name, volume_name);
    assert_eq!(bookmark.volume_uuid, volume_uuid);
    assert_eq!(bookmark.volume_size, volume_size);
    assert_eq!(bookmark.volume_flag, volume_flag);
    assert_eq!(bookmark.volume_root, volume_root);
    assert_eq!(bookmark.localized_name, localized_name);
    assert_eq!(bookmark.target_flags, target_flags);
    assert_eq!(bookmark.username, username);
    assert_eq!(bookmark.folder_index, folder_index);
    assert_eq!(bookmark.uid, uid);
    assert_eq!(bookmark.creation_options, creation_options);
    assert_eq!(bookmark.security_extension_rw, security_extension);
    assert_eq!(bookmark.security_extension_ro, security_extension);
    assert_eq!(bookmark.file_ref_flag, false);
}
