use crate::runtime::filesystem::{
    acquire::js_acquire_file,
    directory::js_read_dir,
    files::{js_glob, js_hash_file, js_read_file, js_read_text_file, js_stat},
};

/// Link Rust filesystem functions to `Deno core` to provide access to the filesystem
pub(crate) fn fs_runtime() -> Vec<deno_core::OpDecl> {
    vec![
        js_read_dir(),
        js_stat(),
        js_hash_file(),
        js_read_text_file(),
        js_glob(),
        js_read_file(),
        js_acquire_file(),
    ]
}
