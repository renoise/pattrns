use std::time::Duration;

use phonic::generators::Sequencer;

use super::{sequencer::PlayerSequencer, source::SequencerSource};
use crate::{Phrase, Sequence};

// -------------------------------------------------------------------------------------------------

/// Handle returned by [`Player::play_sequence`] and [`Player::play_phrase`].
///
/// Call [`PlayerHandle::run`] every iteration of your main loop, passing the current audio position
/// from `player.inner().output_sample_frame_position()` as reference time.
pub struct PlayerHandle<S: SequencerSource> {
    sequencer: PlayerSequencer<S>,
    preload_samples: u64,
    last_run_time: u64,
    sleep_duration: Duration,
}

impl<S: SequencerSource> PlayerHandle<S> {
    /// Create a new sequence playback handle.
    pub fn new(
        sequencer: PlayerSequencer<S>,
        preload_time: Duration,
        start_time: u64,
        sample_rate: u32,
    ) -> Self {
        let last_run_time = start_time;

        let preload_samples = (preload_time.as_secs_f64() * sample_rate as f64) as u64;
        let sleep_duration = preload_time / 2;

        Self {
            sequencer,
            preload_samples,
            last_run_time,
            sleep_duration,
        }
    }

    /// Drive the sequencer to `now` and return the recommended sleep duration.
    ///
    /// Pass `player.inner().output_sample_frame_position()` as reference time.
    ///
    /// returns the recommended sleep duration (half the preload window) so you can pass it directly
    /// to `thread::sleep` before probably checking, doing other idle stuff.
    pub fn run(&mut self, now: u64) -> Duration {
        let start = std::time::Instant::now();
        if now > self.last_run_time {
            self.sequencer.advance_to(now);
        }
        let mut noop = phonic::generators::SequencerNoopEventSink;
        self.sequencer
            .run_until(now + self.preload_samples, &mut noop);
        self.last_run_time = now + self.preload_samples;
        self.sleep_duration.saturating_sub(start.elapsed())
    }

    /// Stop playback, sending note-offs for all currently playing notes.
    pub fn stop(&mut self) {
        let mut noop = phonic::generators::SequencerNoopEventSink;
        self.sequencer.stop(self.last_run_time, &mut noop);
    }

    /// Seamlessly swap to a new source without stopping playback.
    ///
    /// The new source is advanced to the current playback position so the beat continues
    /// uninterrupted. Any notes still playing are stopped at the transition sample.
    pub fn swap(&mut self, source: S) {
        self.sequencer.replace_source(source);
    }

    /// Set (or clear) the generator for the given pattern slot in the running sequencer.
    pub fn set_generator(
        &mut self,
        pattern_index: usize,
        handle: Option<phonic::GeneratorPlaybackHandle>,
    ) {
        self.sequencer.set_generator(pattern_index, handle);
    }
}

impl PlayerHandle<Sequence> {
    /// Seamlessly swap to a new sequence without stopping playback.
    pub fn swap_sequence(&mut self, sequence: Sequence) {
        self.swap(sequence);
    }
}

impl PlayerHandle<Phrase> {
    /// Seamlessly swap to a new phrase without stopping playback.
    pub fn swap_phrase(&mut self, phrase: Phrase) {
        self.swap(phrase);
    }
}
