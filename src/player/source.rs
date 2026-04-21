use crate::{
    phrase::PatternIndex, BeatTimeBase, Pattern, PatternEvent, Phrase, SampleTime, Sequence,
};

// -------------------------------------------------------------------------------------------------

/// Trait that [`PlayerSequencer`] uses to drive an event source.
///
/// Implemented for both [`Sequence`] and [`Phrase`].
pub trait SequencerSource {
    fn consume_events_until_time<F>(&mut self, time: SampleTime, consumer: &mut F)
    where
        F: FnMut(PatternIndex, PatternEvent);
    fn advance_until_time(&mut self, time: SampleTime);
    fn time_base(&self) -> BeatTimeBase;
    fn set_time_base(&mut self, time_base: &BeatTimeBase);
    fn reset(&mut self);
    fn pattern_slot_count(&self) -> usize;
}

impl SequencerSource for Sequence {
    fn consume_events_until_time<F>(&mut self, time: SampleTime, consumer: &mut F)
    where
        F: FnMut(PatternIndex, PatternEvent),
    {
        Sequence::consume_events_until_time(self, time, consumer)
    }
    fn advance_until_time(&mut self, time: SampleTime) {
        Sequence::advance_until_time(self, time)
    }
    fn time_base(&self) -> BeatTimeBase {
        *Sequence::time_base(self)
    }
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        Sequence::set_time_base(self, time_base)
    }
    fn reset(&mut self) {
        Sequence::reset(self)
    }
    fn pattern_slot_count(&self) -> usize {
        Sequence::phrase_pattern_slot_count(self)
    }
}

impl SequencerSource for Phrase {
    fn consume_events_until_time<F>(&mut self, time: SampleTime, consumer: &mut F)
    where
        F: FnMut(PatternIndex, PatternEvent),
    {
        Phrase::consume_events_until_time(self, time, consumer)
    }
    fn advance_until_time(&mut self, time: SampleTime) {
        Phrase::advance_until_time(self, time)
    }
    fn time_base(&self) -> BeatTimeBase {
        *Pattern::time_base(self)
    }
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        Pattern::set_time_base(self, time_base)
    }
    fn reset(&mut self) {
        Pattern::reset(self)
    }
    fn pattern_slot_count(&self) -> usize {
        self.pattern_slots().len()
    }
}
