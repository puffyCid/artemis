use serde::Serialize;

#[derive(Serialize, Debug)]
pub(crate) struct JsFileInfo {
    pub(crate) full_path: String,
    pub(crate) directory: String,
    pub(crate) filename: String,
    pub(crate) extension: String,
    pub(crate) created: String,
    pub(crate) modified: String,
    pub(crate) changed: String,
    pub(crate) accessed: String,
    pub(crate) size: u64,
    pub(crate) inode: u64,
    pub(crate) mode: u32,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) is_file: bool,
    pub(crate) is_directory: bool,
    pub(crate) is_symlink: bool,
}
