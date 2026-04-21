//! Generic [`PlayerSequencer`] and [`PlayerHandle`] implementations.

use std::{collections::HashMap, sync::Arc};

use crate::{
    time::SampleTimeDisplay, BeatTimeBase, Event, Note, Phrase, SampleTime, SampleTimeBase,
    Sequence,
};

use super::{phonic, source::SequencerSource, PlaybackContext, PlayerNewNoteAction};

use phonic::generators::SequencerEventSink;

// -------------------------------------------------------------------------------------------------

/// A [`phonic::generators::Sequencer`] adapter that drives a [`SequencerSource`] and routes
/// note events to the registered generator handles (one per pattern slot).
///
/// Construct via [`Player::play_sequence`], [`Player::play_phrase`], or directly.
pub struct PlayerSequencer<S: SequencerSource> {
    generators: Vec<Option<phonic::GeneratorPlaybackHandle>>,
    start_sample_time: SampleTime,
    emitted_sample_time: SampleTime,
    playing_notes: Vec<HashMap<usize, PlayingNote>>,
    new_note_action: PlayerNewNoteAction,
    sample_root_note: Note,
    sample_rate: u32,
    show_events: bool,
    source: S,
    stopped: bool,
}

// SAFETY: PlayerSequencer is always driven via `&mut self` from a single thread; no
// concurrent shared access ever occurs. The bounds are required by the `phonic::generators::Sequencer`
// trait, but `S` (e.g. `Phrase` with `Rc<RefCell<…>>` pattern slots) may be `!Send`/`!Sync`.
unsafe impl<S: SequencerSource> Sync for PlayerSequencer<S> {}
unsafe impl<S: SequencerSource> Send for PlayerSequencer<S> {}

impl<S: SequencerSource> PlayerSequencer<S> {
    pub fn new(
        source: S,
        new_note_action: PlayerNewNoteAction,
        sample_root_note: Note,
        sample_rate: u32,
    ) -> Self {
        let slot_count = source.pattern_slot_count().max(1);
        Self {
            source,
            generators: vec![None; slot_count],
            playing_notes: vec![HashMap::new(); slot_count],
            new_note_action,
            sample_root_note,
            sample_rate,
            start_sample_time: 0,
            emitted_sample_time: 0,
            show_events: false,
            stopped: false,
        }
    }

    /// When `true`, each emitted event is printed to stdout.
    pub fn set_show_events(&mut self, show: bool) {
        self.show_events = show;
    }

    /// Set (or clear) the generator for the given pattern slot.
    pub fn set_generator(
        &mut self,
        pattern_index: usize,
        handle: Option<phonic::GeneratorPlaybackHandle>,
    ) {
        if pattern_index >= self.generators.len() {
            self.generators.resize(pattern_index + 1, None);
        }
        self.generators[pattern_index] = handle;
    }

    /// Insert a generator at `pattern_index`, shifting higher slots right.
    pub fn insert_generator(
        &mut self,
        pattern_index: usize,
        handle: phonic::GeneratorPlaybackHandle,
    ) {
        if pattern_index >= self.generators.len() {
            self.generators.resize(pattern_index, None);
        }
        self.generators.insert(pattern_index, Some(handle));
        if pattern_index >= self.playing_notes.len() {
            self.playing_notes.resize_with(pattern_index, HashMap::new);
        }
        self.playing_notes.insert(pattern_index, HashMap::new());
    }

    /// Remove the generator at `pattern_index`, shifting higher slots left.
    pub fn remove_generator(&mut self, pattern_index: usize) {
        if pattern_index < self.generators.len() {
            if let Some(generator) = self.generators[pattern_index].as_ref() {
                if let Some(notes) = self.playing_notes.get(pattern_index) {
                    for playing_note in notes.values() {
                        if let Err(err) = generator.note_off(playing_note.note_id, None) {
                            log::warn!("Failed to trigger note off: {err}");
                        }
                    }
                }
            }
            self.generators.remove(pattern_index);
            if pattern_index < self.playing_notes.len() {
                self.playing_notes.remove(pattern_index);
            }
        }
    }

    /// Move the generator slot at `from` to `to`, shifting intermediate slots accordingly.
    pub fn move_generator(&mut self, from: usize, to: usize) {
        if from == to || from >= self.generators.len() {
            return;
        }
        let generator_slot = self.generators.remove(from);
        let notes_item = if from < self.playing_notes.len() {
            self.playing_notes.remove(from)
        } else {
            HashMap::new()
        };
        if to >= self.generators.len() {
            self.generators.resize(to, None);
        }
        self.generators.insert(to, generator_slot);
        if to >= self.playing_notes.len() {
            self.playing_notes.resize_with(to, HashMap::new);
        }
        self.playing_notes.insert(to, notes_item);
    }

    /// Return a reference to the generator handle at `pattern_index`, if any.
    pub fn generator(&self, pattern_index: usize) -> Option<&phonic::GeneratorPlaybackHandle> {
        self.generators.get(pattern_index).and_then(Option::as_ref)
    }

    /// Return the last sample time that was emitted via [`phonic::generators::Sequencer::run_until`].
    pub fn current_time(&self) -> SampleTime {
        self.emitted_sample_time
    }

    /// Advance the sequencer's internal position to `sample_time` without emitting events.
    ///
    /// Useful when the animation frame loop falls too far behind and needs to skip ahead
    /// (e.g. after a browser tab is suspended).
    pub fn advance_to(&mut self, sample_time: SampleTime) {
        let relative_time = sample_time.saturating_sub(self.start_sample_time);
        if relative_time > self.emitted_sample_time {
            self.source.advance_until_time(relative_time);
            self.emitted_sample_time = relative_time;
        }
    }

    /// Stop all playing notes in the given pattern slot immediately.
    pub fn stop_notes_in_pattern_slot(&mut self, pattern_index: usize) {
        if let Some(notes) = self.playing_notes.get_mut(pattern_index) {
            for playing_note in notes.values() {
                if let Some(generator) = self.generators.get(pattern_index).and_then(Option::as_ref)
                {
                    if let Err(err) = generator.note_off(playing_note.note_id, None) {
                        log::warn!("Failed to trigger note off: {err}");
                    }
                }
            }
            notes.clear();
        }
    }

    /// Seamlessly swap to a new source, continuing from the current playback position.
    ///
    /// The old source is scanned ahead to find its upcoming note-offs, so only notes that
    /// have no scheduled stop after the scan are stopped at the new sample time. The new
    /// source is advanced to the current `emitted_sample_time` so playback is not interrupted.
    pub fn replace_source(&mut self, mut new_source: S) {
        let start_sample_time = self.start_sample_time;
        let current_time = self.emitted_sample_time;
        let abs_current_time = start_sample_time + current_time;

        // Scan the old source ahead to collect natural note-offs before swapping.
        // Look ahead by at least one bar or two seconds.
        let time_base = self.source.time_base();
        let lookahead_until = current_time
            + (time_base.samples_per_bar() as SampleTime).max(time_base.seconds_to_samples(2.0));

        let generators: &[Option<phonic::GeneratorPlaybackHandle>] = &self.generators;
        let playing_notes = &mut self.playing_notes;
        let new_note_action = self.new_note_action;

        self.source.consume_events_until_time(
            lookahead_until,
            &mut |pattern_index, pattern_event| {
                if let Some(Event::NoteEvents(notes)) = &pattern_event.event {
                    for (voice_index, note_event) in notes.iter().enumerate() {
                        let Some(note_event) = note_event else {
                            continue;
                        };
                        if note_event.note.is_note_off()
                            || (note_event.note.is_note_on()
                                && note_event.glide.is_none()
                                && new_note_action != PlayerNewNoteAction::Continue)
                        {
                            let delay = note_event.delay.clamp(0.0, 1.0);
                            let abs_stop = start_sample_time
                                + pattern_event.time
                                + (delay * pattern_event.duration as f32) as SampleTime;
                            if let Some(notes_map) = playing_notes.get_mut(pattern_index) {
                                if let Some(playing_note) = notes_map.get_mut(&voice_index) {
                                    if playing_note.stop_time.is_none_or(|t| t > abs_stop) {
                                        if let Some(generator) =
                                            generators.get(pattern_index).and_then(Option::as_ref)
                                        {
                                            let _ = generator
                                                .note_off(playing_note.note_id, Some(abs_stop));
                                        }
                                        playing_note.stop_time = Some(abs_stop);
                                    }
                                }
                            }
                        }
                    }
                }
            },
        );

        // Force-stop any notes that the scan didn't cover with a natural note-off.
        for (pattern_index, playing_notes) in self.playing_notes.iter_mut().enumerate() {
            for playing_note in playing_notes.values_mut() {
                if playing_note.stop_time.is_none() {
                    if let Some(generator) =
                        self.generators.get(pattern_index).and_then(Option::as_ref)
                    {
                        let _ = generator.note_off(playing_note.note_id, Some(abs_current_time));
                    }
                    playing_note.stop_time = Some(abs_current_time);
                }
            }
        }

        new_source.advance_until_time(self.emitted_sample_time);
        self.source = new_source;
        let slot_count = self.source.pattern_slot_count().max(1);
        self.playing_notes = vec![HashMap::new(); slot_count];
    }
}

impl PlayerSequencer<Sequence> {
    /// Access to the sequence.
    pub fn sequence(&self) -> &Sequence {
        &self.source
    }
    /// Mutable access to the sequence.
    pub fn sequence_mut(&mut self) -> &mut Sequence {
        &mut self.source
    }
    /// Seamlessly swap to a new sequence. Equivalent to [`Self::replace_source`].
    pub fn replace_sequence(&mut self, sequence: Sequence) {
        self.replace_source(sequence);
    }
}

impl PlayerSequencer<Phrase> {
    /// Access to the phrase.
    pub fn phrase(&self) -> &Phrase {
        &self.source
    }
    /// Mutable access to the phrase.
    pub fn phrase_mut(&mut self) -> &mut Phrase {
        &mut self.source
    }
    /// Seamlessly swap to a new phrase. Equivalent to [`Self::replace_source`].
    pub fn replace_phrase(&mut self, phrase: Phrase) {
        self.replace_source(phrase);
    }
}

// -------------------------------------------------------------------------------------------------

/// Single note actively playing in a [`PlayerSequencer`] voice slot.
#[derive(Clone)]
struct PlayingNote {
    note_id: phonic::NotePlaybackId,
    note: Note,
    stop_time: Option<SampleTime>,
}

// -------------------------------------------------------------------------------------------------

/// Routes note events to the generator registered for `current_pattern_index`, falling back to
/// `fallback` when no generator is registered for that slot.
///
/// Set `current_pattern_index` before processing each pattern slot's events.
struct PlayerEventSink<'a> {
    generators: &'a [Option<phonic::GeneratorPlaybackHandle>],
    current_pattern_index: usize,
    fallback: &'a mut dyn phonic::generators::SequencerEventSink,
}

impl phonic::generators::SequencerEventSink for PlayerEventSink<'_> {
    fn note_on(
        &mut self,
        note: u8,
        volume: Option<f32>,
        panning: Option<f32>,
        start_time: u64,
    ) -> Option<phonic::NotePlaybackId> {
        self.note_on_with_context(note, volume, panning, None, start_time)
    }

    fn note_on_with_context(
        &mut self,
        note: u8,
        volume: Option<f32>,
        panning: Option<f32>,
        context: Option<phonic::PlaybackStatusContext>,
        start_time: u64,
    ) -> Option<phonic::NotePlaybackId> {
        if let Some(generator) = self
            .generators
            .get(self.current_pattern_index)
            .and_then(Option::as_ref)
        {
            match generator.note_on_with_context(note, volume, panning, context, Some(start_time)) {
                Ok(note_id) => Some(note_id),
                Err(err) => {
                    log::warn!("note_on failed: {err}");
                    None
                }
            }
        } else {
            self.fallback
                .note_on_with_context(note, volume, panning, context, start_time)
        }
    }

    fn note_off(&mut self, note_id: phonic::NotePlaybackId, stop_time: u64) {
        if let Some(generator) = self
            .generators
            .get(self.current_pattern_index)
            .and_then(Option::as_ref)
        {
            let _ = generator.note_off(note_id, Some(stop_time));
        } else {
            self.fallback.note_off(note_id, stop_time);
        }
    }

    fn set_speed(
        &mut self,
        note_id: phonic::NotePlaybackId,
        speed: f64,
        glide: Option<f32>,
        sample_time: u64,
    ) {
        if let Some(generator) = self
            .generators
            .get(self.current_pattern_index)
            .and_then(Option::as_ref)
        {
            let _ = generator.set_note_speed(note_id, speed, glide, Some(sample_time));
        } else {
            self.fallback.set_speed(note_id, speed, glide, sample_time);
        }
    }

    fn set_volume(&mut self, note_id: phonic::NotePlaybackId, volume: f32, sample_time: u64) {
        if let Some(generator) = self
            .generators
            .get(self.current_pattern_index)
            .and_then(Option::as_ref)
        {
            let _ = generator.set_note_volume(note_id, volume, Some(sample_time));
        } else {
            self.fallback.set_volume(note_id, volume, sample_time);
        }
    }

    fn set_panning(&mut self, note_id: phonic::NotePlaybackId, panning: f32, sample_time: u64) {
        if let Some(generator) = self
            .generators
            .get(self.current_pattern_index)
            .and_then(Option::as_ref)
        {
            let _ = generator.set_note_panning(note_id, panning, Some(sample_time));
        } else {
            self.fallback.set_panning(note_id, panning, sample_time);
        }
    }
}

// -------------------------------------------------------------------------------------------------

impl<S: SequencerSource> phonic::generators::Sequencer for PlayerSequencer<S> {
    fn is_playing(&self) -> bool {
        !self.stopped
    }

    fn set_transport(&mut self, transport: phonic::Transport, _current_sample_time: u64) {
        let time_base = BeatTimeBase {
            beats_per_min: transport.beats_per_minute() as f32,
            beats_per_bar: transport.beats_per_bar() as u32,
            samples_per_sec: transport.sample_rate(),
        };
        self.sample_rate = transport.sample_rate();
        self.source.set_time_base(&time_base);
    }

    fn reset(&mut self, sample_time: u64) {
        self.source.reset();
        self.start_sample_time = sample_time;
        self.emitted_sample_time = 0;
        self.stopped = false;
        for notes in &mut self.playing_notes {
            notes.clear();
        }
    }

    fn run_until(
        &mut self,
        sample_time: u64,
        event_sink: &mut dyn phonic::generators::SequencerEventSink,
    ) {
        let relative_time = sample_time.saturating_sub(self.start_sample_time);
        if relative_time <= self.emitted_sample_time {
            return;
        }

        let start_sample_time = self.start_sample_time;
        let generators: &[Option<phonic::GeneratorPlaybackHandle>] = &self.generators;
        let playing_notes = &mut self.playing_notes;
        let new_note_action = self.new_note_action;
        let sample_root_note = self.sample_root_note;
        let sample_rate = self.sample_rate;
        let show_events = self.show_events;
        let time_base = self.source.time_base();

        let mut sink = PlayerEventSink {
            generators,
            current_pattern_index: 0,
            fallback: event_sink,
        };

        self.source.consume_events_until_time(
            relative_time,
            &mut |pattern_index, pattern_event| {
                if show_events {
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
                if playing_notes.len() <= pattern_index {
                    playing_notes.resize_with(pattern_index + 1, HashMap::new);
                }
                // Remove notes that have been explicitly stopped in a previous event
                playing_notes[pattern_index].retain(|_, playing_note| {
                    playing_note
                        .stop_time
                        .is_none_or(|t| t >= start_sample_time + pattern_event.time)
                });

                sink.current_pattern_index = pattern_index;

                if let Some(Event::NoteEvents(notes)) = &pattern_event.event {
                    for (voice_index, note_event) in notes.iter().enumerate() {
                        let Some(note_event) = note_event else {
                            continue;
                        };
                        let delay = note_event.delay.clamp(0.0, 1.0);
                        let abs_time = start_sample_time
                            + pattern_event.time
                            + (delay * pattern_event.duration as f32) as SampleTime;

                        // note off / stop-before-new-note
                        if note_event.note.is_note_off()
                            || (note_event.note.is_note_on()
                                && note_event.glide.is_none()
                                && new_note_action != PlayerNewNoteAction::Continue)
                        {
                            if let Some(playing_note) =
                                playing_notes[pattern_index].get_mut(&voice_index)
                            {
                                if playing_note.stop_time.is_none_or(|t| t > abs_time) {
                                    sink.note_off(playing_note.note_id, abs_time);
                                    playing_note.stop_time = Some(abs_time);
                                }
                            }
                        }

                        // note on
                        if note_event.note.is_note_on() {
                            let midi_note = (note_event.note as i32 + 60 - sample_root_note as i32)
                                .clamp(0, 127) as u8;
                            let volume = note_event.volume.max(0.0);
                            let panning = note_event.panning.clamp(-1.0, 1.0);

                            // try glide: only possible when a generator is registered for this slot
                            let glided = generators
                                .get(pattern_index)
                                .and_then(Option::as_ref)
                                .is_some()
                                && note_event.glide.is_some_and(|glide| {
                                    if let Some(playing_note) =
                                        playing_notes[pattern_index].get(&voice_index)
                                    {
                                        if playing_note.stop_time.is_none_or(|t| t > abs_time) {
                                            let speed = phonic::utils::speed_from_note(midi_note);
                                            let semitones = (note_event.note as u8 as f32
                                                - playing_note.note as u8 as f32)
                                                .abs();
                                            let semitones_per_sec = if glide > 0.0
                                                && semitones > 0.0
                                                && pattern_event.duration > 0
                                            {
                                                let dur_secs = pattern_event.duration as f32
                                                    / sample_rate as f32;
                                                semitones / dur_secs / glide
                                            } else {
                                                f32::MAX
                                            };
                                            sink.set_speed(
                                                playing_note.note_id,
                                                speed,
                                                Some(semitones_per_sec),
                                                abs_time,
                                            );
                                            sink.set_volume(playing_note.note_id, volume, abs_time);
                                            sink.set_panning(
                                                playing_note.note_id,
                                                panning,
                                                abs_time,
                                            );
                                            true
                                        } else {
                                            false
                                        }
                                    } else {
                                        false
                                    }
                                });

                            if glided {
                                // update tracked note so future glides use the correct pitch
                                if let Some(playing_note) =
                                    playing_notes[pattern_index].get_mut(&voice_index)
                                {
                                    playing_note.note = note_event.note;
                                }
                            } else {
                                let context: Option<phonic::PlaybackStatusContext> =
                                    Some(Arc::new(PlaybackContext {
                                        note: Note::from(midi_note),
                                        pattern_index: Some(pattern_index),
                                        voice_index: Some(voice_index),
                                    }));
                                if let Some(note_id) = sink.note_on_with_context(
                                    midi_note,
                                    Some(volume),
                                    Some(panning),
                                    context,
                                    abs_time,
                                ) {
                                    playing_notes[pattern_index].insert(
                                        voice_index,
                                        PlayingNote {
                                            note_id,
                                            note: note_event.note,
                                            stop_time: None,
                                        },
                                    );
                                }
                            }
                        }
                    }
                }
            },
        );

        self.emitted_sample_time = relative_time;
    }

    fn stop(
        &mut self,
        sample_time: u64,
        event_sink: &mut dyn ::phonic::generators::SequencerEventSink,
    ) {
        // Stop all playing notes which have no stop note scheduled yet
        for (pattern_index, playing_notes) in self.playing_notes.iter_mut().enumerate() {
            for playing_note in playing_notes.values_mut() {
                // Is a note stop already scheduled?
                if playing_note.stop_time.is_none() {
                    if let Some(generator) =
                        self.generators.get(pattern_index).and_then(Option::as_ref)
                    {
                        let _ = generator.note_off(playing_note.note_id, Some(sample_time));
                    } else {
                        event_sink.note_off(playing_note.note_id, sample_time);
                    }
                    playing_note.stop_time = Some(sample_time);
                }
            }
        }
        // Mark as stopped
        self.stopped = true;
    }
}
