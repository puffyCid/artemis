/**
 * Inspired by https://deno.com/blog/roll-your-own-javascript-runtime-pt3
 */
use deno_core::{include_js_files, snapshot::CreateSnapshotOptions, Extension};
use std::{env, fs::File, io::Write, path::PathBuf};

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
    let options = CreateSnapshotOptions {
        cargo_manifest_dir: env!("CARGO_MANIFEST_DIR"),
        startup_snapshot: None,
        extensions: vec![extensions],
        with_runtime_cb: Default::default(),
        skip_op_registration: false,
        extension_transpiler: None,
    };
    let snapshot_path = out.join("RUNJS_SNAPSHOT.bin");

    // Create the snapshot.
    let script_out = deno_core::snapshot::create_snapshot(options, None).unwrap();
    let mut snapshot = File::create(snapshot_path).unwrap();
    snapshot.write_all(&script_out.output).unwrap();
}
