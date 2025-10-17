use std::{cell::RefCell, ffi, rc::Rc};

use emscripten_rs_sys::{emscripten_request_animation_frame_loop, emscripten_run_script};

use pattrns::prelude::*;
use serde::ser::SerializeStruct;

use crate::app::App;

// -------------------------------------------------------------------------------------------------

// We're called from single thread in JS only, thus we can avoid using Mutex or other RWLocks
// which actually would require using atomics in the WASM.
thread_local!( //
    static APP: RefCell<Option<App>> = const { RefCell::new(None) }
);

/// Helper function to safely access the thread-local App state immutably.
fn with_app<F, R>(f: F) -> R
where
    F: FnOnce(&App) -> R,
    R: Default,
{
    APP.with_borrow(|app| {
        if let Some(app) = app {
            f(app)
        } else {
            R::default()
        }
    })
}

/// Helper function to safely access the thread-local App state mutably.
fn with_app_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut App) -> R,
    R: Default,
{
    APP.with_borrow_mut(|app| {
        if let Some(app) = app {
            f(app)
        } else {
            R::default()
        }
    })
}

// -------------------------------------------------------------------------------------------------

/// Single sample asset, passed as JSON to the frontend.
#[derive(serde::Serialize)]
pub struct SampleEntry {
    pub name: String,
    pub id: usize,
}

/// Single example script content section, passed as JSON to the frontend.
#[derive(serde::Serialize)]
pub struct ScriptSection {
    pub name: String,
    pub entries: Vec<ScriptEntry>,
}

/// Single example script asset, passed as JSON to the frontend.
#[derive(serde::Serialize)]
pub struct ScriptEntry {
    pub name: String,
    pub content: String,
}

/// Single script parameter, passed as JSON to the frontend.
#[derive(Clone, PartialEq)]
pub struct ScriptParameter(pub Rc<RefCell<Parameter>>);

impl serde::Serialize for ScriptParameter {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let parameter = self.0.borrow();

        let mut s = serializer.serialize_struct("Parameter", 8)?;
        s.serialize_field("id", parameter.id())?;
        s.serialize_field("name", parameter.name())?;
        s.serialize_field("description", parameter.description())?;
        let parameter_type = {
            match &parameter.parameter_type() {
                ParameterType::Boolean => "boolean",
                ParameterType::Float => "float",
                ParameterType::Integer => "integer",
                ParameterType::Enum => "enum",
            }
        };
        s.serialize_field("type", &parameter_type)?;
        s.serialize_field("range", parameter.range())?;
        s.serialize_field("default", &parameter.default())?;
        s.serialize_field("value", &parameter.value())?;
        s.serialize_field("value_strings", parameter.value_strings())?;
        s.end()
    }
}

impl From<&Rc<RefCell<pattrns::Parameter>>> for ScriptParameter {
    fn from(value: &Rc<RefCell<pattrns::Parameter>>) -> Self {
        Self(Rc::clone(value))
    }
}

/// Mixer info for JSON serialization
#[derive(serde::Serialize)]
pub struct MixerInfo {
    pub id: MixerId,
    pub name: String,
    pub instrument_id: Option<usize>,
    pub effects: Vec<EffectInfo>,
}

/// Effect info for JSON serialization
#[derive(serde::Serialize)]
pub struct EffectInfo {
    pub id: EffectId,
    pub name: String,
    pub parameters: Vec<EffectParameterInfo>,
}

/// Effect parameter info for JSON serialization
#[derive(serde::Serialize)]
pub struct EffectParameterInfo {
    pub id: u32,
    pub name: String,
    #[serde(rename = "type")]
    pub param_type: String,
    pub default: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
}

// -------------------------------------------------------------------------------------------------

/// Creates a new global `App` state.
#[no_mangle]
pub extern "C" fn initialize_app() -> *const ffi::c_char {
    // create or recreate the player instance
    println!("Creating new player instance...");
    match App::new() {
        Err(err) => {
            eprintln!("Failed to create player instance: {}", err);
            APP.replace(None);
            unsafe { new_raw_cstring(&err.to_string()) }
        }
        Ok(player) => {
            println!("Successfully created a new player instance");
            APP.replace(Some(player));
            unsafe {
                println!("Start running...");
                emscripten_request_animation_frame_loop(Some(run_app), std::ptr::null_mut())
            };
            std::ptr::null()
        }
    }
}

/// Destroys the global `App` state.
#[no_mangle]
pub extern "C" fn shutdown_app() {
    // drop the player instance
    println!("Dropping player instance...");
    APP.replace(None);
}

/// Emscripten animation frame callback that drives the audio playback.
/// Returns 1 to continue running or 0 to stop if the app is not available.
extern "C" fn run_app(_time: f64, _user_data: *mut ffi::c_void) -> bool {
    APP.with_borrow_mut(|player| {
        if let Some(playground) = player {
            playground.run();
            true // continue
        } else {
            false // stop
        }
    })
}

/// Start playback.
#[no_mangle]
pub extern "C" fn start_playing() {
    with_app_mut(|playground| playground.start_playing());
}

/// Stop playback.
#[no_mangle]
pub extern "C" fn stop_playing() {
    with_app_mut(|playground| playground.stop_playing());
}

/// Stop all playing notes.
#[no_mangle]
pub extern "C" fn stop_playing_notes() {
    with_app_mut(|playground| playground.stop_playing_notes());
}

/// Set new global volume factor.
#[no_mangle]
pub extern "C" fn set_volume(volume: f32) {
    with_app_mut(|playground| playground.set_volume(volume));
}

/// Handle note on event from the frontend
#[no_mangle]
pub extern "C" fn midi_note_on(note: u8, velocity: u8) {
    with_app_mut(|playground| playground.handle_midi_note_on(note, velocity));
}

/// Handle note off event from the frontend
#[no_mangle]
pub extern "C" fn midi_note_off(note: u8) {
    with_app_mut(|playground| playground.handle_midi_note_off(note));
}

/// Update player's BPM.
#[no_mangle]
pub extern "C" fn set_bpm(bpm: ffi::c_int) {
    with_app_mut(|playground| playground.set_bpm(bpm as f32));
}

/// Update player's default instrument id.
#[no_mangle]
pub extern "C" fn set_instrument(id: ffi::c_int) {
    with_app_mut(|playground| playground.set_instrument(id));
}

/// Returns example script names and contents as json string.
#[no_mangle]
pub unsafe extern "C" fn example_scripts() -> *const ffi::c_char {
    let example_scripts = App::example_scripts().unwrap();
    new_raw_cstring(&serde_json::to_string(&example_scripts).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn quickstart_scripts() -> *const ffi::c_char {
    let quickstart_scripts = App::quickstart_scripts().unwrap();
    new_raw_cstring(&serde_json::to_string(&quickstart_scripts).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn update_script(content_ptr: *const ffi::c_char) {
    let content = unsafe {
        ffi::CStr::from_ptr(content_ptr)
            .to_string_lossy()
            .into_owned()
    };
    with_app_mut(|playground| playground.update_script_content(content));
}

/// Returns actual script runtime errors, if any
#[no_mangle]
pub unsafe extern "C" fn script_error() -> *const ffi::c_char {
    let string = with_app(|playground| playground.script_error().to_string());
    new_raw_cstring(&string)
}

/// Returns actual script parameters, if any
#[no_mangle]
pub unsafe extern "C" fn script_parameters() -> *const ffi::c_char {
    let parameters = with_app(|playground| playground.script_parameters().to_vec());
    new_raw_cstring(&serde_json::to_string(&parameters).unwrap())
}

/// Set a script parameter value.
#[no_mangle]
pub unsafe extern "C" fn set_script_parameter_value(id_ptr: *const ffi::c_char, value: f64) {
    let id = ffi::CStr::from_ptr(id_ptr).to_string_lossy().into_owned();
    with_app_mut(|playground| playground.set_script_parameter_value(&id, value));
}

/// Returns available sample names and ids as json string.
#[no_mangle]
pub unsafe extern "C" fn samples() -> *const ffi::c_char {
    let json = with_app(|playground| serde_json::to_string(&playground.samples()).unwrap());
    new_raw_cstring(&json)
}

/// Load a sample from a file buffer.
#[no_mangle]
pub unsafe extern "C" fn load_sample(
    filename_ptr: *const ffi::c_char,
    buffer_ptr: *const u8,
    buffer_len: usize,
) -> ffi::c_int {
    let file_buffer = std::slice::from_raw_parts(buffer_ptr, buffer_len).to_vec();
    let file_name = ffi::CStr::from_ptr(filename_ptr)
        .to_string_lossy()
        .into_owned();
    with_app_mut(
        |playground| match playground.load_sample(file_buffer, &file_name) {
            Ok(id) => {
                println!("Loaded sample '{}' with id {}", file_name, id);
                id as ffi::c_int
            }
            Err(err) => {
                eprintln!("Failed to load sample '{}': {}", file_name, err);
                -1
            }
        },
    )
}

/// Clears all loaded samples.
#[no_mangle]
pub extern "C" fn clear_samples() {
    with_app_mut(|playground| {
        playground.clear_samples();
    });
}

/// Returns all mixers with their effects as JSON
#[no_mangle]
pub unsafe extern "C" fn mixers() -> *const ffi::c_char {
    let mixers = with_app(|playground| playground.mixers());
    new_raw_cstring(&serde_json::to_string(&mixers).unwrap())
}

/// Returns list of available effects as JSON
#[no_mangle]
pub unsafe extern "C" fn available_effects() -> *const ffi::c_char {
    let effects = App::available_effects();
    new_raw_cstring(&serde_json::to_string(&effects).unwrap())
}

/// Add effect to mixer. Returns JSON with effect ID and parameters or null on error.
#[no_mangle]
pub unsafe extern "C" fn add_effect_to_mixer(
    mixer_id: ffi::c_int,
    effect_name_ptr: *const ffi::c_char,
) -> *const ffi::c_char {
    let effect_name = ffi::CStr::from_ptr(effect_name_ptr)
        .to_string_lossy()
        .into_owned();
    with_app_mut(|playground| {
        match playground.add_effect_by_name(mixer_id as pattrns::prelude::MixerId, &effect_name) {
            Ok((effect_id, params)) => {
                let result = serde_json::json!({
                    "effectId": effect_id,
                    "params": params
                });
                new_raw_cstring(&result.to_string())
            }
            Err(err) => {
                eprintln!("Failed to add effect: {}", err);
                std::ptr::null()
            }
        }
    })
}

/// Move effect within mixer's effect chain
#[no_mangle]
pub extern "C" fn move_effect_in_mixer(
    effect_id: ffi::c_int,
    mixer_id: ffi::c_int,
    direction: ffi::c_int,
) -> ffi::c_int {
    with_app_mut(|playground| {
        match playground.move_effect(
            effect_id as pattrns::prelude::EffectId,
            mixer_id as pattrns::prelude::MixerId,
            direction,
        ) {
            Ok(_) => 0,
            Err(err) => {
                eprintln!("Failed to move effect: {}", err);
                -1
            }
        }
    })
}

/// Remove effect from mixer
#[no_mangle]
pub extern "C" fn remove_effect_from_mixer(effect_id: ffi::c_int) -> ffi::c_int {
    with_app_mut(|playground| {
        match playground.remove_effect(effect_id as pattrns::prelude::EffectId) {
            Ok(_) => 0,
            Err(err) => {
                eprintln!("Failed to remove effect: {}", err);
                -1
            }
        }
    })
}

/// Get effect parameter value as string
#[no_mangle]
pub unsafe extern "C" fn effect_parameter_string(
    effect_id: ffi::c_int,
    param_id: ffi::c_uint,
    normalized_value: ffi::c_float,
) -> *const ffi::c_char {
    with_app(|playground| {
        match playground.effect_parameter_string(
            effect_id as pattrns::prelude::EffectId,
            param_id,
            normalized_value,
        ) {
            Ok(value_string) => new_raw_cstring(&value_string),
            Err(err) => {
                eprintln!("Failed to get parameter string: {}", err);
                std::ptr::null()
            }
        }
    })
}

/// Set effect parameter value
#[no_mangle]
pub extern "C" fn set_effect_parameter_value(
    effect_id: ffi::c_int,
    param_id: ffi::c_uint,
    value: ffi::c_float,
) -> ffi::c_int {
    with_app_mut(|playground| {
        match playground.set_effect_parameter_value(
            effect_id as pattrns::prelude::EffectId,
            param_id,
            value,
        ) {
            Ok(_) => 0,
            Err(err) => {
                eprintln!("Failed to set parameter value: {}", err);
                -1
            }
        }
    })
}

/// Frees a string ptr which got passed to JS after it got consumed.
#[no_mangle]
pub unsafe extern "C" fn free_cstring(ptr: *mut ffi::c_char) {
    drop_raw_cstring(ptr);
}

// -------------------------------------------------------------------------------------------------

/// Helper function to create a new raw CString from strings which may contain inner \0 chars.
unsafe fn new_raw_cstring(str: &str) -> *mut ffi::c_char {
    if str.contains('\0') {
        ffi::CString::from_vec_unchecked(str.replace('\0', "\\0").into()).into_raw()
    } else {
        ffi::CString::from_vec_unchecked(str.into()).into_raw()
    }
}

/// Helper function to drop a string created with `new_raw_cstring`
unsafe fn drop_raw_cstring(chars: *const ffi::c_char) {
    if !chars.is_null() {
        drop(ffi::CString::from_raw(chars as *mut ffi::c_char))
    }
}

// -------------------------------------------------------------------------------------------------

/// Call the given `window.$NOTIFIER` function in the frontend
pub unsafe fn call_frontend_notifier(notifier_name: &str) {
    // NB: async to avoid that JS is calling back into rust while the playground ref is borrowed
    let ptr = new_raw_cstring(format!("window.setTimeout(window.{}, 0)", notifier_name).as_str());
    emscripten_run_script(ptr);
    free_cstring(ptr);
}
