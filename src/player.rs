//! Example player implementation, which plays back a [`Sequence`] or [`Phrase`]
//! via the [`phonic`](https://crates.io/crates/phonic) crate.

use std::{sync::mpsc, time::Duration};

use crate::{Note, Phrase, Sequence};

// -------------------------------------------------------------------------------------------------

mod handle;
mod sequencer;
mod source;

pub use handle::PlayerHandle;
pub use sequencer::PlayerSequencer;
pub use source::SequencerSource;

// -------------------------------------------------------------------------------------------------

/// [`phonic`](https://crates.io/crates/phonic) mixer, generators and effects.
pub mod phonic {
    pub use phonic::*;
}

// -------------------------------------------------------------------------------------------------

/// Player's behavior when playing a new note on the same voice channel.
#[derive(Copy, Clone, Debug, Default, PartialEq)]
pub enum PlayerNewNoteAction {
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

/// An example player implementation using [`phonic`](https://crates.io/crates/phonic),
/// which plays back a [`Sequence`] or [`Phrase`] using the default audio output device.
///
/// To create and use mixers, custom sequencers and DSP effects, use the [`Self::inner_mut`]
/// function to access the underlying phonic player.
pub struct Player {
    inner: phonic::Player,
    generators: Vec<Option<phonic::GeneratorPlaybackHandle>>,
    new_note_action: PlayerNewNoteAction,
    sample_root_note: Note,
    show_events: bool,
    playback_preload_time: Duration,
}

impl Player {
    /// Default preload time of the player's `run_until` function.
    /// Quite high by default and bigger in slower debug builds.
    const DEFAULT_PLAYBACK_PRELOAD_MS: u64 = if cfg!(debug_assertions) { 500 } else { 250 };

    /// Create a new sample player.
    ///
    /// # Errors
    /// returns an error if the player could not be created.
    pub fn new<S: Into<Option<mpsc::SyncSender<phonic::PlaybackStatusEvent>>>>(
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
    pub fn new_with_config<S: Into<Option<mpsc::SyncSender<phonic::PlaybackStatusEvent>>>>(
        output_device: phonic::DefaultOutputDevice,
        playback_status_sender: S,
        config: phonic::PlayerConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let inner = phonic::Player::new_with_config(output_device, playback_status_sender, config);

        Ok(Self {
            inner,
            generators: Vec::new(),
            new_note_action: PlayerNewNoteAction::default(),
            sample_root_note: Note::C5,
            show_events: false,
            playback_preload_time: Duration::from_millis(Self::DEFAULT_PLAYBACK_PRELOAD_MS),
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

    /// Get current new note action behavior.
    pub fn new_note_action(&self) -> PlayerNewNoteAction {
        self.new_note_action
    }
    // Set a new new note action behavior.
    pub fn set_new_note_action(&mut self, action: PlayerNewNoteAction) {
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

    /// Sets a generator for the given pattern index.
    ///
    /// If a generator already exists at this index, it gets replaced with the new one.
    pub fn set_generator(
        &mut self,
        pattern_index: usize,
        generator: Box<dyn phonic::Generator>,
        mixer_id: Option<phonic::MixerId>,
    ) -> Result<phonic::GeneratorPlaybackHandle, phonic::Error> {
        if let Some(Some(prev_handle)) = self.generators.get(pattern_index) {
            let _ = self.inner.remove_generator(prev_handle.id());
        }
        let handle = self.inner.add_generator(generator, mixer_id)?;
        if pattern_index >= self.generators.len() {
            self.generators.resize(pattern_index + 1, None);
        }
        self.generators[pattern_index] = Some(handle.clone());
        Ok(handle)
    }

    /// Clears the generator at the given index.
    pub fn clear_generator(&mut self, pattern_index: usize) {
        if let Some(Some(handle)) = self.generators.get(pattern_index) {
            if let Err(err) = self.inner.remove_generator(handle.id()) {
                log::warn!("Failed to remove generator: '{err}'");
            }
        }
        if let Some(handle_slot) = self.generators.get_mut(pattern_index) {
            *handle_slot = None;
        }
    }

    /// Play a [`Sequence`] on a background thread, using the generators registered with this player.
    ///
    /// The returned [`PlayerHandle<Sequence>`] can be used to stop playback or swap to a new
    /// sequence seamlessly. Dropping the handle also stops the background thread.
    pub fn play_sequence(&mut self, sequence: Sequence) -> PlayerHandle<Sequence> {
        self.play_source(sequence)
    }

    /// Play a single [`Phrase`] on a background thread, using the generators registered with this player.
    ///
    /// The returned [`PlayerHandle<Phrase>`] can be used to stop playback or swap to a new
    /// phrase seamlessly. Dropping the handle also stops the background thread.
    pub fn play_phrase(&mut self, phrase: Phrase) -> PlayerHandle<Phrase> {
        self.play_source(phrase)
    }

    fn play_source<S: SequencerSource>(&mut self, source: S) -> PlayerHandle<S> {
        use phonic::generators::Sequencer as _;

        // Stop any notes left over from a previous sequence
        self.stop_all_notes();

        let mut sequencer = PlayerSequencer::new(
            source,
            self.new_note_action,
            self.sample_root_note,
            self.inner.output_sample_rate(),
        );
        // Seed sequencer from current generator slots
        for (pattern_index, handle) in self.generators.iter().enumerate() {
            if let Some(handle) = handle {
                sequencer.set_generator(pattern_index, Some(handle.clone()));
            }
        }
        sequencer.set_show_events(self.show_events);

        let start_time = self.inner.output_sample_frame_position();
        sequencer.reset(start_time);

        PlayerHandle::new(
            sequencer,
            self.playback_preload_time,
            start_time,
            self.inner.output_sample_rate(),
        )
    }

    /// Stop all currently playing notes in all generators.
    pub fn stop_all_notes(&mut self) {
        for handle in self.generators.iter().flatten() {
            if let Err(err) = handle.all_notes_off(None) {
                log::warn!("Failed to trigger all notes off: {err}");
            }
        }
        if let Err(err) = self.inner.stop_all_sources() {
            log::warn!("Failed to trigger stop all sources event: {err}");
        }
    }
}
