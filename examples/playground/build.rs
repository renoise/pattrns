fn main() {
    let target = std::env::var("TARGET").expect("No TARGET env variable set");
    let profile = std::env::var("PROFILE").expect("No PROFILE env variable set");
    // inject emscripten build options
    if target.contains("emscripten") {
        // debug options
        if profile == "debug" {
            println!("cargo::rustc-link-arg=-sSAFE_HEAP=1");
            println!("cargo::rustc-link-arg=-sASSERTIONS=2");
        }
        // compile options
        println!("cargo::rustc-link-arg=-fexceptions");
        println!("cargo::rustc-link-arg=-sNO_DISABLE_EXCEPTION_CATCHING");
        println!("cargo::rustc-link-arg=-sUSE_PTHREADS=1");
        println!("cargo::rustc-link-arg=-sPTHREAD_POOL_SIZE=4");
        // memory options
        println!("cargo::rustc-link-arg=-sSTACK_SIZE=2MB");
        println!("cargo::rustc-link-arg=-sINITIAL_MEMORY=100MB");
        println!("cargo::rustc-link-arg=-sALLOW_MEMORY_GROWTH=1");
        println!("cargo::rustc-link-arg=-sMALLOC=mimalloc");
        // export options
        println!("cargo::rustc-link-arg=-sEXPORT_ES6=1");
        println!("cargo::rustc-link-arg=-sMODULARIZE");
        println!("cargo::rustc-link-arg=-sINVOKE_RUN=0");
        // exports
        println!("cargo::rustc-link-arg=--no-entry");
        let exports = [
            "_free_cstring",
            "_initialize_app",
            "_shutdown_app",
            "_start_playing",
            "_stop_playing",
            "_stop_playing_notes",
            "_midi_note_on",
            "_midi_note_off",
            "_set_volume",
            "_set_bpm",
            "_set_instrument",
            "_example_scripts",
            "_quickstart_scripts",
            "_update_script",
            "_script_error",
            "_script_parameters",
            "_set_script_parameter_value",
            "_samples",
            "_load_sample",
            "_clear_samples",
            "_mixers",
            "_available_effects",
            "_add_effect_to_mixer",
            "_move_effect_in_mixer",
            "_remove_effect_from_mixer",
            "_effect_parameter_string",
            "_set_effect_parameter_value",
        ];
        println!(
            "cargo::rustc-link-arg=-sEXPORTED_FUNCTIONS={}",
            exports.join(",")
        );
        println!("cargo::rustc-link-arg=-sEXPORTED_RUNTIME_METHODS=ccall,UTF8ToString");
        // assets
        println!(
            "cargo::rustc-link-arg=--preload-file={}/assets@/assets",
            std::env::var("CARGO_MANIFEST_DIR").unwrap()
        );
    } else {
        println!("cargo::warning=This example only works with target 'wasm32-unknown-emscripten'")
    }
}
