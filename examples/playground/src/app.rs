use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use four_cc::FourCC;
use pattrns::prelude::*;

use crate::ffi::{
    call_frontend_notifier, EffectInfo, EffectParameterInfo, MixerInfo, SampleEntry, ScriptEntry,
    ScriptParameter, ScriptSection,
};

// -------------------------------------------------------------------------------------------------

/// Max expected MIDI notes
const NUM_MIDI_NOTES: usize = 127;
/// Path to our assets folder. see build.rs.
const ASSETS_PATH: &str = "/assets";
/// Event scheduler read-ahead time (latency)
const PLAYBACK_PRELOAD_SECONDS: f64 = if cfg!(debug_assertions) { 0.2 } else { 0.1 };

// -------------------------------------------------------------------------------------------------

/// Playback-related state
pub struct PlaybackState {
    playing: bool,
    output_start_sample_time: u64,
    emitted_sample_time: u64,
}

impl PlaybackState {
    pub fn new(player: &SamplePlayer) -> Self {
        Self {
            playing: false,
            output_start_sample_time: player.inner().output_sample_frame_position(),
            emitted_sample_time: 0,
        }
    }

    /// Starts playback of the current sequence.
    pub fn start(&mut self, player: &SamplePlayer, sequence: &mut Option<Sequence>) {
        if !self.playing {
            // reset play head
            self.output_start_sample_time = player.inner().output_sample_frame_position();
            self.emitted_sample_time = 0;
            // reset sequence
            if let Some(sequence) = sequence.as_mut() {
                sequence.reset();
            }
            // start playback
            self.playing = true;
        }
    }

    /// Stops all currently playing audio sources and resets the sequence.
    pub fn stop(&mut self, player: &mut SamplePlayer) {
        let _ = player.stop_all_sources();
        self.playing = false;
    }
}

/// Script-related state
pub struct ScriptState {
    content: String,
    changed: bool,
    parameters: Vec<ScriptParameter>,
    parameter_values: HashMap<String, f64>,
    error: String,
}

impl ScriptState {
    pub fn new() -> Self {
        Self {
            content: "return pattern { }".to_string(),
            changed: true,
            parameters: Vec::new(),
            parameter_values: HashMap::new(),
            error: String::new(),
        }
    }

    /// Update script error internally and in frontend if needed
    pub fn update_error(&mut self, error: &str) {
        if self.error != error {
            self.error = error.to_string();
            unsafe {
                call_frontend_notifier("on_script_error_changed");
            }
        }
    }

    /// Update script parameters internally and in frontend if needed
    pub fn update_parameters(&mut self, parameters: &[ScriptParameter]) {
        let parameters_changed = self.parameters != parameters;
        // memorize new parameters
        self.parameters = parameters.to_vec();
        self.parameter_values
            .retain(|id, _| self.parameters.iter().any(|p| p.0.borrow().id() == id));
        if parameters_changed {
            unsafe {
                call_frontend_notifier("on_script_parameters_changed");
            }
        }
    }

    /// Create a new pattern from script content.
    pub fn create_pattern(
        time_base: BeatTimeBase,
        instrument_id: Option<InstrumentId>,
        script_content: &str,
    ) -> (Rc<RefCell<dyn Pattern>>, String) {
        // create a new pattern from our script
        match new_pattern_from_string(time_base, instrument_id, script_content, "[script]") {
            Ok(pattern) => {
                // return pattern as it is
                (pattern, String::new())
            }
            Err(err) => {
                // create an empty fallback pattern on errors
                (
                    Rc::new(RefCell::new(BeatTimePattern::new(
                        time_base,
                        BeatTimeStep::Beats(1.0),
                    ))),
                    err.to_string(),
                )
            }
        }
    }
}

/// MIDI note handling
pub struct MidiState {
    pub playing_notes: Vec<PlayingNote>,
}

impl MidiState {
    pub fn new() -> Self {
        Self {
            playing_notes: Vec::new(),
        }
    }
}

/// Single pattern triggered by a MIDI note
#[derive(Clone)]
pub struct PlayingNote {
    note: u8,
    velocity: u8,
    sample_offset: SampleTime,
}

/// Metadata for an effect instance with its position in the chain
pub struct EffectMetadata {
    pub id: EffectId,
    pub name: String,
    pub parameters: Vec<Box<dyn EffectParameter>>,
    pub parameter_values: HashMap<u32, f32>,
}

/// Effect chain for a mixer, maintaining insertion order
#[derive(Default)]
pub struct EffectChain {
    pub effects: Vec<EffectMetadata>,
}

impl EffectChain {
    fn effect_position(&self, effect_id: EffectId) -> Option<usize> {
        self.effects.iter().position(|e| e.id == effect_id)
    }

    fn effect(&self, effect_id: EffectId) -> Option<&EffectMetadata> {
        self.effects.iter().find(|e| e.id == effect_id)
    }
    fn effect_mut(&mut self, effect_id: EffectId) -> Option<&mut EffectMetadata> {
        self.effects.iter_mut().find(|e| e.id == effect_id)
    }

    fn add_effect(&mut self, metadata: EffectMetadata) {
        self.effects.push(metadata);
    }

    fn remove_effect(&mut self, effect_id: EffectId) -> Option<EffectMetadata> {
        if let Some(index) = self.effect_position(effect_id) {
            Some(self.effects.remove(index))
        } else {
            None
        }
    }

    fn move_effect(&mut self, effect_id: EffectId, direction: i32) -> Result<()> {
        let current_index = self
            .effect_position(effect_id)
            .ok_or_else(|| anyhow!("Effect {effect_id} not found"))?;

        let new_index = (current_index as i32 + direction)
            .max(0)
            .min(self.effects.len() as i32 - 1) as usize;

        if current_index != new_index {
            let effect = self.effects.remove(current_index);
            self.effects.insert(new_index, effect);
        }

        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------

/// The backend's global app state.
pub struct App {
    player: SamplePlayer,
    sample_pool: Arc<SamplePool>,
    samples: Vec<SampleEntry>,
    time_base: BeatTimeBase,
    time_base_changed: bool,
    sequence: Option<Sequence>,
    pattern: Option<Rc<RefCell<dyn Pattern>>>,
    instrument_id: Option<usize>,
    script: ScriptState,
    playback: PlaybackState,
    midi: MidiState,
    sample_mixers: HashMap<InstrumentId, (MixerId, String)>,
    mixer_effects: HashMap<MixerId, EffectChain>,
}

impl App {
    /// Creates a new App instance with initialized state.
    /// Returns an error if initialization fails at any step.
    pub fn new() -> Result<Self> {
        // load samples
        let sample_pool = Arc::new(SamplePool::new());
        let samples = Self::load_bundled_samples(&sample_pool)?;

        // create sample player
        println!("Creating audio player...");
        let mut player = SamplePlayer::new(Arc::clone(&sample_pool), None)
            .map_err(|err| anyhow!("Audio error: {err}"))?;
        player.set_sample_root_note(Note::C4);
        player.set_new_note_action(NewNoteAction::Off(Some(Duration::from_millis(350))));
        player.set_playback_preload_time(Duration::from_secs_f64(PLAYBACK_PRELOAD_SECONDS));

        let time_base_changed = false;
        let time_base = BeatTimeBase {
            beats_per_min: 120.0,
            beats_per_bar: 4,
            samples_per_sec: player.sample_rate(),
        };

        let sequence = None;
        let pattern = None;

        // Create a mixer for each sample
        let mut sample_mixers: HashMap<InstrumentId, (MixerId, String)> = HashMap::new();
        let mixer_effects: HashMap<MixerId, EffectChain> = HashMap::new();

        for sample in &samples {
            let mixer_name = format!("{} FX", sample.name);
            let mixer_id = player
                .inner_mut()
                .add_mixer(None)
                .map_err(|err| anyhow!("Mixer error: {err}"))?;

            let instrument_id = InstrumentId::from(sample.id);
            sample_pool.set_target_mixer(instrument_id, Some(mixer_id));
            sample_mixers.insert(instrument_id, (mixer_id, mixer_name.clone()));
            println!(
                "Created mixer '{}' for sample '{}'",
                mixer_name, sample.name
            );
        }

        // default instrument
        let instrument_id = samples.first().map(|e| e.id);

        // components
        let playback = PlaybackState::new(&player);
        let script = ScriptState::new();
        let midi = MidiState::new();

        Ok(Self {
            playback,
            player,
            sample_pool,
            samples,
            sequence,
            pattern,
            time_base,
            time_base_changed,
            script,
            sample_mixers,
            mixer_effects,
            midi,
            instrument_id,
        })
    }

    /// Main playback loop: Handles player state updates and runs the player
    pub fn run(&mut self) {
        // apply script content changes
        if self.script.changed || self.sequence.is_none() {
            self.rebuild_sequence();
        }

        // apply time base changes
        if self.time_base_changed {
            self.time_base_changed = false;
            if let Some(sequence) = &mut self.sequence {
                sequence.set_time_base(&self.time_base);
            }
        }

        // verify internal state
        debug_assert!(
            !self.script.changed && self.sequence.is_some(),
            "Should have a valid sequence here"
        );

        // check if audio output has been suspended by the browser
        let suspended = self.player.inner().output_suspended();

        // run the player, when playing and audio output is not suspended
        if !suspended && (self.playback.playing || !self.midi.playing_notes.is_empty()) {
            // calculate samples to emit
            let samples_to_emit = self.player.calculate_samples_to_emit(
                &self.time_base,
                self.playback.output_start_sample_time,
                self.playback.emitted_sample_time,
            );
            let playback_preload = self
                .time_base
                .seconds_to_samples(self.player.playback_preload_time().as_secs_f64());
            if samples_to_emit > 4 * playback_preload {
                // we lost too much time: maybe because the browser suspended the run loop
                self.player.advance_until_time(
                    self.sequence.as_mut().unwrap(),
                    self.playback.emitted_sample_time + samples_to_emit,
                );
            } else if samples_to_emit > 0 {
                // continue generating events in real-time
                self.player.run_until_time(
                    self.sequence.as_mut().unwrap(),
                    self.playback.output_start_sample_time,
                    self.playback.emitted_sample_time + samples_to_emit,
                );
                // handle runtime errors
                if let Some(err) = pattrns::bindings::has_lua_callback_errors() {
                    self.script.update_error(&err.to_string());
                    pattrns::bindings::clear_lua_callback_errors();
                }
            }
            self.playback.emitted_sample_time += samples_to_emit;
        }
    }

    /// Starts playback of the current sequence.
    pub fn start_playing(&mut self) {
        self.playback.start(&self.player, &mut self.sequence);
    }

    /// Stops all currently playing audio sources and resets the sequence.
    pub fn stop_playing(&mut self) {
        self.playback.stop(&mut self.player);
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
        assert!(note as usize <= NUM_MIDI_NOTES);
        if self.midi.playing_notes.is_empty()
            || Self::pattern_slot(&mut self.sequence, note as usize).is_none()
        {
            // reset play head
            self.playback.output_start_sample_time =
                self.player.inner().output_sample_frame_position();
            self.playback.emitted_sample_time = 0;
            // memorize playing note
            let new_note = PlayingNote {
                note,
                velocity,
                sample_offset: 0,
            };
            self.midi.playing_notes.push(new_note);
            // rebuild sequence
            self.script.changed = true;
        } else {
            // memorize playing note
            let playback_preload = self
                .time_base
                .seconds_to_samples(self.player.playback_preload_time().as_secs_f64());
            let sample_offset = self
                .playback
                .emitted_sample_time
                .saturating_sub(playback_preload);
            let new_note = PlayingNote {
                note,
                velocity,
                sample_offset,
            };
            self.midi.playing_notes.push(new_note.clone());
            // add a new pattern for the new note
            let pattern = self
                .pattern
                .as_ref()
                .expect("Expecting a valid pattern instance when notes are playing");
            let new_pattern = Self::create_pattern_instance(
                pattern,
                Some(new_note),
                self.instrument_id.map(InstrumentId::from),
            );
            let pattern_slot = Self::pattern_slot(&mut self.sequence, note as usize)
                .expect("Missing MIDI pattern slot");
            *pattern_slot = PatternSlot::Pattern(new_pattern);
        }
    }

    /// Handle incoming MIDI note off event
    pub fn handle_midi_note_off(&mut self, note: u8) {
        assert!(note as usize <= NUM_MIDI_NOTES);

        // only handle off events when we got an on event
        if let Some((playing_notes_index, _)) = self
            .midi
            .playing_notes
            .iter()
            .enumerate()
            .find(|(_, n)| n.note == note)
        {
            // remove playing note
            self.midi.playing_notes.remove(playing_notes_index);
            // remove the pattern slot from sequence's phrase
            if let Some(slot) = Self::pattern_slot(&mut self.sequence, note as usize) {
                *slot = PatternSlot::Stop;
                // stop pending from the note
                self.player.stop_sources_in_pattern_slot(note as usize);
            }
            // restore default playback in `run` with the last note removed
            if self.midi.playing_notes.is_empty() {
                self.script.changed = true;
                let _ = self.player.stop_all_sources();
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
        self.script.changed = true;
    }

    /// Read examples from the file system into a vector of ScriptEntry
    pub fn example_scripts() -> Result<Vec<ScriptEntry>> {
        let mut example_entries = Vec::new();
        let example_paths = std::fs::read_dir(format!("{}/examples", ASSETS_PATH))?;
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
            let content = std::fs::read_to_string(&path)?;
            example_entries.push(ScriptEntry { name, content });
        }
        Ok(example_entries)
    }

    /// Read quickstart examples from the file system into a vector of ScriptSection
    pub fn quickstart_scripts() -> Result<Vec<ScriptSection>> {
        let mut quickstart_scripts = Vec::new();
        let section_paths = std::fs::read_dir(format!("{}/quickstart", ASSETS_PATH))?;
        for section_path in section_paths.flatten() {
            if section_path.metadata()?.is_dir() {
                let mut section_name = section_path.file_name().to_string_lossy().to_string();
                section_name = section_name
                    .trim_start_matches(|c: char| c.is_ascii_digit() || c == ' ')
                    .to_string();
                let mut section_entries = Vec::new();
                let script_paths = std::fs::read_dir(section_path.path())?;
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
                        let script_content = std::fs::read_to_string(script_path.path())?;
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

    /// Updates the script content and marks it as changed to trigger recompilation.
    pub fn update_script_content(&mut self, content: String) {
        if self.script.content != content {
            self.script.content = content;
            self.script.changed = true;
        }
    }

    /// Get current script runtime or compile errors.
    pub fn script_error(&self) -> &str {
        &self.script.error
    }

    /// Get list of parameters for the current script.
    pub fn script_parameters(&self) -> &[ScriptParameter] {
        &self.script.parameters
    }

    /// Sets a script parameter value.
    pub fn set_script_parameter_value(&mut self, id: &str, value: f64) {
        self.script.parameter_values.insert(id.to_owned(), value);
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

    /// Get list of available samples from the assets folder.
    pub fn samples(&self) -> &[SampleEntry] {
        &self.samples
    }

    /// Load a sample from a raw file buffer and add it to the pool
    pub fn load_sample(&mut self, file_buffer: Vec<u8>, file_name: &str) -> Result<usize> {
        let (id, name) = Self::load_sample_from_buffer(&self.sample_pool, file_buffer, file_name)?;

        // Create a dedicated mixer for this sample
        let mixer_name = format!("{} FX", name);
        let mixer_id = self
            .player
            .inner_mut()
            .add_mixer(None)
            .map_err(|err| anyhow!("Mixer error: {err}"))?;

        let instrument_id = InstrumentId::from(id);
        self.sample_pool
            .set_target_mixer(instrument_id, Some(mixer_id));
        self.sample_mixers
            .insert(instrument_id, (mixer_id, mixer_name));

        self.samples.push(SampleEntry { name, id });
        Ok(id)
    }

    /// Reset sample pool, removing all samples and their mixers
    pub fn clear_samples(&mut self) {
        // Remove all mixers associated with samples
        for (_, (mixer_id, _)) in self.sample_mixers.drain() {
            let _ = self.player.inner_mut().remove_mixer(mixer_id);
            self.mixer_effects.remove(&mixer_id);
        }

        self.sample_pool.clear();
        self.samples.clear();
        self.instrument_id = None;
        self.script.changed = true;
    }

    /// Get all instantiated mixers with their effects
    pub fn mixers(&self) -> Vec<MixerInfo> {
        let mut mixer_infos: Vec<_> = self
            .sample_mixers
            .iter()
            .map(|(instrument_id, (mixer_id, name))| MixerInfo {
                id: *mixer_id,
                name: name.clone(),
                instrument_id: Some(usize::from(*instrument_id)),
                effects: self.mixer_effects(*mixer_id),
            })
            .collect();
        mixer_infos.sort_by_key(|m| m.id);
        mixer_infos
    }

    /// Get list of available effects
    pub fn available_effects() -> Vec<&'static str> {
        vec![
            "Gain",
            "DcFilter",
            "Filter",
            "Eq5",
            "Reverb",
            "Chorus",
            "Compressor",
            "Distortion",
        ]
    }

    /// Add effect by name
    pub fn add_effect_by_name(
        &mut self,
        mixer_id: MixerId,
        effect_name: &str,
    ) -> Result<(EffectId, Vec<EffectParameterInfo>)> {
        if !self.sample_mixers.values().any(|(mid, _)| *mid == mixer_id) {
            return Err(anyhow!("Mixer {mixer_id} not found"));
        }
        match effect_name {
            "Gain" => self.add_effect(mixer_id, effects::GainEffect::new(), effect_name),
            "DcFilter" => self.add_effect(mixer_id, effects::DcFilterEffect::new(), effect_name),
            "Filter" => self.add_effect(mixer_id, effects::FilterEffect::new(), effect_name),
            "Eq5" => self.add_effect(mixer_id, effects::Eq5Effect::new(), effect_name),
            "Reverb" => self.add_effect(mixer_id, effects::ReverbEffect::new(), effect_name),
            "Chorus" => self.add_effect(mixer_id, effects::ChorusEffect::new(), effect_name),
            "Compressor" => self.add_effect(
                mixer_id,
                effects::CompressorEffect::new_compressor(),
                effect_name,
            ),
            "Distortion" => {
                self.add_effect(mixer_id, effects::DistortionEffect::new(), effect_name)
            }
            _ => Err(anyhow!("Unknown effect: {effect_name}")),
        }
    }

    /// Move effect within mixer's effect chain
    pub fn move_effect(
        &mut self,
        effect_id: EffectId,
        mixer_id: MixerId,
        direction: i32,
    ) -> Result<()> {
        let movement = if direction != 0 {
            EffectMovement::Direction(direction)
        } else {
            return Ok(()); // No movement needed
        };

        // Move in the player
        self.player
            .inner_mut()
            .move_effect(movement, effect_id, mixer_id)
            .map_err(|err| anyhow!("Effect error: {err}"))?;

        // Update local tracking
        if let Some(chain) = self.mixer_effects.get_mut(&mixer_id) {
            chain.move_effect(effect_id, direction)?;
        }

        Ok(())
    }

    /// Remove effect from mixer
    pub fn remove_effect(&mut self, effect_id: EffectId) -> Result<()> {
        self.player
            .inner_mut()
            .remove_effect(effect_id)
            .map_err(|err| anyhow!("Effect error: {err}"))?;

        for chain in self.mixer_effects.values_mut() {
            chain.remove_effect(effect_id);
        }
        Ok(())
    }

    /// Get effect parameter value as string
    pub fn effect_parameter_string(
        &self,
        effect_id: EffectId,
        param_id: u32,
        normalized_value: f32,
    ) -> Result<String> {
        let param_fourcc = FourCC::from(param_id);
        for chain in self.mixer_effects.values() {
            if let Some(metadata) = chain.effect(effect_id) {
                if let Some(param) = metadata.parameters.iter().find(|p| p.id() == param_fourcc) {
                    return Ok(param.value_to_string(normalized_value, true));
                }
            }
        }
        Err(anyhow!(
            "Parameter {param_id} not found in effect {effect_id}",
        ))
    }

    /// Set effect parameter from a normalized value.
    pub fn set_effect_parameter_value(
        &mut self,
        effect_id: EffectId,
        param_id: u32,
        normalized_value: f32,
    ) -> Result<()> {
        let param_fourcc = FourCC::from(param_id);
        let clamped_value = normalized_value.clamp(0.0, 1.0);

        // Store the parameter value in metadata
        for chain in self.mixer_effects.values_mut() {
            if let Some(metadata) = chain.effect_mut(effect_id) {
                metadata.parameter_values.insert(param_id, clamped_value);
                break;
            }
        }

        self.player
            .inner_mut()
            .set_effect_parameter_normalized(effect_id, param_fourcc, clamped_value, None)
            .map_err(|err| anyhow!("Effect error: {err}"))
    }

    /// Rebuild sequence and pattern from actual script content
    fn rebuild_sequence(&mut self) {
        // clear runtime errors
        pattrns::bindings::clear_lua_callback_errors();

        // build pattern and set compile errors and parameters
        let (new_pattern, error) = ScriptState::create_pattern(
            self.time_base,
            self.instrument_id.map(InstrumentId::from),
            &self.script.content,
        );
        self.script.update_error(&error);
        self.script.update_parameters(
            &new_pattern
                .borrow()
                .parameters()
                .iter()
                .map(ScriptParameter::from)
                .collect::<Vec<_>>(),
        );

        // restore previous parameter values in new pattern
        for (id, value) in &self.script.parameter_values {
            if let Some(parameter) = new_pattern
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
            if !self.midi.playing_notes.is_empty() {
                // one pattern for each live played note
                let mut slots = vec![PatternSlot::Stop; NUM_MIDI_NOTES];
                for playing_note in &self.midi.playing_notes.clone() {
                    slots[playing_note.note as usize] =
                        PatternSlot::Pattern(Self::create_pattern_instance(
                            &new_pattern,
                            Some(playing_note.clone()),
                            self.instrument_id.map(InstrumentId::from),
                        ));
                }
                slots
            } else {
                // a single pattern slot for regular playback
                vec![PatternSlot::Pattern(Rc::clone(&new_pattern))]
            }
        };

        // replace pattern and sequence
        let mut new_sequence = Sequence::new(
            self.time_base,
            vec![Phrase::new(
                self.time_base,
                pattern_slots,
                BeatTimeStep::Bar(4.0),
            )],
        );
        self.player.prepare_run_until_time(
            self.sequence.take().as_mut(),
            &mut new_sequence,
            self.playback.output_start_sample_time,
            self.playback.emitted_sample_time,
        );
        self.sequence = Some(new_sequence);
        self.pattern = Some(new_pattern);

        // reset all update flags: we're fully up to date now.
        self.script.changed = false;
    }

    /// Access a pattern slot by index. pattern_index is used as MIDI note number.
    fn pattern_slot(
        sequence: &mut Option<Sequence>,
        pattern_index: usize,
    ) -> Option<&mut PatternSlot> {
        if let Some(sequence) = sequence {
            let phrase = sequence
                .phrases_mut()
                .first_mut()
                .expect("Failed to access phrase");
            phrase.pattern_slots_mut().get_mut(pattern_index)
        } else {
            None
        }
    }

    /// Create a new pattern instance clone for the given note from the passed pattern
    /// for the given optional midi note for note transforms.
    fn create_pattern_instance(
        pattern: &Rc<RefCell<dyn Pattern>>,
        midi_note: Option<PlayingNote>,
        instrument_id: Option<InstrumentId>,
    ) -> Rc<RefCell<dyn Pattern>> {
        // create a new pattern clone
        let new_pattern = pattern.borrow().duplicate();
        // and apply sample offset and event transform
        new_pattern
            .borrow_mut()
            .set_sample_offset(midi_note.as_ref().map(|n| n.sample_offset).unwrap_or(0));
        new_pattern
            .borrow_mut()
            .set_event_transform(Self::create_event_transform(midi_note, instrument_id));
        new_pattern
    }

    /// Create a note event transform function which applies instrument and
    /// note_transpose transforms, when set.
    fn create_event_transform(
        midi_note: Option<PlayingNote>,
        instrument_id: Option<InstrumentId>,
    ) -> Option<EventTransform> {
        let transforms: Vec<_> = [
            // Instrument transform
            instrument_id.map(|id| {
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

    /// Load all samples from the assets/samples folder
    fn load_bundled_samples(sample_pool: &Arc<SamplePool>) -> Result<Vec<SampleEntry>> {
        println!("Loading sample files...");
        let mut samples = Vec::new();
        for dir_entry in std::fs::read_dir(format!("{}/samples", ASSETS_PATH))?.flatten() {
            let path = dir_entry.path();
            if let Some(extension) = path.extension().map(|e| e.to_string_lossy()) {
                if matches!(extension.as_bytes(), b"mp3" | b"wav" | b"flac") {
                    let id = usize::from(
                        sample_pool
                            .load_sample(&path)
                            .map_err(|err| anyhow!("Sample error: {err}"))?,
                    );
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    println!("Added sample '{}' with id {}", name, id);
                    samples.push(SampleEntry { id, name });
                }
            }
        }
        Ok(samples)
    }

    /// Load a sample from a raw file buffer and add it to the pool
    fn load_sample_from_buffer(
        sample_pool: &Arc<SamplePool>,
        file_buffer: Vec<u8>,
        file_name: &str,
    ) -> Result<(usize, String)> {
        let instrument_id = sample_pool
            .load_sample_buffer(file_buffer, file_name)
            .map_err(|err| anyhow!("Sample error: {err}"))?;
        let id = usize::from(instrument_id);
        let name = std::path::Path::new(&file_name)
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        Ok((id, name))
    }

    /// Add a new effect to the given mixer.
    fn add_effect<E: Effect>(
        &mut self,
        mixer_id: MixerId,
        effect: E,
        effect_name: &str,
    ) -> Result<(EffectId, Vec<EffectParameterInfo>)> {
        // Store parameter metadata
        let parameters: Vec<Box<dyn EffectParameter>> =
            effect.parameters().iter().map(|p| p.dyn_clone()).collect();

        let param_info: Vec<EffectParameterInfo> =
            parameters.iter().map(Self::effect_parameter_info).collect();

        let effect_id = self
            .player
            .inner_mut()
            .add_effect(effect, mixer_id)
            .map_err(|err| anyhow!("Effect error: {err}"))?;

        // Initialize parameter values with defaults
        let mut parameter_values = HashMap::new();
        for param in &parameters {
            parameter_values.insert(param.id().into(), param.default_value());
        }

        self.mixer_effects
            .entry(mixer_id)
            .or_default()
            .add_effect(EffectMetadata {
                id: effect_id,
                name: effect_name.to_string(),
                parameters,
                parameter_values,
            });

        Ok((effect_id, param_info))
    }

    /// Convert effect parameter to EffectParameterInfo
    fn effect_parameter_info(param: &Box<dyn EffectParameter>) -> EffectParameterInfo {
        let param_type = match param.parameter_type() {
            EffectParameterType::Float => "Float",
            EffectParameterType::Integer => "Integer",
            EffectParameterType::Boolean => "Boolean",
            EffectParameterType::Enum { .. } => "Enum",
        };
        EffectParameterInfo {
            id: param.id().into(),
            name: param.name().to_string(),
            param_type: param_type.to_string(),
            default: param.default_value(),
            values: match param.parameter_type() {
                EffectParameterType::Enum { values } => Some(values),
                _ => None,
            },
        }
    }

    /// Get all effects for a mixer
    fn mixer_effects(&self, mixer_id: MixerId) -> Vec<EffectInfo> {
        self.mixer_effects
            .get(&mixer_id)
            .map(|chain| {
                chain
                    .effects
                    .iter()
                    .map(|metadata| EffectInfo {
                        id: metadata.id,
                        name: metadata.name.clone(),
                        parameters: metadata
                            .parameters
                            .iter()
                            .map(|param| {
                                let mut param_info = Self::effect_parameter_info(param);
                                // Use stored value if available, otherwise use default
                                if let Some(stored_value) =
                                    metadata.parameter_values.get(&param_info.id)
                                {
                                    param_info.default = *stored_value;
                                }
                                param_info
                            })
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}
