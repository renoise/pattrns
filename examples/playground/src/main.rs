#![allow(clippy::missing_safety_doc)]

use std::{
    cell::RefCell, collections::HashMap, ffi, fs, path::Path, rc::Rc, sync::Arc, time::Duration,
};

use emscripten_rs_sys::{emscripten_request_animation_frame_loop, emscripten_run_script};
use serde::ser::SerializeStruct;

use pattrns::prelude::*;

// -------------------------------------------------------------------------------------------------

// We're called from single thread in JS only, thus we can avoid using Mutex or other RWLocks
// which actually would require using atomics in the WASM.
thread_local!( //
    static PLAYGROUND: RefCell<Option<Playground>> = const { RefCell::new(None) }
);

/// Helper function to safely access the thread-local Playground state immutably.
fn with_playground<F, R>(f: F) -> R
where
    F: FnOnce(&Playground) -> R,
    R: Default,
{
    PLAYGROUND.with_borrow(|player| {
        if let Some(playground) = player {
            f(playground)
        } else {
            R::default()
        }
    })
}

/// Helper function to safely access the thread-local Playground state mutably.
fn with_playground_mut<F, R>(f: F) -> R
where
    F: FnOnce(&mut Playground) -> R,
    R: Default,
{
    PLAYGROUND.with_borrow_mut(|player| {
        if let Some(playground) = player {
            f(playground)
        } else {
            R::default()
        }
    })
}

// -------------------------------------------------------------------------------------------------

/// Single sample asset, passed as JSON to the frontend.
#[derive(serde::Serialize)]
struct SampleEntry {
    name: String,
    id: usize,
}

/// Single example script content section, passed as JSON to the frontend.
#[derive(serde::Serialize)]
struct ScriptSection {
    name: String,
    entries: Vec<ScriptEntry>,
}

/// Single example script asset, passed as JSON to the frontend.
#[derive(serde::Serialize)]
struct ScriptEntry {
    name: String,
    content: String,
}

/// Single script parameter, passed as JSON to the frontend.

#[derive(Clone, PartialEq)]
struct ScriptParameter(Rc<RefCell<Parameter>>);

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

/// Single pattern triggered by a MIDI note
#[derive(Clone)]
struct PlayingNote {
    note: u8,
    velocity: u8,
    sample_offset: SampleTime,
}

/// The backend's global app state.
struct Playground {
    playing: bool,
    player: SamplePlayer,
    sample_pool: Arc<SamplePool>,
    samples: Vec<SampleEntry>,
    sequence: Option<Sequence>,
    pattern: Option<Rc<RefCell<dyn Pattern>>>,
    time_base: BeatTimeBase,
    time_base_changed: bool,
    instrument_id: Option<usize>,
    script_content: String,
    script_changed: bool,
    script_parameters: Vec<ScriptParameter>,
    script_parameter_values: HashMap<String, f64>,
    script_error: String,
    playing_notes: Vec<PlayingNote>,
    output_start_sample_time: u64,
    emitted_sample_time: u64,
}

impl Playground {
    // Event scheduler read-ahead time (latency)
    const PLAYBACK_PRELOAD_SECONDS: f64 = if cfg!(debug_assertions) { 0.5 } else { 0.25 };
    // Max expected MIDI notes
    const NUM_MIDI_NOTES: usize = 127;
    // Path to our assets folder. see build.rs.
    const ASSETS_PATH: &str = "/assets";

    /// Creates a new Playground instance with initialized state.
    /// Returns an error if initialization fails at any step.
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // load samples
        println!("Loading sample files...");
        let mut samples = Vec::new();
        let sample_pool = Arc::new(SamplePool::new());
        for dir_entry in fs::read_dir(format!("{}/samples", Self::ASSETS_PATH))?.flatten() {
            let path = dir_entry.path();
            if let Some(extension) = path.extension().map(|e| e.to_string_lossy()) {
                if matches!(extension.as_bytes(), b"mp3" | b"wav" | b"flac") {
                    let id = usize::from(sample_pool.load_sample(&path)?);
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    println!("Added sample '{}' with id {}", name, id);
                    samples.push(SampleEntry { id, name });
                }
            }
        }

        // create and configure sample player
        println!("Creating audio player...");
        let playing = false;
        let mut player = SamplePlayer::new(Arc::clone(&sample_pool), None)?;
        player.set_sample_root_note(Note::C4);
        player.set_new_note_action(NewNoteAction::Off(Some(Duration::from_millis(350))));
        player.set_playback_preload_time(Duration::from_secs_f64(Self::PLAYBACK_PRELOAD_SECONDS));

        // sequence & pattern
        let sequence = None;
        let pattern = None;

        // time base
        let time_base = BeatTimeBase {
            beats_per_min: 120.0,
            beats_per_bar: 4,
            samples_per_sec: player.sample_rate(),
        };
        let time_base_changed = false;

        // script content
        let script_content = "return pattern { }".to_string();
        let script_changed = true;
        let script_parameters = Vec::new();
        let script_parameter_values = HashMap::new();
        let script_error = String::new();

        // MIDI note playback
        let playing_notes = Vec::new();

        // default instrument
        let instrument_id = samples.first().map(|e| e.id);

        // playback time
        let output_start_sample_time = player.inner().output_sample_frame_position();
        let emitted_sample_time = 0;

        // install emscripten frame timer
        unsafe {
            println!("Start running...");
            emscripten_request_animation_frame_loop(Some(Self::run_frame), std::ptr::null_mut())
        };

        Ok(Self {
            player,
            playing,
            sample_pool,
            samples,
            sequence,
            pattern,
            time_base,
            time_base_changed,
            script_content,
            script_changed,
            script_parameters,
            script_parameter_values,
            script_error,
            playing_notes,
            instrument_id,
            output_start_sample_time,
            emitted_sample_time,
        })
    }

    // read examples from the file system into a vector of ExampleScriptEntry
    pub fn example_scripts() -> Result<Vec<ScriptEntry>, Box<dyn std::error::Error>> {
        let mut example_entries = Vec::new();
        let example_paths = fs::read_dir(format!("{}/examples", Self::ASSETS_PATH))?;
        for example in example_paths.flatten() {
            let path = example.path();
            let mut name = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            name = name
                .trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ')
                .to_string();
            let content = fs::read_to_string(&path)?;
            example_entries.push(ScriptEntry { name, content });
        }
        Ok(example_entries)
    }

    // read quickstart examples from the file system into a vector of ExampleScriptSection
    pub fn quickstart_scripts() -> Result<Vec<ScriptSection>, Box<dyn std::error::Error>> {
        let mut quickstart_scripts = Vec::new();
        let section_paths = fs::read_dir(format!("{}/quickstart", Self::ASSETS_PATH))?;
        for section_path in section_paths.flatten() {
            if section_path.metadata()?.is_dir() {
                let mut section_name = section_path.file_name().to_string_lossy().to_string();
                section_name = section_name
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ')
                    .to_string();
                let mut section_entries = Vec::new();
                let script_paths = fs::read_dir(section_path.path())?;
                for script_path in script_paths {
                    let script_path = script_path?;
                    if script_path.metadata()?.is_file() {
                        let mut script_name = script_path
                            .path()
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string();
                        script_name = script_name
                            .trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ')
                            .to_string();
                        let script_content = fs::read_to_string(script_path.path())?;
                        section_entries.push(ScriptEntry {
                            name: script_name.to_string(),
                            content: script_content,
                        });
                    }
                }
                if !section_entries.is_empty() {
                    quickstart_scripts.push(ScriptSection {
                        name: section_name.to_string(),
                        entries: section_entries,
                    });
                }
            }
        }
        Ok(quickstart_scripts)
    }

    /// Starts playback of the current sequence.
    pub fn start_playing(&mut self) {
        if !self.playing {
            // reset play head
            self.output_start_sample_time = self.player.inner().output_sample_frame_position();
            self.emitted_sample_time = 0;
            // reset sequence
            if let Some(sequence) = self.sequence.as_mut() {
                sequence.reset();
            }
            // start playback
            self.playing = true;
        }
    }

    /// Stops all currently playing audio sources and resets the sequence.
    pub fn stop_playing(&mut self) {
        let _ = self.player.stop_all_sources();
        self.playing = false;
    }

    /// Stops all currently playing audio sources.
    pub fn stop_playing_notes(&mut self) {
        let _ = self.player.stop_all_sources();
    }

    /// Set global playback volume.
    pub fn set_volume(&mut self, volume: f32) {
        self.player.inner_mut().set_output_volume(volume);
    }

    /// Handle incoming MIDI note on event
    pub fn handle_midi_note_on(&mut self, note: u8, velocity: u8) {
        assert!(note as usize <= Self::NUM_MIDI_NOTES);
        if self.playing_notes.is_empty() || self.pattern_slot(note as usize).is_none() {
            // reset play head
            self.output_start_sample_time = self.player.inner().output_sample_frame_position();
            self.emitted_sample_time = 0;
            // memorize playing note
            let sample_offset = 0;
            let new_note = PlayingNote {
                note,
                velocity,
                sample_offset,
            };
            self.playing_notes.push(new_note);
            // rebuild sequence
            self.script_changed = true;
        } else {
            // memorize playing note
            let playback_preload = self
                .time_base
                .seconds_to_samples(self.player.playback_preload_time().as_secs_f64());
            let sample_offset = self.emitted_sample_time.saturating_sub(playback_preload);
            let new_note = PlayingNote {
                note,
                velocity,
                sample_offset,
            };
            self.playing_notes.push(new_note.clone());
            // add a new pattern for the new note
            let pattern = self
                .pattern
                .as_ref()
                .expect("Expecting a valid pattern instance when notes are playing");
            let new_pattern = self.new_pattern_instance(pattern, Some(new_note));
            let pattern_slot = self
                .pattern_slot(note as usize)
                .expect("Missing MIDI pattern slot");
            *pattern_slot = PatternSlot::Pattern(new_pattern);
        }
    }

    /// Handle incoming MIDI note off event
    pub fn handle_midi_note_off(&mut self, note: u8) {
        assert!(note as usize <= Self::NUM_MIDI_NOTES);
        // ony handle off events when we got an on event
        if let Some((playing_notes_index, _)) = self
            .playing_notes
            .iter()
            .enumerate()
            .find(|(_, n)| n.note == note)
        {
            // remove playing note
            self.playing_notes.remove(playing_notes_index);
            // remove the pattern slot from sequence's phrase
            if let Some(pattern_slot) = self.pattern_slot(note as usize) {
                *pattern_slot = PatternSlot::Stop;
                // stop pending from the note
                self.player.stop_sources_in_pattern_slot(note as usize);
            }
            // restore default playback in `run` with the last note removed
            if self.playing_notes.is_empty() {
                let _ = self.player.stop_all_sources();
                self.script_changed = true;
            }
        }
    }

    /// Updates the tempo (beats per minute) of playback.
    pub fn set_bpm(&mut self, bpm: f32) {
        self.time_base.beats_per_min = bpm;
        self.time_base_changed = true;
    }

    /// Sets the default instrument ID for playback.
    pub fn set_instrument(&mut self, id: i32) {
        self.instrument_id = if id < 0 { None } else { Some(id as usize) };
        self.script_changed = true;
    }

    /// Sets a script parameter value.
    pub fn set_parameter_value(&mut self, id: &str, value: f64) {
        self.script_parameter_values.insert(id.to_owned(), value);
        if let Some(pattern) = &mut self.pattern {
            if let Some(parameter) = pattern
                .borrow()
                .parameters()
                .iter()
                .find(|p| p.borrow().id() == id)
            {
                parameter.borrow_mut().set_value(value);
            }
        }
    }

    /// Updates the script content and marks it as changed to trigger recompilation.
    pub fn update_script_content(&mut self, content: String) {
        if self.script_content != content {
            self.script_content = content;
            self.script_changed = true;
        }
    }

    /// Load a sample from a raw file buffer and add it to the pool
    pub fn load_sample(&mut self, file_buffer: Vec<u8>, file_name: &str) -> Result<usize, String> {
        match self.sample_pool.load_sample_buffer(file_buffer, file_name) {
            Ok(instrument_id) => {
                let id = usize::from(instrument_id);
                let name = Path::new(&file_name)
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                self.samples.push(SampleEntry { name, id });
                Ok(id)
            }
            Err(err) => Err(err.to_string()),
        }
    }

    /// Reset sample pool, removing all samples
    pub fn clear_samples(&mut self) {
        // Clear the sample pool
        self.sample_pool.clear();
        self.samples.clear();
        // Reset the current instrument
        self.instrument_id = None;
        self.script_changed = true;
    }

    /// Emscripten animation frame callback that drives the audio playback.
    /// Returns 1 to continue running or 0 to stop if Playground is not available.
    extern "C" fn run_frame(_time: f64, _user_data: *mut ffi::c_void) -> bool {
        PLAYGROUND.with_borrow_mut(|player| {
            if let Some(playground) = player {
                playground.run();
                true // continue
            } else {
                false // stop
            }
        })
    }

    /// Main playback loop: Handles player state updates and runs the player
    fn run(&mut self) {
        // apply script content changes
        if self.script_changed || self.sequence.is_none() {
            self.rebuild_sequence();
        }

        // apply time base changes
        if self.time_base_changed {
            self.rebuild_time_base();
        }

        // verify internal state
        debug_assert!(
            !self.script_changed
                && !self.time_base_changed
                && self.sequence.is_some()
                && self.pattern.is_some(),
            "Should have a valid sequence and pattern here"
        );

        // check if audio output has been suspended by the browser
        let suspended = self.player.inner().output_suspended();

        // run the player, when playing and audio output is not suspended
        if !suspended && (self.playing || !self.playing_notes.is_empty()) {
            // calculate samples to emit
            let samples_to_emit = self.player.calculate_samples_to_emit(
                &self.time_base,
                self.output_start_sample_time,
                self.emitted_sample_time,
            );
            let playback_preload = self
                .time_base
                .seconds_to_samples(self.player.playback_preload_time().as_secs_f64());
            if samples_to_emit > 4 * playback_preload {
                // we lost too much time: maybe because the browser suspended the run loop
                self.player.advance_until_time(
                    self.sequence.as_mut().unwrap(),
                    self.emitted_sample_time + samples_to_emit,
                );
            } else if samples_to_emit > 0 {
                // continue generating events in real-time
                self.player.run_until_time(
                    self.sequence.as_mut().unwrap(),
                    self.output_start_sample_time,
                    self.emitted_sample_time + samples_to_emit,
                );
                // handle runtime errors
                if let Some(err) = pattrns::bindings::has_lua_callback_errors() {
                    self.update_script_error(&err.to_string());
                    pattrns::bindings::clear_lua_callback_errors();
                }
            }
            self.emitted_sample_time += samples_to_emit;
        }
    }

    // Rebuild sequence and pattern from actual script content
    fn rebuild_sequence(&mut self) {
        // clear runtime errors
        pattrns::bindings::clear_lua_callback_errors();
        // build pattern and set compile errors and parameters
        let (pattern, error) = self.new_pattern();
        self.update_script_error(&error);
        self.update_script_parameters(
            &pattern
                .borrow()
                .parameters()
                .iter()
                .map(ScriptParameter::from)
                .collect::<Vec<_>>(),
        );
        // restore previous parameter values in new pattern
        for (id, value) in &self.script_parameter_values {
            if let Some(parameter) = pattern
                .borrow()
                .parameters()
                .iter()
                .find(|p| p.borrow().id() == id)
            {
                let clamped_value = value.clamp(
                    *parameter.borrow().range().start(),
                    *parameter.borrow().range().end(),
                );
                parameter.borrow_mut().set_value(clamped_value);
            }
        }
        // build pattern slots
        let pattern_slots = {
            if !self.playing_notes.is_empty() {
                // one pattern for each live played note
                let mut slots = vec![PatternSlot::Stop; Self::NUM_MIDI_NOTES];
                for playing_note in &self.playing_notes.clone() {
                    slots[playing_note.note as usize] = PatternSlot::Pattern(
                        self.new_pattern_instance(&pattern, Some(playing_note.clone())),
                    );
                }
                slots
            } else {
                // a single pattern slot for regular playback
                vec![PatternSlot::Pattern(Rc::clone(&pattern))]
            }
        };
        // replace pattern and sequence
        let mut sequence = Sequence::new(
            self.time_base,
            vec![Phrase::new(
                self.time_base,
                pattern_slots,
                BeatTimeStep::Bar(4.0),
            )],
        );
        self.player.prepare_run_until_time(
            self.sequence.take().as_mut(),
            &mut sequence,
            self.output_start_sample_time,
            self.emitted_sample_time,
        );
        self.sequence.replace(sequence);
        self.pattern.replace(pattern);
        // reset all update flags: we're fully up to date now.
        self.script_changed = false;
        self.time_base_changed = false;
    }

    // Rebuild sequence time base from actual timer base
    fn rebuild_time_base(&mut self) {
        self.time_base_changed = false;
        if let Some(sequence) = &mut self.sequence {
            sequence.set_time_base(&self.time_base);
        }
    }

    /// Update script error internally and in frontend if needed
    fn update_script_error(&mut self, error: &str) {
        if self.script_error != error {
            self.script_error = error.to_string();
            unsafe {
                call_frontend_notifier("on_script_error_changed");
            }
        }
    }

    /// Update script parameters internally and in frontend if needed
    fn update_script_parameters(&mut self, parameters: &[ScriptParameter]) {
        let parameters_changed = self.script_parameters != parameters;
        // memorize new parameters
        self.script_parameters = parameters.to_vec();
        self.script_parameter_values.retain(|id, _| {
            self.script_parameters
                .iter()
                .any(|p| p.0.borrow().id() == id)
        });
        if parameters_changed {
            unsafe {
                call_frontend_notifier("on_script_parameters_changed");
            }
        }
    }

    /// Access a pattern slot by index. pattern_index is used as MIDI note number.
    fn pattern_slot(&mut self, pattern_index: usize) -> Option<&mut PatternSlot> {
        if let Some(sequence) = &mut self.sequence {
            let phrase = sequence
                .phrases_mut()
                .first_mut()
                .expect("Failed to access phrase");
            phrase.pattern_slots_mut().get_mut(pattern_index)
        } else {
            None
        }
    }

    /// Create a new pattern from the currently set script content.
    fn new_pattern(&self) -> (Rc<RefCell<dyn Pattern>>, String) {
        // create a new pattern from our script
        match new_pattern_from_string(
            self.time_base,
            self.instrument_id.map(InstrumentId::from),
            &self.script_content,
            "[script]",
        ) {
            Ok(pattern) => {
                // return pattern as it is
                (pattern, String::new())
            }
            Err(err) => {
                // create an empty fallback pattern on errors
                (
                    Rc::new(RefCell::new(BeatTimePattern::new(
                        self.time_base,
                        BeatTimeStep::Beats(1.0),
                    ))),
                    err.to_string(),
                )
            }
        }
    }

    /// Create a new pattern instance clone for the given note from the passed pattern
    /// for the given optional midi note for note transforms.
    fn new_pattern_instance(
        &self,
        pattern: &Rc<RefCell<dyn Pattern>>,
        midi_note: Option<PlayingNote>,
    ) -> Rc<RefCell<dyn Pattern>> {
        // create a new pattern clone
        let pattern = pattern.borrow().duplicate();
        // and apply sample offset and event transform
        pattern
            .borrow_mut()
            .set_sample_offset(midi_note.as_ref().map(|n| n.sample_offset).unwrap_or(0));
        pattern
            .borrow_mut()
            .set_event_transform(self.new_pattern_event_transform(midi_note));
        pattern
    }

    /// Create a note event transform function which applies instrument and
    /// note_transpose transforms, when set.
    fn new_pattern_event_transform(
        &self,
        midi_note: Option<PlayingNote>,
    ) -> Option<EventTransform> {
        let transforms: Vec<_> = [
            // Instrument transform
            self.instrument_id.map(InstrumentId::from).map(|id| {
                Box::new(move |note: &mut NoteEvent| {
                    if note.instrument.is_none() {
                        note.instrument = Some(id)
                    }
                }) as Box<dyn Fn(&mut NoteEvent)>
            }),
            // Note transform
            midi_note.map(|note| {
                let offset = note.note as i32 - 48;
                let volume = note.velocity as f32 / 127.0;
                Box::new(move |note_event: &mut NoteEvent| {
                    note_event.note = note_event.note.transposed(offset);
                    note_event.volume *= volume;
                }) as Box<dyn Fn(&mut NoteEvent)>
            }),
        ]
        .into_iter()
        .flatten()
        .collect();

        if !transforms.is_empty() {
            Some(Rc::new(move |event: &mut Event| {
                if let Event::NoteEvents(note_events) = event {
                    note_events.iter_mut().flatten().for_each(|note_event| {
                        transforms
                            .iter()
                            .for_each(|transform| transform(note_event))
                    });
                }
            }))
        } else {
            None
        }
    }
}

// -------------------------------------------------------------------------------------------------

// helper function to create a new raw CString from strings which may contain inner \0 chars.
unsafe fn new_raw_cstring(str: &str) -> *mut ffi::c_char {
    if str.contains('\0') {
        ffi::CString::from_vec_unchecked(str.replace('\0', "\\0").into()).into_raw()
    } else {
        ffi::CString::from_vec_unchecked(str.into()).into_raw()
    }
}

// helper function to drop a string created with `new_raw_cstring`
unsafe fn drop_raw_cstring(chars: *const ffi::c_char) {
    if !chars.is_null() {
        drop(ffi::CString::from_raw(chars as *mut ffi::c_char))
    }
}

/// Frees a string ptr which got passed to JS after it got consumed.
#[no_mangle]
pub unsafe extern "C" fn free_cstring(ptr: *mut ffi::c_char) {
    drop_raw_cstring(ptr);
}

// -------------------------------------------------------------------------------------------------

fn main() {
    // Disabled in build.rs via `cargo::rustc-link-arg=--no-entry`
    panic!("The main function is not exported and thus should never be called");
}

/// Creates global `Playground` state.
#[no_mangle]
pub extern "C" fn initialize_playground() -> *const ffi::c_char {
    // create or recreate the player instance
    println!("Creating new player instance...");
    match Playground::new() {
        Err(err) => {
            eprintln!("Failed to create player instance: {}", err);
            PLAYGROUND.replace(None);
            unsafe { new_raw_cstring(&err.to_string()) }
        }
        Ok(player) => {
            println!("Successfully created a new player instance");
            PLAYGROUND.replace(Some(player));
            std::ptr::null()
        }
    }
}

/// Destroys global `Playground` state.
#[no_mangle]
pub extern "C" fn shutdown_playground() {
    // drop the player instance
    println!("Dropping player instance...");
    PLAYGROUND.replace(None);
}

/// Start playback.
#[no_mangle]
pub extern "C" fn start_playing() {
    with_playground_mut(|playground| playground.start_playing());
}

/// Stop playback.
#[no_mangle]
pub extern "C" fn stop_playing() {
    with_playground_mut(|playground| playground.stop_playing());
}

/// Stop all playing notes.
#[no_mangle]
pub extern "C" fn stop_playing_notes() {
    with_playground_mut(|playground| playground.stop_playing_notes());
}

/// Set new global volume factor.
#[no_mangle]
pub extern "C" fn set_volume(volume: f32) {
    with_playground_mut(|playground| playground.set_volume(volume));
}

/// Handle note on event from the frontend
#[no_mangle]
pub extern "C" fn midi_note_on(note: u8, velocity: u8) {
    with_playground_mut(|playground| playground.handle_midi_note_on(note, velocity));
}

/// Handle note off event from the frontend
#[no_mangle]
pub extern "C" fn midi_note_off(note: u8) {
    with_playground_mut(|playground| playground.handle_midi_note_off(note));
}

/// Update player's BPM.
#[no_mangle]
pub extern "C" fn set_bpm(bpm: ffi::c_int) {
    with_playground_mut(|playground| playground.set_bpm(bpm as f32));
}

/// Update player's default instrument id.
#[no_mangle]
pub extern "C" fn set_instrument(id: ffi::c_int) {
    with_playground_mut(|playground| playground.set_instrument(id));
}

/// Set a script parameter value.
#[no_mangle]
pub unsafe extern "C" fn set_parameter_value(id_ptr: *const ffi::c_char, value: f64) {
    let id = ffi::CStr::from_ptr(id_ptr).to_string_lossy().into_owned();
    with_playground_mut(|playground| playground.set_parameter_value(&id, value));
}

#[no_mangle]
pub unsafe extern "C" fn update_script(content_ptr: *const ffi::c_char) {
    let content = unsafe {
        ffi::CStr::from_ptr(content_ptr)
            .to_string_lossy()
            .into_owned()
    };
    with_playground_mut(|playground| playground.update_script_content(content));
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
    with_playground_mut(
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
    with_playground_mut(|playground| {
        playground.clear_samples();
    });
}

/// Returns available sample names and ids as json string.
#[no_mangle]
pub unsafe extern "C" fn get_samples() -> *const ffi::c_char {
    let json = with_playground(|playground| serde_json::to_string(&playground.samples).unwrap());
    new_raw_cstring(&json)
}

/// Returns example script names and contents as json string.
#[no_mangle]
pub unsafe extern "C" fn get_example_scripts() -> *const ffi::c_char {
    let example_scripts = Playground::example_scripts().unwrap();
    new_raw_cstring(&serde_json::to_string(&example_scripts).unwrap())
}

#[no_mangle]
pub unsafe extern "C" fn get_quickstart_scripts() -> *const ffi::c_char {
    let quickstart_scripts = Playground::quickstart_scripts().unwrap();
    new_raw_cstring(&serde_json::to_string(&quickstart_scripts).unwrap())
}

/// Returns actual script parameters, if any
#[no_mangle]
pub unsafe extern "C" fn get_script_parameters() -> *const ffi::c_char {
    let parameters = with_playground(|playground| playground.script_parameters.clone());
    new_raw_cstring(&serde_json::to_string(&parameters).unwrap())
}

/// Returns actual script runtime errors, if any
#[no_mangle]
pub unsafe extern "C" fn get_script_error() -> *const ffi::c_char {
    let string = with_playground(|playground| playground.script_error.clone());
    new_raw_cstring(&string)
}

// -------------------------------------------------------------------------------------------------

/// Call the given `window.$NOTIFIER` function in the frontend
unsafe fn call_frontend_notifier(notifier_name: &str) {
    // NB: async to avoid that JS is calling back into rust while the playground ref is borrowed
    let ptr = new_raw_cstring(format!("window.setTimeout(window.{}, 0)", notifier_name).as_str());
    emscripten_run_script(ptr);
    free_cstring(ptr);
}
