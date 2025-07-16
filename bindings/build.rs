use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").expect("Failed to get cargo manifest dir");
    cbindgen::generate(crate_dir)
        .expect("Failed to generate bindings")
        .write_to_file("includes/pattrns.h");
}
