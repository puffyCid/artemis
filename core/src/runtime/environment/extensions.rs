use crate::runtime::environment::env::{js_env, js_env_value};

/// Link Rust environment functions to `Deno core` to provide access to the artemis environment variables
pub(crate) fn env_runtime() -> Vec<deno_core::OpDecl> {
    vec![js_env(), js_env_value()]
}
