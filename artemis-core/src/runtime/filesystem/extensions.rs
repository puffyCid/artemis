use crate::runtime::filesystem::{
    directory::js_read_dir,
    files::{js_hash_file, js_read_text_file, js_stat},
};
use deno_core::Op;

/// Link Rust filesystem functions to `Deno core` to provide access to the filesystem
pub(crate) fn fs_runtime() -> Vec<deno_core::OpDecl> {
    vec![
        js_read_dir::DECL,
        js_stat::DECL,
        js_hash_file::DECL,
        js_read_text_file::DECL,
    ]
}
