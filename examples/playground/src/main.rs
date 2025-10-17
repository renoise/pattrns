mod app;
mod ffi;

// -------------------------------------------------------------------------------------------------

fn main() {
    // Disabled in build.rs via `cargo::rustc-link-arg=--no-entry`
    panic!("The main function is not exported and thus should never be called");
}
