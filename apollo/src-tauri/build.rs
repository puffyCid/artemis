use vergen::{BuildBuilder, Emitter, RustcBuilder};

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

    tauri_build::build()
}
