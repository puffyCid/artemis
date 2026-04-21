use std::process::Command;
use vergen::{BuildBuilder, Emitter, RustcBuilder};

// Include some additional compile time info in our binary
fn main() {
    let build = BuildBuilder::all_build().unwrap();
    let rustc = RustcBuilder::all_rustc().unwrap();

    Emitter::default()
        .add_instructions(&build)
        .unwrap()
        .add_instructions(&rustc)
        .unwrap()
        .emit()
        .unwrap();

    println!("cargo:rerun-if-changed=../.git/HEAD");

    let git_hash = if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if !output.status.success() {
            String::from("Missing Git Commit")
        } else {
            String::from_utf8(output.stdout).unwrap().trim().to_string()
        }
    } else {
        String::from("Missing Git")
    };

    println!("cargo:rustc-env=GIT_HASH={git_hash}");

    let mut features = Vec::new();
    // Cargo sets an env var for every enabled feature (e.g., CARGO_FEATURE_MY_FEATURE)
    for (key, _) in std::env::vars() {
        if let Some(feature) = key.strip_prefix("CARGO_FEATURE_") {
            features.push(feature.to_lowercase());
        }
    }
    // Pass the list to your main code as a compile-time env var
    println!("cargo:rustc-env=ENABLED_FEATURES={}", features.join(", "));

    // Read the TARGET variable provided by Cargo
    let target = std::env::var("TARGET").unwrap();

    // Tell Cargo to set a compile-time env variable for your main code
    println!("cargo:rustc-env=COMPILE_TARGET={}", target);

    // Read the PROFILE env var set by Cargo
    let profile = std::env::var("PROFILE").unwrap();

    // Optional: Pass it to your code as a custom environment variable
    println!("cargo:rustc-env=BUILD_PROFILE={}", profile);
}
