//! Arrange `Phrase`s into a playback sequence.

use crate::{phrase::PatternIndex, BeatTimeBase, Pattern, PatternEvent, Phrase, SampleTime};

// -------------------------------------------------------------------------------------------------

/// Sequentially arrange [`Phrase`]s to form simple arrangements.
#[derive(Clone, Debug)]
pub struct Sequence {
    time_base: BeatTimeBase,
    phrases: Vec<Phrase>,
    phrase_index: usize,
    sample_position_in_phrase: SampleTime,
    sample_position: SampleTime,
    sample_offset: SampleTime,
}

impl Sequence {
    /// Create a new sequence from a vector of [`Phrase`]s.
    pub fn new(time_base: BeatTimeBase, phrases: Vec<Phrase>) -> Self {
        let phrase_index = 0;
        let sample_position_in_phrase = 0;
        let sample_position = 0;
        let sample_offset = 0;
        Self {
            time_base,
            phrases,
            phrase_index,
            sample_position_in_phrase,
            sample_position,
            sample_offset,
        }
    }

    /// Read-only access to our beat time base.
    pub fn time_base(&self) -> &BeatTimeBase {
        &self.time_base
    }

    /// Update the sequence's internal time bases with a new time base.
    pub fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        self.time_base = *time_base;
        for phrase in &mut self.phrases {
            phrase.set_time_base(time_base);
        }
    }

    /// Read-only access to the currently played back phrase.
    pub fn current_phrase(&self) -> &Phrase {
        &self.phrases[self.phrase_index]
    }

    /// Read-only access to the currently played back phrase.
    pub fn current_phrase_mut(&mut self) -> &mut Phrase {
        &mut self.phrases[self.phrase_index]
    }

    /// Read-only access to all phrases.
    pub fn phrases(&self) -> &[Phrase] {
        &self.phrases
    }

    /// Mut access to all phrases.
    pub fn phrases_mut(&mut self) -> &mut [Phrase] {
        &mut self.phrases
    }

    /// returns maximum pattern count in all phrases.
    pub fn phrase_pattern_slot_count(&self) -> usize {
        let mut count = 0;
        for phrase in &self.phrases {
            count = count.max(phrase.pattern_slots().len());
        }
        count
    }

    /// Run patterns until a given sample time is reached, calling the given `visitor`
    /// function for all emitted events to consume emitted events.
    pub fn consume_events_until_time<F>(&mut self, time: SampleTime, consumer: &mut F)
    where
        F: FnMut(PatternIndex, PatternEvent),
    {
        debug_assert!(time >= self.sample_position, "can not rewind playback here");
        while time - self.sample_position > 0 {
            let (next_phrase_start, samples_to_run) = self.samples_until_next_phrase(time);
            if next_phrase_start <= samples_to_run {
                // run current phrase until it ends
                let sample_position = self.sample_position;
                self.current_phrase_mut()
                    .consume_events_until_time(sample_position + next_phrase_start, consumer);
                // select next phrase in the sequence
                let previous_phrase = self.current_phrase_mut().clone();
                self.phrase_index = (self.phrase_index + 1) % self.phrases().len();
                self.sample_position_in_phrase = 0;
                self.sample_position += next_phrase_start;
                // reset the new phrase or apply continues modes
                if self.phrases().len() > 1 {
                    let sample_offset = self.sample_position;
                    self.current_phrase_mut()
                        .reset_with_offset(sample_offset, &previous_phrase);
                }
            } else {
                // keep running the current phrase
                let sample_position = self.sample_position;
                self.current_phrase_mut()
                    .consume_events_until_time(sample_position + samples_to_run, consumer);
                self.sample_position_in_phrase += samples_to_run;
                self.sample_position += samples_to_run;
            }
        }
    }

    /// Move sequence playback head to the given sample time, ignoring all events.
    pub fn advance_until_time(&mut self, sample_time: SampleTime) {
        debug_assert!(
            sample_time >= self.sample_position,
            "can not rewind playback here"
        );
        while sample_time - self.sample_position > 0 {
            let (next_phrase_start, samples_to_run) = self.samples_until_next_phrase(sample_time);
            if next_phrase_start <= samples_to_run {
                // run current phrase until it ends
                let sample_position = self.sample_position;
                self.current_phrase_mut()
                    .advance_until_time(sample_position + next_phrase_start);
                // select next phrase in the sequence
                let previous_phrase = self.current_phrase_mut().clone();
                self.phrase_index = (self.phrase_index + 1) % self.phrases().len();
                self.sample_position_in_phrase = 0;
                self.sample_position += next_phrase_start;
                // reset the new phrase or apply continues modes
                if self.phrases().len() > 1 {
                    let sample_offset = self.sample_position;
                    self.current_phrase_mut()
                        .reset_with_offset(sample_offset, &previous_phrase);
                }
            } else {
                // keep running the current phrase
                let sample_position = self.sample_position;
                self.current_phrase_mut()
                    .advance_until_time(sample_position + samples_to_run);
                self.sample_position_in_phrase += samples_to_run;
                self.sample_position += samples_to_run;
            }
        }
    }

    /// Reset phrases to their initial state.
    pub fn reset(&mut self) {
        // reset sample offset
        self.sample_offset = 0;
        // reset our own iter state
        self.sample_position = 0;
        self.sample_position_in_phrase = 0;
        // reset all our phrase iters
        for phrase in &mut self.phrases {
            phrase.reset();
        }
    }

    fn samples_until_next_phrase(&self, time: u64) -> (u64, u64) {
        let phrase_length_in_samples =
            self.current_phrase().length().to_samples(&self.time_base) as SampleTime;
        let next_phrase_start = phrase_length_in_samples - self.sample_position_in_phrase;
        let samples_to_run = time - self.sample_position;
        (next_phrase_start, samples_to_run)
    }
}
