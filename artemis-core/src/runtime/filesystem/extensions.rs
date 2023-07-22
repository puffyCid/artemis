use crate::runtime::filesystem::{directory::js_read_dir, files::js_stat};

/// Link Rust filesystem functions to `Deno core` to provide access to the filesystem
pub(crate) fn fs_runtime() -> Vec<deno_core::OpDecl> {
    vec![js_read_dir::decl(), js_stat::decl()]
}
