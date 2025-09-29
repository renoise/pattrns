//! Stack multiple `Pattern`s into a single pattern.

use std::{cell::RefCell, cmp::Ordering, fmt::Debug, rc::Rc};

use crate::{
    BeatTimeBase, BeatTimeStep, Event, EventTransform, ExactSampleTime, Parameter, ParameterSet,
    Pattern, PatternEvent, SampleTime,
};

// -------------------------------------------------------------------------------------------------

/// A single slot in a [`Phrase`] vector.
#[derive(Clone, Debug, Default)]
pub enum PatternSlot {
    /// Stop previous playing pattern and/or simply play nothing. This can be useful to
    /// create empty placeholder slots in e.g. a [`Sequence`][crate::Sequence].
    #[default]
    Stop,
    /// Continue playing a previously played pattern in a [`Sequence`][crate::Sequence].
    Continue,
    /// Play a shared pattern in this slot. NB: This is a shared reference, in order to
    /// resolve 'Continue' modes in a [`Sequence`](crate::Sequence).
    Pattern(Rc<RefCell<dyn Pattern>>),
}

/// Convert an unboxed [`Pattern`] to a [`PatternSlot`]
impl<R> From<R> for PatternSlot
where
    R: Pattern + 'static,
{
    fn from(pattern: R) -> PatternSlot {
        PatternSlot::Pattern(Rc::new(RefCell::new(pattern)))
    }
}

/// Convert a shared [`Pattern`] to a [`PatternSlot`]
impl From<Rc<RefCell<dyn Pattern>>> for PatternSlot {
    fn from(pattern: Rc<RefCell<dyn Pattern>>) -> PatternSlot {
        PatternSlot::Pattern(pattern)
    }
}

// -------------------------------------------------------------------------------------------------

/// Pattern index in `PhraseEvent`.
pub type PatternIndex = usize;
/// Event as emitted by the Phrase, tagged with an additional pattern index.
pub type PhraseEvent = (PatternIndex, PatternEvent);

// -------------------------------------------------------------------------------------------------

/// Combines multiple [`Pattern`]s into a new pattern stack.
#[derive(Clone)]
pub struct Phrase {
    time_base: BeatTimeBase,
    length: BeatTimeStep,
    parameters: ParameterSet,
    pattern_slots: Vec<PatternSlot>,
    next_events: Vec<Option<PhraseEvent>>,
    event_transform: Option<EventTransform>,
    sample_offset: SampleTime,
}

impl Debug for Phrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Phrase")
            .field("time_base", &self.time_base)
            .field("length", &self.length)
            .field("parameters", &self.parameters)
            .field("pattern_slots", &self.pattern_slots)
            // Skip event_transform, which has no Debug impl and next_events to reduce noise
            .field("sample_offset", &self.sample_offset)
            .finish()
    }
}

impl Phrase {
    /// Create a new phrase from a vector of [`PatternSlot`]s and the given default length.
    /// NB: `PatternSlot` has `Into` implementations, so you can also pass a vector of
    /// boxed or raw pattern instance here too.
    pub fn new<P: Into<PatternSlot>>(
        time_base: BeatTimeBase,
        pattern_slots: Vec<P>,
        length: BeatTimeStep,
    ) -> Self {
        let pattern_slots = pattern_slots
            .into_iter()
            .map(|r| r.into())
            .collect::<Vec<PatternSlot>>();
        // collect input parameters from all slots
        let mut parameters = ParameterSet::new();
        for slot in &pattern_slots {
            if let PatternSlot::Pattern(pattern) = slot {
                let pattern = (**pattern).borrow();
                for param in pattern.parameters() {
                    // silently skip duplicate parameter ids
                    if !parameters
                        .iter()
                        .any(|p| p.borrow().id() == param.borrow().id())
                    {
                        parameters.push(Rc::clone(param));
                    }
                }
            }
        }
        let next_events = vec![None; pattern_slots.len()];
        let event_transform = None;
        let sample_offset = 0;
        Self {
            time_base,
            length,
            parameters,
            pattern_slots,
            next_events,
            event_transform,
            sample_offset,
        }
    }

    /// Read-only access to our phrase length.
    /// This is applied in [`Sequence`][crate::Sequence] only.
    pub fn length(&self) -> BeatTimeStep {
        self.length
    }

    /// Read-only access to our pattern slots.
    pub fn pattern_slots(&self) -> &[PatternSlot] {
        &self.pattern_slots
    }

    /// Mut access to our pattern slots.
    pub fn pattern_slots_mut(&mut self) -> &mut [PatternSlot] {
        &mut self.pattern_slots
    }

    /// Run patterns to generate the next pending event, if any, calling the given `consumer`
    /// visitor function with the emitted event.
    pub fn consume_event<F>(&mut self, consumer: &mut F)
    where
        F: FnMut(PatternIndex, PatternEvent),
    {
        // emit and consume next events until we've reached the desired sample_time
        if let Some((pattern_index, mut pattern_event)) =
            self.next_event_until_time(SampleTime::MAX)
        {
            self.apply_event_transform(&mut pattern_event);
            consumer(pattern_index, pattern_event);
        }
    }

    /// Run patterns until a given sample time is reached, calling the given `consumer`
    /// visitor function for all emitted events.
    pub fn consume_events_until_time<F>(&mut self, sample_time: SampleTime, consumer: &mut F)
    where
        F: FnMut(PatternIndex, PatternEvent),
    {
        // emit and consume next events until we've reached the desired sample_time
        while let Some((pattern_index, mut pattern_event)) = self.next_event_until_time(sample_time)
        {
            debug_assert!(pattern_event.time < sample_time);
            self.apply_event_transform(&mut pattern_event);
            consumer(pattern_index, pattern_event);
        }
    }

    /// Move patterns until a given sample time is reached, ignoring all events until that time.
    pub fn advance_until_time(&mut self, sample_time: SampleTime) {
        // skip next events in all patterns
        for (pattern_slot, next_event) in self
            .pattern_slots
            .iter_mut()
            .zip(self.next_events.iter_mut())
        {
            // skip cached, next due events
            if let Some((_, event)) = next_event {
                if event.time >= sample_time {
                    // cached event is not yet due: no need to advance the slot
                    continue;
                }
                *next_event = None;
            }
            if let PatternSlot::Pattern(pattern) = pattern_slot {
                pattern.borrow_mut().advance_until_time(sample_time);
            }
        }
    }

    /// reset playback status and shift events to the given sample position.
    /// Further take over patterns from the passed previously playing phrase for `PatternSlot::Continue` slots.   
    pub fn reset_with_offset(&mut self, sample_offset: SampleTime, previous_phrase: &Phrase) {
        // reset pattern iters, unless they are in continue mode. in continue mode, copy the slot
        // from the previously playing phrase and adjust sample offsets to fit.
        for (pattern_index, pattern_slot) in self.pattern_slots.iter_mut().enumerate() {
            match pattern_slot {
                PatternSlot::Pattern(pattern) => {
                    {
                        let mut pattern = pattern.borrow_mut();
                        pattern.reset();
                        pattern.set_sample_offset(sample_offset);
                    }
                    self.next_events[pattern_index] = None;
                }
                PatternSlot::Stop => {
                    self.next_events[pattern_index] = None;
                }
                PatternSlot::Continue => {
                    // take over pending events
                    self.next_events[pattern_index]
                        .clone_from(&previous_phrase.next_events[pattern_index]);
                    // take over pattern
                    pattern_slot.clone_from(&previous_phrase.pattern_slots[pattern_index]);
                }
            }
        }
    }

    /// Apply custom event transform function, if any, to all emitted events.
    fn apply_event_transform(&self, pattern_event: &mut PatternEvent) {
        if let Some(transform) = &self.event_transform {
            if let Some(event) = &mut pattern_event.event {
                transform(event);
            }
        }
    }

    fn next_event_until_time(&mut self, sample_time: SampleTime) -> Option<PhraseEvent> {
        // fetch next events in all patterns
        for (pattern_index, (pattern_slot, next_event)) in self
            .pattern_slots
            .iter_mut()
            .zip(self.next_events.iter_mut())
            .enumerate()
        {
            if !next_event.is_some() {
                match pattern_slot {
                    // NB: Continue mode is resolved by the Sequence - if not, it should behave like Stop
                    PatternSlot::Stop | PatternSlot::Continue => *next_event = None,
                    PatternSlot::Pattern(pattern) => {
                        if let Some(event) = pattern.borrow_mut().run_until_time(sample_time) {
                            *next_event = Some((pattern_index, event));
                        } else {
                            *next_event = None;
                        }
                    }
                }
            }
        }
        // select the next from all pre-fetched events with the smallest sample time
        let next_due = self.next_events.iter_mut().reduce(|min, next| {
            if let Some((_, min_event)) = min {
                if let Some((_, next_event)) = next {
                    match min_event.time.cmp(&next_event.time) {
                        Ordering::Less | Ordering::Equal => min,
                        Ordering::Greater => next,
                    }
                } else {
                    min
                }
            } else {
                next
            }
        });
        if let Some(next_due) = next_due {
            if let Some((pattern_index, event)) = next_due.clone() {
                if event.time < sample_time {
                    *next_due = None; // consume
                    Some((pattern_index, event.with_offset(self.sample_offset)))
                } else {
                    None // not yet due
                }
            } else {
                None // no event available
            }
        } else {
            None
        }
    }
}

impl Pattern for Phrase {
    fn time_base(&self) -> &BeatTimeBase {
        &self.time_base
    }
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        self.time_base.clone_from(time_base);
        for pattern_slot in &mut self.pattern_slots {
            if let PatternSlot::Pattern(pattern) = pattern_slot {
                pattern.borrow_mut().set_time_base(time_base);
            }
        }
    }

    fn step_length(&self) -> ExactSampleTime {
        // use our length's step, likely won't be used anyway for phrases
        self.length.samples_per_step(&self.time_base)
    }
    fn step_count(&self) -> usize {
        // use our length's step, likely won't be used anyway for phrases
        self.length.steps() as usize
    }

    fn parameters(&self) -> &[Rc<RefCell<Parameter>>] {
        &self.parameters
    }

    fn set_trigger_event(&mut self, event: &Event) {
        for pattern_slot in &mut self.pattern_slots {
            if let PatternSlot::Pattern(pattern) = pattern_slot {
                pattern.borrow_mut().set_trigger_event(event);
            }
        }
    }

    fn set_event_transform(&mut self, transform: Option<EventTransform>) {
        self.event_transform = transform;
    }

    fn sample_offset(&self) -> SampleTime {
        self.sample_offset
    }
    fn set_sample_offset(&mut self, sample_offset: SampleTime) {
        self.sample_offset = sample_offset;
    }

    fn run_until_time(&mut self, sample_time: SampleTime) -> Option<PatternEvent> {
        self.next_event_until_time(sample_time)
            .map(|(_, event)| event)
    }

    fn advance_until_time(&mut self, sample_time: SampleTime) {
        self.advance_until_time(sample_time)
    }

    fn duplicate(&self) -> Rc<RefCell<dyn Pattern>> {
        Rc::new(RefCell::new(self.clone()))
    }

    fn reset(&mut self) {
        // reset sample offset
        self.sample_offset = 0;
        // reset iterator state
        self.next_events.fill(None);
        // reset all patterns in all slots as well
        for pattern_slot in &mut self.pattern_slots {
            if let PatternSlot::Pattern(pattern) = pattern_slot {
                pattern.borrow_mut().reset();
            }
        }
    }
}

/// Custom iterator impl for phrases:
/// returning a tuple of the pattern index and the pattern event.
impl Iterator for Phrase {
    type Item = PhraseEvent;
    fn next(&mut self) -> Option<Self::Item> {
        self.next_event_until_time(SampleTime::MAX)
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use std::rc::Rc;

    use crate::prelude::*;

    fn create_phrase() -> Result<Phrase, String> {
        let beat_time = BeatTimeBase {
            samples_per_sec: 44100,
            beats_per_min: 130.0,
            beats_per_bar: 4,
        };

        let seed = 12312312312_u64;
        let kick_cycle = new_cycle_emitter_with_seed(
            "bd? [~ bd] ~ ~ bd [~ bd] _ ~ bd? [~ bd] ~ ~ bd [~ bd] [_ bd2] [~ bd _ ~]",
            seed,
        )?;
        let mut kick_pattern = beat_time.every_nth_beat(16.0).emit(kick_cycle);
        kick_pattern.set_sample_offset(20); // test with offsets

        let snare_pattern = beat_time
            .every_nth_beat(2.0)
            .with_offset(BeatTimeStep::Beats(1.0))
            .with_event_transform(Rc::new(|event| {
                if let Event::NoteEvents(notes) = event {
                    for note in notes.iter_mut().flatten() {
                        note.note = Note::D4;
                    }
                }
            }))
            .emit(new_note_emitter("C_5"));

        let hihat_pattern =
            beat_time
                .every_nth_sixteenth(2.0)
                .emit(new_note_emitter("C_5").mutate({
                    let mut step = 0;
                    move |event| {
                        if let Event::NoteEvents(notes) = event {
                            for note in notes.iter_mut().flatten() {
                                note.volume = 1.0 / (step + 1) as f32;
                                step += 1;
                                if step >= 3 {
                                    step = 0;
                                }
                            }
                        }
                    }
                }));

        let hihat_pattern2 = beat_time
            .every_nth_sixteenth(2.0)
            .with_offset(BeatTimeStep::Sixteenth(1.0))
            .emit(new_note_emitter("C_5").mutate({
                let mut vel_step = 0;
                let mut note_step = 0;
                move |event| {
                    if let Event::NoteEvents(notes) = event {
                        for note in notes.iter_mut().flatten() {
                            note.volume = 1.0 / (vel_step + 1) as f32 * 0.5;
                            vel_step += 1;
                            if vel_step >= 3 {
                                vel_step = 0;
                            }
                            note.note = Note::from((Note::C4 as u8) + 32 - note_step);
                            note_step += 1;
                            if note_step >= 32 {
                                note_step = 0;
                            }
                        }
                    }
                }
            }));

        let hihat_pattern = Phrase::new(
            beat_time,
            vec![hihat_pattern, hihat_pattern2],
            BeatTimeStep::Bar(4.0),
        );

        let bass_notes = Scale::try_from((Note::C5, "aeolian")).unwrap().notes();
        let bass_pattern = beat_time
            .every_nth_eighth(1.0)
            .with_rhythm([1, 0, 1, 0, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1].to_rhythm())
            .emit(new_note_sequence_emitter(vec![
                new_note((bass_notes[0], None, None, 0.5)),
                new_note((bass_notes[2], None, None, 0.5)),
                new_note((bass_notes[3], None, None, 0.5)),
                new_note((bass_notes[0], None, None, 0.5)),
                new_note((bass_notes[2], None, None, 0.5)),
                new_note((bass_notes[3], None, None, 0.5)),
                new_note((bass_notes[6].transposed(-12), None, None, 0.5)),
            ]));

        let synth_pattern =
            beat_time
                .every_nth_bar(4.0)
                .emit(new_polyphonic_note_sequence_emitter(vec![
                    vec![
                        new_note(("C 4", None, None, 0.3)),
                        new_note(("D#4", None, None, 0.3)),
                        new_note(("G 4", None, None, 0.3)),
                    ],
                    vec![
                        new_note(("C 4", None, None, 0.3)),
                        new_note(("D#4", None, None, 0.3)),
                        new_note(("F 4", None, None, 0.3)),
                    ],
                    vec![
                        new_note(("C 4", None, None, 0.3)),
                        new_note(("D#4", None, None, 0.3)),
                        new_note(("G 4", None, None, 0.3)),
                    ],
                    vec![
                        new_note(("C 4", None, None, 0.3)),
                        new_note(("D#4", None, None, 0.3)),
                        new_note(("A#4", None, None, 0.3)),
                    ],
                ]));

        let fx_pattern =
            beat_time
                .every_nth_seconds(8.0)
                .emit(new_polyphonic_note_sequence_emitter(vec![
                    vec![new_note(("C 4", None, None, 0.2)), None, None],
                    vec![None, new_note(("C 4", None, None, 0.2)), None],
                    vec![None, None, new_note(("F 4", None, None, 0.2))],
                ]));

        let tone_pattern = beat_time
            .every_nth_eighth(1.0)
            .emit(new_cycle_emitter("[60 63 65 <58 ~>]/4")?);

        Ok(Phrase::new(
            beat_time,
            vec![
                PatternSlot::from(kick_pattern),
                PatternSlot::from(snare_pattern),
                PatternSlot::from(hihat_pattern),
                PatternSlot::from(bass_pattern),
                PatternSlot::from(fx_pattern),
                PatternSlot::from(synth_pattern),
                PatternSlot::from(tone_pattern),
            ],
            BeatTimeStep::Bar(8.0),
        ))
    }

    fn run_phrase(phrase: &mut Phrase, time: SampleTime) -> Vec<PatternEvent> {
        let mut events = Vec::new();
        while let Some(event) = phrase.run_until_time(time) {
            events.push(event)
        }
        events
    }

    // slow skip using run_until_time
    fn skip_phrase_by_running(phrase: &mut Phrase, time: SampleTime) {
        while phrase.run_until_time(time).is_some() {
            // ignore event
        }
    }

    // fast skip using skip_events_until_time
    fn skip_phrase_by_advancing(phrase: &mut Phrase, time: SampleTime) {
        phrase.advance_until_time(time)
    }

    #[test]
    fn skip_events() -> Result<(), String> {
        let sample_offset = 2345676;

        let mut phrase1 = create_phrase()?;
        phrase1.set_sample_offset(sample_offset);
        let mut events1 = Vec::new();

        let mut phrase2 = create_phrase()?;
        phrase2.set_sample_offset(sample_offset);
        let mut events2 = Vec::new();

        // run_time, advance_time
        let run_steps = [
            (1024, 1),
            (2000, 555432),
            (5012, 666),
            (200, 211),
            (100, 11200),
            (1024, 122),
            (8000, 5577432),
            (50700, 66),
            (21020, 2121),
            (1000, 100),
        ];

        let mut sample_time = sample_offset;
        for (run_time, seek_time) in run_steps {
            sample_time += run_time;
            events1.append(&mut run_phrase(&mut phrase1, sample_time));
            events2.append(&mut run_phrase(&mut phrase2, sample_time));

            sample_time += seek_time;
            skip_phrase_by_running(&mut phrase1, sample_time);
            skip_phrase_by_advancing(&mut phrase2, sample_time);
        }

        assert_eq!(events1, events2);

        Ok(())
    }
}
