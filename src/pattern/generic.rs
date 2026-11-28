//! Generic `Pattern` implementation with custom time step and offset types.

use std::{borrow::Borrow, cell::RefCell, collections::VecDeque, fmt::Debug, rc::Rc};

type Fraction = num_rational::Rational32;
use num_traits::ToPrimitive;

#[cfg(all(feature = "scripting", test))]
use std::borrow::BorrowMut;

use crate::{
    emitter::{fixed::FixedEmitter, Emitter, EmitterEvent},
    event::{Event, InstrumentId},
    gate::threshold::ThresholdGate,
    rhythm::{fixed::FixedRhythm, Rhythm},
    time::BeatTimeBase,
    EventTransform, ExactSampleTime, Gate, Parameter, ParameterSet, Pattern, PatternEvent,
    RhythmEvent, SampleTime,
};

// -------------------------------------------------------------------------------------------------

/// Time value of a `GenericPattern`, used either as `Step` or `Offset` value.
pub trait GenericPatternTimeStep: Debug + Clone + Copy + 'static {
    /// The default offset value of the `PatternTimeStep`. Usually some `0` value.
    fn default_offset() -> Self;
    /// The step value of the `PatternTimeStep`. Usually some non `0` value.
    fn default_step() -> Self;

    /// Converts the `PatternTimeStep` to an exact sample time.
    fn to_samples(&self, time_base: &BeatTimeBase) -> ExactSampleTime;
}

// -------------------------------------------------------------------------------------------------

/// Generic [`Pattern`] impl which uses a [`Pattern`] to generate pulse events, filtered by
/// a [`Gate`] which then drives an [`Emitter`][crate::Emitter] to create [`Event`]s.
///
/// Internal time units are generic, and will usually be beats or seconds.
pub struct GenericPattern<Step: GenericPatternTimeStep, Offset: GenericPatternTimeStep> {
    time_base: BeatTimeBase,
    step: Step,
    offset: Offset,
    instrument: Option<InstrumentId>,
    parameters: ParameterSet,
    rhythm: Box<dyn Rhythm>,
    rhythm_event: RhythmEvent,
    rhythm_repeat_count: Option<usize>,
    rhythm_playback_finished: bool,
    gate: Box<dyn Gate>,
    emitter: Box<dyn Emitter>,
    emitter_sample_time: SampleTime,
    emitter_next_sample_time: ExactSampleTime,
    events: VecDeque<EmitterEvent>,
    event_transform: Option<EventTransform>,
    sample_offset: SampleTime,
}

impl<Step: GenericPatternTimeStep, Offset: GenericPatternTimeStep> Debug
    for GenericPattern<Step, Offset>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenericPattern")
            .field("time_base", &self.time_base)
            .field("step", &self.step)
            .field("offset", &self.offset)
            .field("instrument", &self.instrument)
            .field("parameters", &self.parameters)
            .field("rhythm", &self.rhythm)
            .field("rhythm_repeat_count", &self.rhythm_repeat_count)
            .field("rhythm_playback_finished", &self.rhythm_playback_finished)
            .field("gate", &self.gate)
            .field("emitter", &self.emitter)
            // Skip event_transform, which has no Debug impl and event_iter state to reduce noise
            .field("sample_offset", &self.sample_offset)
            .finish()
    }
}

impl<Step: GenericPatternTimeStep, Offset: GenericPatternTimeStep> GenericPattern<Step, Offset> {
    /// Create a new pattern which emits events every `beat_time_base` `step`.
    pub fn new(time_base: BeatTimeBase, step: Step) -> Self {
        let offset = Offset::default_offset();
        let instrument = None;
        let parameters = ParameterSet::new();
        let rhythm = Box::<FixedRhythm>::default();
        let rhythm_event = RhythmEvent::default();
        let rhythm_repeat_count = None;
        let rhythm_playback_finished = false;
        let gate = Box::new(ThresholdGate::new());
        let emitter = Box::<FixedEmitter>::default();
        let emitter_sample_time = 0;
        let emitter_next_sample_time = offset.to_samples(&time_base);
        let events = VecDeque::new();
        let event_transform = None;
        let sample_offset = 0;
        Self {
            time_base,
            step,
            offset,
            instrument,
            parameters,
            rhythm,
            rhythm_event,
            rhythm_repeat_count,
            rhythm_playback_finished,
            gate,
            emitter,
            emitter_sample_time,
            emitter_next_sample_time,
            events,
            event_transform,
            sample_offset,
        }
    }

    /// Get current time base.
    pub fn time_base(&self) -> BeatTimeBase {
        self.time_base
    }
    /// Get current step.
    pub fn step(&self) -> Step {
        self.step
    }
    /// Get current offset.
    pub fn offset(&self) -> Offset {
        self.offset
    }
    /// Get current rhythm.
    pub fn rhythm(&self) -> &dyn Rhythm {
        self.rhythm.borrow()
    }
    /// Mut access the current rhythm (only allowed in tests).
    #[cfg(test)]
    pub(crate) fn rhythm_mut(&mut self) -> &mut dyn Rhythm {
        self.rhythm.borrow_mut()
    }

    /// Return a new pattern instance which applies the given step offset to all events.
    #[must_use]
    pub fn with_offset<O: Into<Option<Offset>>>(self, offset: O) -> Self {
        let offset = offset.into().unwrap_or(Offset::default_offset());
        let event_iter_sample_time = 0;
        let event_iter_next_sample_time = offset.to_samples(&self.time_base);
        Self {
            offset,
            emitter_sample_time: event_iter_sample_time,
            emitter_next_sample_time: event_iter_next_sample_time,
            ..self
        }
    }

    /// Return a new pattern instance which uses the given instrument for all note events
    /// which have no instrument set.
    #[must_use]
    pub fn with_instrument<I: Into<Option<InstrumentId>>>(self, instrument: I) -> Self {
        let instrument = instrument.into();
        Self { instrument, ..self }
    }

    /// Return a new pattern instance with the given input parameter map.  
    #[must_use]
    pub fn with_parameters(self, parameters: ParameterSet) -> Self {
        let mut new = self;
        new.parameters.clone_from(&parameters);
        new.rhythm.set_parameters(parameters.clone());
        new.gate.set_parameters(parameters.clone());
        new.emitter.set_parameters(parameters);
        new
    }

    /// Return a new pattern instance which trigger events with the given [`Rhythm`].  
    #[must_use]
    pub fn with_rhythm<T: Rhythm + Sized + 'static>(self, rhythm: T) -> Self {
        self.with_rhythm_dyn(Box::new(rhythm))
    }

    /// Return a new pattern instance which triggers events with the given dyn [`Rhythm`].  
    #[must_use]
    pub fn with_rhythm_dyn(self, rhythm: Box<dyn Rhythm>) -> Self {
        let time_base = self.time_base;
        let parameters = self.parameters.clone();
        let repeat_count = self.rhythm_repeat_count;
        let mut new = self;
        new.rhythm = rhythm;
        new.rhythm.set_time_base(&time_base);
        new.rhythm.set_parameters(parameters);
        new.rhythm.set_repeat_count(repeat_count);
        new
    }

    /// Return a new pattern instance which repeats the pattern up to `count` times.
    /// When None, it repeats forever.
    #[must_use]
    pub fn with_repeat(self, count: Option<usize>) -> Self {
        let mut new = self;
        new.rhythm_repeat_count = count;
        new.rhythm.set_repeat_count(count);
        new
    }

    /// Return a new pattern instance which uses the given [`Gate`] instead of the default gate.  
    #[must_use]
    pub fn with_gate<T: Gate + Sized + 'static>(self, gate: T) -> Self {
        self.with_gate_dyn(Box::new(gate))
    }

    /// Return a new pattern instance which uses the given dyn [`Gate`] instead of the default gate.  
    #[must_use]
    pub fn with_gate_dyn(self, gate: Box<dyn Gate>) -> Self {
        let time_base = self.time_base;
        let parameters = self.parameters.clone();
        let mut new = self;
        new.gate = gate;
        new.gate.set_time_base(&time_base);
        new.gate.set_parameters(parameters);
        new
    }

    /// Return a new pattern instance which uses the given [`Emitter`] to trigger events.
    #[must_use]
    pub fn emit<E: Emitter + 'static>(self, iter: E) -> Self {
        self.trigger_dyn(Box::new(iter))
    }

    /// Return a new pattern instance which uses the given dyn [`Emitter`] to trigger events.
    #[must_use]
    pub fn trigger_dyn(self, emitter: Box<dyn Emitter>) -> Self {
        let time_base = self.time_base;
        let parameters = self.parameters.clone();
        let mut new = self;
        new.emitter = emitter;
        new.emitter.set_time_base(&time_base);
        new.emitter.set_parameters(parameters);
        new
    }

    /// Return a new pattern instance which uses the given event transform function
    #[must_use]
    pub fn with_event_transform(self, transform: EventTransform) -> Self {
        Self {
            event_transform: Some(transform),
            ..self
        }
    }

    /// Return current pulse duration in samples.
    #[inline]
    pub fn current_steps_sample_duration(&self) -> ExactSampleTime {
        self.step.to_samples(&self.time_base) * self.rhythm_event.step_time
    }

    /// Return sample time of the given fraction within the current emitter event.
    #[inline]
    fn event_iter_item_time(&self, fraction: f64) -> SampleTime {
        let step_time = self.current_steps_sample_duration();
        let event_iter_time = self.sample_offset as f64 + self.emitter_next_sample_time;
        // NB: round up instead of truncating to keep sample, beat timing in sync with Renoise
        (event_iter_time + (step_time * fraction)).ceil() as SampleTime
    }

    /// Return start sample time of the given emitter event start time.
    #[inline]
    fn event_iter_item_start_time(&self, start: &Fraction) -> SampleTime {
        self.event_iter_item_time(start.to_f64().unwrap_or(0.0))
    }

    /// Return duration in sample time of the given emitter event start and length.
    #[inline]
    fn event_iter_item_duration(&self, start: &Fraction, length: &Fraction) -> SampleTime {
        let start = start.to_f64().unwrap_or(0.0);
        let length = length.to_f64().unwrap_or(1.0);
        // Note: Derive duration from quantized start and end times, so that
        // event durations always line up with the quantized event times.
        self.event_iter_item_time(start + length)
            .saturating_sub(self.event_iter_item_time(start))
    }

    /// Set a default instrument, if set, and apply event transform functions.
    fn apply_event_transform(&self, event_item: &mut EmitterEvent) {
        if let Some(instrument) = self.instrument {
            if let Event::NoteEvents(note_events) = &mut event_item.event {
                for note_event in note_events.iter_mut().flatten() {
                    note_event.instrument = note_event.instrument.or(Some(instrument));
                }
            }
            if let Some(transform) = &self.event_transform {
                transform(&mut event_item.event);
            }
        }
    }

    fn run_rhythm(&mut self) -> Option<(RhythmEvent, bool)> {
        debug_assert!(
            self.events.is_empty(),
            "Should only run rhythms when there are no pending emitter items"
        );
        if let Some(event) = self.rhythm.run() {
            let emit_event = self.gate.run(&event);
            self.rhythm_event = event;
            Some((event, emit_event))
        } else {
            None
        }
    }

    fn run(&mut self, sample_time: SampleTime, fetch_new_events: bool) -> Option<PatternEvent> {
        // quickly check if pattern playback finished
        if self.rhythm_playback_finished {
            return None;
        }
        // quickly check if the next event is due before the given target time
        let next_sample_time = self.sample_offset + self.emitter_next_sample_time as SampleTime;
        if next_sample_time >= sample_time {
            // next event is not yet due
            return None;
        }
        // fetch new events, if necessary
        if self.events.is_empty() {
            if !fetch_new_events {
                // if we should not fetch new events we're done here
                return None;
            }
            // generate a pulse from the pattern and pass the pulse to the gate
            if let Some((pulse, emit_event)) = self.run_rhythm() {
                // generate new events from the gated pulse
                self.events = self
                    .emitter
                    .run(pulse, emit_event)
                    .map_or_else(VecDeque::default, VecDeque::from);
            } else {
                // pattern playback finished
                self.rhythm_playback_finished = true;
                return None;
            }
        }
        // fetch a new event item from the events deque
        if let Some(event_item) = self.events.pop_front().map(|mut event| {
            self.apply_event_transform(&mut event);
            event
        }) {
            // return event as sample timed rhythm iter item
            let time = self.event_iter_item_start_time(&event_item.start);
            if time >= sample_time {
                // the given event is not yet due: put it back
                self.events.push_front(event_item);
                return None;
            }
            let event = Some(event_item.event);
            let duration = self.event_iter_item_duration(&event_item.start, &event_item.length);
            // advance to the next pulse in the next iteration when all events got consumed
            if self.events.is_empty() {
                self.emitter_next_sample_time += self.current_steps_sample_duration();
            }
            // return event as rhythm iter item
            Some(PatternEvent {
                time,
                event,
                duration,
            })
        } else {
            // return 'None' event as sample timed rhythm iter item
            let time = self.event_iter_item_start_time(&Fraction::ZERO);
            debug_assert!(time < sample_time, "Event should be due here");
            let event = None;
            let duration = self.event_iter_item_duration(&Fraction::ZERO, &Fraction::ONE);
            // advance to the next pulse in the next iteration
            self.emitter_next_sample_time += self.current_steps_sample_duration();
            // return event as rhythm iter item
            Some(PatternEvent {
                time,
                event,
                duration,
            })
        }
    }
}

impl<Step: GenericPatternTimeStep, Offset: GenericPatternTimeStep> Clone
    for GenericPattern<Step, Offset>
{
    fn clone(&self) -> Self {
        Self {
            parameters: self.parameters.clone(),
            rhythm: self.rhythm.duplicate(),
            emitter: self.emitter.duplicate(),
            events: self.events.clone(),
            event_transform: self.event_transform.clone(),
            gate: self.gate.duplicate(),
            ..*self
        }
    }
}

impl<Step: GenericPatternTimeStep, Offset: GenericPatternTimeStep> Iterator
    for GenericPattern<Step, Offset>
{
    type Item = PatternEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.run_until_time(SampleTime::MAX)
    }
}

impl<Step: GenericPatternTimeStep, Offset: GenericPatternTimeStep> Pattern
    for GenericPattern<Step, Offset>
{
    fn time_base(&self) -> &BeatTimeBase {
        &self.time_base
    }
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        // reschedule next event's sample time to the new time base
        if self.emitter_sample_time > 0 {
            self.emitter_next_sample_time = self.emitter_sample_time as ExactSampleTime
                + (self.emitter_next_sample_time - self.emitter_sample_time as ExactSampleTime)
                    / self.step.to_samples(&self.time_base)
                    * self.step.to_samples(time_base);
        }
        self.time_base.clone_from(time_base);
        // update pattern, gate and emitter
        self.rhythm.set_time_base(time_base);
        self.gate.set_time_base(time_base);
        self.emitter.set_time_base(time_base);
    }

    fn step_length(&self) -> ExactSampleTime {
        self.step.to_samples(&self.time_base)
    }
    fn step_count(&self) -> usize {
        self.rhythm.len()
    }

    fn parameters(&self) -> &[Rc<RefCell<Parameter>>] {
        &self.parameters
    }

    fn set_trigger_event(&mut self, event: &Event) {
        self.rhythm.set_trigger_event(event);
        self.gate.set_trigger_event(event);
        self.emitter.set_trigger_event(event);
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
        // memorize current time
        self.emitter_sample_time = sample_time;
        // fetch events
        let fetch_new_items = true;
        self.run(sample_time, fetch_new_items)
    }

    fn advance_until_time(&mut self, sample_time: SampleTime) {
        // memorize current time
        self.emitter_sample_time = sample_time;
        // clear pending events with regular runs
        while !self.events.is_empty() {
            let fetch_new_items = false;
            if self.run(sample_time, fetch_new_items).is_none() {
                break;
            }
        }
        // when the are still pending events, they are not yet due, so we are done
        if !self.events.is_empty() {
            return;
        }
        // quickly check if pattern playback finished
        if self.rhythm_playback_finished {
            return;
        }
        // batch advance events in full pulse steps
        loop {
            // quickly check if the next event is due before the given target time
            let next_sample_time =
                self.sample_offset as ExactSampleTime + self.emitter_next_sample_time;
            if (next_sample_time as SampleTime) >= sample_time {
                // next event is not yet due: we're done
                return;
            }
            // generate a pulse from the pattern and pass the pulse to the gate
            if let Some((pulse, emit_event)) = self.run_rhythm() {
                // test if the event crosses the target time
                let step_duration = self.current_steps_sample_duration();
                if ((next_sample_time + step_duration) as SampleTime) < sample_time {
                    // skip all events from the gated pulse
                    self.emitter.advance(pulse, emit_event);
                    self.emitter_next_sample_time += step_duration;
                } else {
                    // generate new events from the gated pulse
                    self.events = self
                        .emitter
                        .run(pulse, emit_event)
                        .map_or_else(VecDeque::default, VecDeque::from);
                    // when the remaining step is empty advance to next step
                    if self.events.is_empty() {
                        self.emitter_next_sample_time += self.current_steps_sample_duration();
                    }
                    // we're done either way now...
                    break;
                }
            } else {
                // pattern playback finished: we're done here
                self.rhythm_playback_finished = true;
                return;
            }
        }
        // clear remaining events with regular runs
        while !self.events.is_empty() {
            let fetch_new_items = true;
            if self.run(sample_time, fetch_new_items).is_none() {
                break;
            }
        }
    }

    fn duplicate(&self) -> Rc<RefCell<dyn Pattern>> {
        Rc::new(RefCell::new(self.clone()))
    }

    fn reset(&mut self) {
        // reset sample offset
        self.sample_offset = 0;
        // reset pattern and gate
        self.rhythm.reset();
        self.rhythm_playback_finished = false;
        self.gate.reset();
        // reset iterator state
        self.emitter.reset();
        self.emitter_sample_time = 0;
        self.emitter_next_sample_time = self.offset.to_samples(&self.time_base);
        self.rhythm_event = RhythmEvent::default();
        self.events.clear();
    }
}
