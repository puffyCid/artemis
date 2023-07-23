use crate::runtime::environment::env::{js_env, js_env_value};
use deno_core::Op;

pub(crate) fn env_runtime() -> Vec<deno_core::OpDecl> {
    vec![js_env::DECL, js_env_value::DECL]
}
