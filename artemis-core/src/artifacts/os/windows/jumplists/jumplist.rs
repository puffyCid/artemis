use crate::artifacts::os::windows::shortcuts::shortcut::ShortcutInfo;

#[derive(Debug)]
pub(crate) struct JumplistEntry {
    pub(crate) lnk_info: ShortcutInfo,
    pub(crate) path: String,
    pub(crate) jumplist_type: ListType,
    pub(crate) version: u32,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ListType {
    Automatic,
    Custom,
}
