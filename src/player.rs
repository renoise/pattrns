//! Example player implementation, which plays back a [`Sequence`]
//! via the [`phonic`](https://crates.io/crates/phonic) crate.

use std::{
    collections::HashMap,
    sync::{mpsc::SyncSender, Arc},
    time::Duration,
};

use crate::{
    time::{SampleTimeBase, SampleTimeDisplay},
    BeatTimeBase, Event, ExactSampleTime, Note, NoteEvent, PatternEvent, PatternSlot, SampleTime,
    Sequence,
};

// -------------------------------------------------------------------------------------------------

/// [`phonic`](https://crates.io/crates/phonic) mixer, generators and effects.
pub mod phonic {
    pub use phonic::*;
}

// -------------------------------------------------------------------------------------------------

/// Player's behavior when playing a new note on the same voice channel.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum NewNoteAction {
    /// Continue playing the old note and start a new one.
    Continue,
    /// Stop the playing note before starting a new one.
    #[default]
    Stop,
}

// -------------------------------------------------------------------------------------------------

/// Context, passed along to phonic playback event channels when triggering new notes from the player.
#[derive(Clone)]
pub struct PlaybackContext {
    /// The triggered note.
    pub note: Note,
    /// Pattern index in [`PhraseEvent`].
    pub pattern_index: Option<usize>,
    /// Voice index of note arrays in [`PatternEvent`].
    pub voice_index: Option<usize>,
}

impl PlaybackContext {
    pub fn from_event(context: Option<phonic::PlaybackStatusContext>) -> Self {
        if let Some(context) = context {
            if let Some(context) = context.downcast_ref::<PlaybackContext>() {
                return context.clone();
            }
        }
        PlaybackContext {
            note: Note::EMPTY,
            pattern_index: None,
            voice_index: None,
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// A simple example player implementation using [`phonic`](https://crates.io/crates/phonic),
/// which plays back a [`Sequence`] using the default audio output device.
///
/// To create and use mixers and DSP effects, use the [`Self::inner_mut`] function to access the
/// underlying phonic player.
pub struct Player {
    inner: phonic::Player,
    generators: Vec<Option<phonic::GeneratorPlaybackHandle>>,
    playing_notes: Vec<HashMap<usize, PlayingNote>>,
    new_note_action: NewNoteAction,
    sample_root_note: Note,
    show_events: bool,
    playback_pos_emit_rate: Duration,
    playback_preload_time: Duration,
    playback_start_sample_time: SampleTime,
    emitted_sample_time: SampleTime,
}

impl Player {
    /// Default preload time of the player's `run_until` function.
    /// Quite high by default and bigger in slower debug builds.
    const DEFAULT_PLAYBACK_PRELOAD_MS: u64 = if cfg!(debug_assertions) { 500 } else { 250 };

    /// Create a new sample player.
    ///
    /// # Errors
    /// returns an error if the player could not be created.
    pub fn new<S: Into<Option<SyncSender<phonic::PlaybackStatusEvent>>>>(
        output_device: phonic::DefaultOutputDevice,
        playback_status_sender: S,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let config = phonic::PlayerConfig::default();
        Self::new_with_config(output_device, playback_status_sender, config)
    }

    /// Create a new sample player with the given phonic player configuration.
    ///
    /// # Errors
    /// returns an error if the player could not be created.
    pub fn new_with_config<S: Into<Option<SyncSender<phonic::PlaybackStatusEvent>>>>(
        output_device: phonic::DefaultOutputDevice,
        playback_status_sender: S,
        config: phonic::PlayerConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // create player
        let inner = phonic::Player::new_with_config(output_device, playback_status_sender, config);

        let generators = Vec::new();
        let playing_notes = Vec::new();

        let new_note_action = NewNoteAction::default();
        let sample_root_note = Note::C5;
        let show_events = false;

        let playback_pos_emit_rate = Duration::from_secs(1);
        let playback_preload_time = Duration::from_millis(Self::DEFAULT_PLAYBACK_PRELOAD_MS);
        let playback_start_sample_time = inner.output_sample_frame_position();
        let emitted_sample_time = 0;

        Ok(Self {
            inner,
            generators,
            playing_notes,
            new_note_action,
            sample_root_note,
            show_events,
            playback_preload_time,
            playback_pos_emit_rate,
            playback_start_sample_time,
            emitted_sample_time,
        })
    }

    /// Access to the player's inner phonic instance.
    pub fn inner(&self) -> &phonic::Player {
        &self.inner
    }
    /// Mutable access to the player's inner phonic instance.
    pub fn inner_mut(&mut self) -> &mut phonic::Player {
        &mut self.inner
    }

    /// Return the output backend's sample rate. All sources will be played back at this rate.
    pub fn sample_rate(&self) -> u32 {
        self.inner.output_sample_rate()
    }

    /// Return the output backend's channel count.
    pub fn channel_count(&self) -> usize {
        self.inner.output_channel_count()
    }

    /// true when events are dumped to stdout while playing them.
    pub fn show_events(&self) -> bool {
        self.show_events
    }
    /// By default false: set to true to dump events to stdout while playing them.
    pub fn set_show_events(&mut self, show: bool) {
        self.show_events = show;
    }

    /// Preload time of the `run_until` function. By default 1/2 seconds in debug builds
    /// and 1/4 seconds in release builds. Should be big enough to ensure that events are
    /// scheduled ahead of playback time, but small enough to avoid too much latency.
    ///
    /// NB: real audio latency is twice the amount of the given time duration, plus the
    /// audio driver's output latency.
    pub fn playback_preload_time(&self) -> Duration {
        self.playback_preload_time
    }
    pub fn set_playback_preload_time(&mut self, preload_time: Duration) {
        self.playback_preload_time = preload_time;
    }

    /// Playback pos emit rate of triggered files. by default one second.
    pub fn playback_pos_emit_rate(&self) -> Duration {
        self.playback_pos_emit_rate
    }
    pub fn set_playback_pos_emit_rate(&mut self, emit_rate: Duration) {
        self.playback_pos_emit_rate = emit_rate;
    }

    /// Get current new note action behavior.
    pub fn new_note_action(&self) -> NewNoteAction {
        self.new_note_action
    }
    // Set a new new note action behavior.
    pub fn set_new_note_action(&mut self, action: NewNoteAction) {
        self.new_note_action = action;
    }

    /// Get root note used when converting event note values to sample playback speed.
    pub fn sample_root_note(&self) -> Note {
        self.sample_root_note
    }
    // Set a new global root note.
    pub fn set_sample_root_note(&mut self, root_note: Note) {
        self.sample_root_note = root_note;
    }

    /// Get the generator playback handle for the given pattern index, if it exists.
    ///
    /// This allows controlling the generator (e.g. changing volume, panning, or parameters)
    /// directly while the player is running.
    pub fn generator_handle(
        &self,
        pattern_index: usize,
    ) -> Option<phonic::GeneratorPlaybackHandle> {
        self.generators.get(pattern_index).and_then(Clone::clone)
    }

    /// Sets a generator for the given pattern index.
    ///
    /// If a generator already exists at this index, it gets replaced with the new one.
    pub fn set_generator(
        &mut self,
        pattern_index: usize,
        generator: Box<dyn phonic::Generator>,
        mixer_id: Option<phonic::MixerId>,
    ) -> Result<phonic::GeneratorPlaybackHandle, phonic::Error> {
        // Remove existing generator, if any
        self.clear_generator(pattern_index);
        // Add and memorize the new generator
        let handle = self.inner.add_generator(generator, mixer_id)?;
        if pattern_index >= self.generators.len() {
            self.generators.resize(pattern_index + 1, None);
        }
        self.generators[pattern_index] = Some(handle.clone());
        Ok(handle)
    }

    /// Clears the generator at the given index.
    pub fn clear_generator(&mut self, pattern_index: usize) {
        if let Some(generator_slot) = self.generators.get_mut(pattern_index) {
            if let Some(generator) = generator_slot.take() {
                if let Err(err) = self.inner.remove_generator(generator.id()) {
                    log::warn!("Failed to remove generator: '{err}'");
                }
            }
        }
    }

    /// Inserts a new generator slot at the given pattern index - shifting all generators after it to the right.
    /// Use this when inserting patterns in the sequence, else use [Self::set_generator] instead.
    pub fn insert_generator(
        &mut self,
        pattern_index: usize,
        generator: Box<dyn phonic::Generator>,
        mixer_id: Option<phonic::MixerId>,
    ) -> Result<phonic::GeneratorPlaybackHandle, phonic::Error> {
        self.generators.insert(pattern_index, None);
        self.set_generator(pattern_index, generator, mixer_id)
    }

    /// Removes the generator at the given index - shifting all generators after it to the left.
    /// Use this when removing patterns in the sequence, else use [Self::clear_generator] instead.
    pub fn remove_generator(&mut self, pattern_index: usize) {
        assert!(
            self.generators.get(pattern_index).is_some(),
            "Invalid generator index"
        );
        if let Some(generator) = self.generators.remove(pattern_index) {
            if let Err(err) = self.inner.remove_generator(generator.id()) {
                log::warn!("Failed to remove generator: '{err}'");
            }
        }
    }

    /// Move an existing generator slot to a new index. Use this when moving patterns in the sequence.
    pub fn move_generator(&mut self, from_pattern_index: usize, to_pattern_index: usize) {
        assert!(
            self.generators.get(from_pattern_index).is_none()
                || (self.generators.get(from_pattern_index).is_some()
                    && self.generators.get(to_pattern_index).is_some()),
            "Invalid generator indices"
        );

        if self.generators.get(from_pattern_index).is_some()
            && self.generators.get(to_pattern_index).is_some()
        {
            let item = self.generators.remove(from_pattern_index);
            if to_pattern_index >= self.generators.len() {
                self.generators.resize_with(to_pattern_index + 1, || None);
            }
            self.generators.insert(to_pattern_index, item);
        }
    }

    /// Stop all currently playing notes.
    pub fn stop_all_notes(&mut self) {
        // Stop all notes in all generators
        for generator in self.generators.iter().flatten() {
            if let Err(err) = generator.all_notes_off(None) {
                log::warn!("Failed to trigger all notes off: {err}");
            }
        }
        // Clear playing notes
        for notes in &mut self.playing_notes {
            notes.clear();
        }
        // Remove already scheduled events in all mixers too
        if let Err(err) = self.inner.stop_all_sources() {
            log::warn!("Failed to trigger stop all sources event: {err}");
        }
    }

    /// Stop all currently playing sources in the given pattern slot index.
    pub fn stop_notes_in_pattern_slot(&mut self, pattern_index: usize) {
        if let Some(notes) = self.playing_notes.get_mut(pattern_index) {
            for playing_note in notes.values() {
                if let Some(Some(generator)) = self.generators.get(pattern_index) {
                    if let Err(err) = generator.note_off(playing_note.note_id, None) {
                        log::warn!("Failed to trigger note off: {err}");
                    }
                }
            }
            notes.clear();
        }
    }

    /// Run/play the given sequence until it stops.
    pub fn run(
        &mut self,
        sequence: &mut Sequence,
        time_base: &dyn SampleTimeBase,
        reset_playback_pos: bool,
    ) {
        let previous_sequence = None;
        let dont_stop = || false;
        self.run_until(
            previous_sequence,
            sequence,
            time_base,
            reset_playback_pos,
            dont_stop,
        );
    }

    /// Run the given sequence until it stops or the passed stop condition function returns true.
    pub fn run_until<StopFn: FnMut() -> bool>(
        &mut self,
        previous_sequence: Option<&mut Sequence>,
        sequence: &mut Sequence,
        time_base: &dyn SampleTimeBase,
        reset_playback_pos: bool,
        mut stop_fn: StopFn,
    ) {
        if reset_playback_pos || self.emitted_sample_time == 0 {
            // reset time counters and the sequence when starting the first time or when requested
            self.reset_playback_position(sequence);
            log::debug!(target: "Player", "Resetting playback pos");
        } else {
            // else continue playing from our previous time, to avoid interrupting playback
            self.prepare_run_until_time(
                previous_sequence,
                sequence,
                self.playback_start_sample_time,
                self.emitted_sample_time,
            );
            log::debug!(target: "Player",
                "Advance sequence to time {:.2}",
                time_base.samples_to_seconds(self.emitted_sample_time)
            );
        }

        while !stop_fn() {
            // run sequence ahead of player by the self.playback_preload time
            let samples_to_emit = self.calculate_samples_to_emit(
                time_base,
                self.playback_start_sample_time,
                self.emitted_sample_time,
            );
            let playback_preload =
                time_base.seconds_to_samples(self.playback_preload_time.as_secs_f64());
            if samples_to_emit >= playback_preload || self.emitted_sample_time == 0 {
                // consume events in the preload range
                self.run_until_time(
                    sequence,
                    self.playback_start_sample_time,
                    self.emitted_sample_time + samples_to_emit,
                );
                self.emitted_sample_time += samples_to_emit;
            } else {
                // wait until next events are due and call stop_fn
                let max_sleep_time = self.playback_preload_time.as_secs_f64() / 4.0;
                let seconds_until_next_emit_batch =
                    time_base.samples_to_seconds(playback_preload.saturating_sub(samples_to_emit));
                let mut time_slept = 0.0;
                while time_slept < seconds_until_next_emit_batch && !stop_fn() {
                    let sleep_amount = seconds_until_next_emit_batch.min(max_sleep_time);
                    std::thread::sleep(Duration::from_secs_f64(sleep_amount));
                    time_slept += sleep_amount;
                }
            }
        }
    }

    /// Initialize the given sequence for playback with `run_until_time`.
    ///
    /// This seeks the given sequence to the given sample time, stops still playing notes
    /// and keeps track of internal playback state.
    ///
    /// When `previous_sequence` is set, it's run to lookup note-off and stop events that
    /// would have happened in future to stop pending notes. When its none, all playing notes
    /// will be stopped at the time the new sequence starts playing.
    pub fn prepare_run_until_time(
        &mut self,
        previous_sequence: Option<&mut Sequence>,
        sequence: &mut Sequence,
        time_offset: SampleTime,
        time: SampleTime,
    ) {
        // stop playing notes, if needed
        if self.playing_notes.iter().any(|notes| !notes.is_empty()) {
            // Process note stop events from the previous sequence
            let stop_time = if let Some(previous_sequence) = previous_sequence {
                // Get maximum pattern step length in samples of all currently playing back patterns
                let pattern_slots = {
                    match previous_sequence.current_phrase() {
                        Some(phrase) => phrase.pattern_slots(),
                        None => &[],
                    }
                };
                let mut max_step_length: ExactSampleTime = 0.0;
                for pattern_slot in pattern_slots {
                    if let PatternSlot::Pattern(pattern) = pattern_slot {
                        let pattern = pattern.borrow();
                        // We can't assume that every step produces a note-on, so run entire patterns
                        // or at least 4 steps with dynamic pattern generators.
                        max_step_length = max_step_length
                            .max(pattern.step_length() * pattern.step_count().max(4) as f64);
                    }
                }
                // Run sequence and handle note-offs only to stop playing notes
                let note_stop_lookup_time =
                    time_offset + time + max_step_length.ceil() as SampleTime;
                previous_sequence.consume_events_until_time(
                    note_stop_lookup_time,
                    &mut |pattern_index, pattern_event| {
                        self.handle_pattern_event_note_offs(
                            time_offset,
                            pattern_index,
                            pattern_event,
                        );
                    },
                );
                // stop remaining notes at the lookup time range's end
                note_stop_lookup_time
            } else {
                // stop remaining notes at the time the new sequence starts
                time_offset + time
            };
            // stop remaining playing notes at the lookup time or time we're applying the new sequence
            for (pattern_index, playing_notes) in self.playing_notes.iter_mut().enumerate() {
                for playing_note in playing_notes.values_mut() {
                    if playing_note.stop_time.is_none() {
                        if let Some(Some(generator)) = self.generators.get(pattern_index) {
                            if let Err(err) = generator.note_off(playing_note.note_id, stop_time) {
                                log::warn!("Failed to trigger note off: {err}");
                            }
                        }
                        playing_note.stop_time = Some(stop_time);
                    }
                }
            }
        }
        // update playing notes state to fit the new sequence
        self.playing_notes
            .resize_with(sequence.phrase_pattern_slot_count(), HashMap::new);
        // and finally, prepare the new sequence by advancing it to the target time
        sequence.advance_until_time(time);
    }

    /// Calculate how many samples need to be emitted based on current playback position
    /// and playback preload time.
    pub fn calculate_samples_to_emit(
        &self,
        time_base: &dyn SampleTimeBase,
        playback_start_sample_time: SampleTime,
        emitted_sample_time: SampleTime,
    ) -> SampleTime {
        // current, relative playback time
        let playback_head = self
            .inner
            .output_sample_frame_position()
            .saturating_sub(playback_start_sample_time);
        // current, relative emit time
        let emitter_head = playback_head as i64 - emitted_sample_time as i64;
        // apply 2 * preload: first half as buffer, second as fill area
        let preload_samples =
            time_base.seconds_to_samples(self.playback_preload_time.as_secs_f64());
        (emitter_head + 2 * preload_samples as i64).max(0) as SampleTime
    }

    /// Manually seek the given sequence to the given time offset and actual position.
    pub fn advance_until_time(&mut self, sequence: &mut Sequence, time: SampleTime) {
        self.stop_all_notes();
        sequence.advance_until_time(time);
    }

    /// Manually run the given sequence with the given time offset to the given time.
    /// When exchanging the sequence, call `prepare_run_until_time` before calling `run_until_time`.
    pub fn run_until_time(
        &mut self,
        sequence: &mut Sequence,
        time_offset: SampleTime,
        time: SampleTime,
    ) {
        // run sequence to the given time
        let time_base = *sequence.time_base();
        sequence.consume_events_until_time(time, &mut |pattern_index, pattern_event| {
            self.handle_pattern_event(pattern_index, pattern_event, time_base, time_offset);
        });
    }

    /// Handle pattern event note offs and new note actions only, skipping note-ons.
    fn handle_pattern_event_note_offs(
        &mut self,
        time_offset: u64,
        pattern_index: usize,
        pattern_event: PatternEvent,
    ) {
        if let Some(Event::NoteEvents(notes)) = &pattern_event.event {
            for (voice_index, note_event) in notes.iter().enumerate() {
                let note_event = match note_event {
                    None => continue,
                    Some(note_event) => note_event,
                };
                // Handle note off or stop action only
                if note_event.note.is_note_off()
                    || (note_event.note.is_note_on()
                        && note_event.glide.is_none()
                        && self.new_note_action != NewNoteAction::Continue)
                {
                    let stop_time = self.note_event_time(&pattern_event, note_event, time_offset);

                    if let Some(playing_note) =
                        self.playing_notes[pattern_index].get_mut(&voice_index)
                    {
                        if playing_note.stop_time.is_none_or(|time| time > stop_time) {
                            if let Some(Some(generator)) = self.generators.get(pattern_index) {
                                if let Err(err) =
                                    generator.note_off(playing_note.note_id, Some(stop_time))
                                {
                                    log::warn!("Failed to trigger note off: {err}");
                                }
                            }
                            playing_note.stop_time = Some(stop_time)
                        }
                    }
                }
            }
        }
    }

    /// Handle a single pattern event from the sequence
    fn handle_pattern_event(
        &mut self,
        pattern_index: usize,
        pattern_event: PatternEvent,
        time_base: BeatTimeBase,
        time_offset: SampleTime,
    ) {
        // Print event if enabled
        if self.show_events {
            const SHOW_INSTRUMENTS_AND_PARAMETERS: bool = true;
            println!(
                "{}: {}",
                time_base.display(pattern_event.time),
                match &pattern_event.event {
                    Some(event) => event.to_string(SHOW_INSTRUMENTS_AND_PARAMETERS),
                    None => "---".to_string(),
                }
            );
        }

        // Remove expired pending note stops
        self.playing_notes[pattern_index].retain(|_, playing_note| {
            playing_note
                .stop_time
                .is_none_or(|stop_time| stop_time >= pattern_event.time + time_offset)
        });

        // Process note events
        if let Some(Event::NoteEvents(notes)) = &pattern_event.event {
            for (voice_index, note_event) in notes.iter().enumerate() {
                let note_event = match note_event {
                    Some(note_event) => note_event,
                    None => continue,
                };
                // Handle note off or stop action
                if note_event.note.is_note_off()
                    || (note_event.note.is_note_on()
                        && note_event.glide.is_none()
                        && self.new_note_action != NewNoteAction::Continue)
                {
                    let stop_time = self.note_event_time(&pattern_event, note_event, time_offset);

                    if let Some(playing_note) =
                        self.playing_notes[pattern_index].get_mut(&voice_index)
                    {
                        if playing_note.stop_time.is_none_or(|time| time > stop_time) {
                            if let Some(Some(generator)) = self.generators.get(pattern_index) {
                                if let Err(err) =
                                    generator.note_off(playing_note.note_id, Some(stop_time))
                                {
                                    log::warn!("Failed to trigger note off: {err}");
                                }
                            }
                            playing_note.stop_time = Some(stop_time);
                        }
                    }
                }
                // Play new note
                if note_event.note.is_note_on() {
                    let start_time = self.note_event_time(&pattern_event, note_event, time_offset);
                    if note_event.glide.is_none()
                        || !self.play_glided_note(
                            pattern_index,
                            voice_index,
                            &pattern_event,
                            note_event,
                            start_time,
                        )
                    {
                        self.play_new_note(pattern_index, voice_index, note_event, start_time);
                    }
                }
            }
        }
    }

    // calculate absolute sample time from the given time_offset, applying note event delay.
    fn note_event_time(
        &self,
        pattern_event: &PatternEvent,
        note_event: &NoteEvent,
        time_offset: SampleTime,
    ) -> SampleTime {
        let delay = note_event.delay.clamp(0.0, 1.0);
        time_offset + pattern_event.time + (delay * pattern_event.duration as f32) as SampleTime
    }

    // convert given normalized glide value into a semitones per second based glide value.
    fn note_glide_value(
        glide: f32,
        source_note: Note,
        target_note: Note,
        samples_per_sec: u32,
        event_duration_in_samples: u64,
    ) -> f32 {
        let semitones = (target_note as u8 as f32 - source_note as u8 as f32).abs();
        if glide <= 0.0 || semitones == 0.0 || event_duration_in_samples == 0 {
            return f32::MAX;
        }
        let event_duration_in_seconds =
            (event_duration_in_samples as f64 / samples_per_sec as f64) as f32;
        semitones / event_duration_in_seconds / glide
    }

    fn play_glided_note(
        &mut self,
        pattern_index: usize,
        voice_index: usize,
        pattern_event: &PatternEvent,
        note_event: &crate::NoteEvent,
        start_time: SampleTime,
    ) -> bool {
        if let Some(playing_note) = self.playing_notes[pattern_index].get(&voice_index) {
            if playing_note.stop_time.is_none_or(|t| t > start_time) {
                let midi_note = (note_event.note as i32 + 60 - self.sample_root_note as i32)
                    .clamp(0, 127) as u8;
                let speed = phonic::utils::speed_from_note(midi_note);
                let volume = note_event.volume.max(0.0);
                let panning = note_event.panning.clamp(-1.0, 1.0);
                let glide = note_event.glide.unwrap_or(0.0).max(0.0);
                let semitones_per_sec_glide = Self::note_glide_value(
                    glide,
                    playing_note.note,
                    note_event.note,
                    self.inner.output_sample_rate(),
                    pattern_event.duration,
                );
                if let Some(Some(generator)) = self.generators.get(pattern_index) {
                    let note_id = playing_note.note_id;
                    return generator
                        .set_note_speed(note_id, speed, Some(semitones_per_sec_glide), start_time)
                        .and(generator.set_note_volume(note_id, volume, start_time))
                        .and(generator.set_note_panning(note_id, panning, start_time))
                        .is_ok();
                }
            }
        }
        // no note playing which can be glided
        false
    }

    fn play_new_note(
        &mut self,
        pattern_index: usize,
        voice_index: usize,
        note_event: &NoteEvent,
        start_time: SampleTime,
    ) {
        let midi_note =
            (note_event.note as i32 + 60 - self.sample_root_note as i32).clamp(0, 127) as u8;
        let volume = note_event.volume.max(0.0);
        let panning = note_event.panning.clamp(-1.0, 1.0);

        if let Some(Some(generator)) = self.generators.get(pattern_index) {
            // Trigger note on
            let context: Option<phonic::PlaybackStatusContext> = Some(Arc::new(PlaybackContext {
                note: Note::from(midi_note),
                pattern_index: Some(pattern_index),
                voice_index: Some(voice_index),
            }));
            match generator.note_on_with_context(
                midi_note,
                Some(volume),
                Some(panning),
                context,
                start_time,
            ) {
                Ok(note_id) => {
                    self.playing_notes[pattern_index].insert(
                        voice_index,
                        PlayingNote {
                            note_id,
                            note: note_event.note,
                            stop_time: None,
                        },
                    );
                }
                Err(err) => {
                    log::warn!("Failed to trigger note on: {err}");
                }
            }
        }
    }

    fn reset_playback_position(&mut self, sequence: &Sequence) {
        // stop whatever is playing in case we're restarting
        self.stop_all_notes();
        // rebuild playing notes vec
        self.playing_notes
            .resize_with(sequence.phrase_pattern_slot_count(), HashMap::new);
        // fetch player's actual position and use it as start offset
        self.playback_start_sample_time = self.inner.output_sample_frame_position();
        self.emitted_sample_time = 0;
    }
}

// -------------------------------------------------------------------------------------------------

/// Single playing note in a player pattern.
#[derive(Clone)]
struct PlayingNote {
    /// The note playback ID of the playing note.
    note_id: phonic::NotePlaybackId,
    /// The MIDI note value of the playing note.
    note: Note,
    /// Some, when a stop note is scheduled for the note.
    stop_time: Option<SampleTime>,
}
