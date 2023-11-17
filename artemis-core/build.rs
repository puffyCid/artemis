/**
 * Inspired by https://deno.com/blog/roll-your-own-javascript-runtime-pt3
 */
use deno_core::{include_js_files, Extension};
use std::{env, path::PathBuf};

/// Create a SnapShot at build time to help speed up our JavaScript Runtime
fn main() {
    let extensions = Extension {
        esm_files: include_js_files!(artemis 
        "javascript/console.js",
        "javascript/filesystem.js",
        "javascript/environment.js",
        "javascript/encoding.js",
        "javascript/system.js",
        "javascript/time.js",
        "javascript/http.js",
        "javascript/main.js",)
        .to_vec()
        .into(),
        esm_entry_point: Some("ext:artemis/javascript/main.js"),
        ..Default::default()
    };

    // Build the file path to the snapshot.
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let snapshot_path = out.join("RUNJS_SNAPSHOT.bin");

    // Create the snapshot.
    let _ = deno_core::snapshot_util::create_snapshot(
        deno_core::snapshot_util::CreateSnapshotOptions {
            cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
            snapshot_path,
            startup_snapshot: None,
            extensions: vec![extensions],
            compression_cb: None,
            with_runtime_cb: Default::default(),
            skip_op_registration: false,
        },
    );
}
