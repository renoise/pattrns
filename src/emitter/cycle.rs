use std::{collections::HashMap, ops::RangeBounds};

type Fraction = num_rational::Rational32;

use crate::{
    event::new_note, BeatTimeBase, Chord, Cycle, CycleEvent, CycleTarget, CycleValue, Emitter,
    EmitterEvent, Event, InstrumentId, Note, NoteEvent, ParameterSet, RhythmEvent,
};

// -------------------------------------------------------------------------------------------------

/// Default conversion of a CycleValue into a note stack.
///
/// Returns an error when resolving chord modes failed.
impl TryFrom<&CycleValue> for Vec<Option<NoteEvent>> {
    type Error = String;

    fn try_from(value: &CycleValue) -> Result<Self, String> {
        match value {
            CycleValue::Hold => Ok(vec![None]),
            CycleValue::Rest => Ok(vec![new_note(Note::OFF)]),
            CycleValue::Float(_f) => Ok(vec![None]),
            CycleValue::Integer(i) => Ok(vec![new_note(Note::from((*i).clamp(0, 0x7f) as u8))]),
            CycleValue::Pitch(p) => Ok(vec![new_note(Note::from(p.midi_note()))]),
            CycleValue::Chord(p, m) => {
                let chord = Chord::try_from((p.midi_note(), m.as_ref()))?;
                Ok(chord
                    .intervals()
                    .iter()
                    .map(|i| new_note(chord.note().transposed(*i as i32)))
                    .collect())
            }
            CycleValue::Target(_) => Ok(vec![None]),
            CycleValue::Name(s) => {
                if s.eq_ignore_ascii_case("off") {
                    Ok(vec![new_note(Note::OFF)])
                } else {
                    Ok(vec![None])
                }
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

// Conversion helpers for cycle targets
fn float_value_in_range<Range>(
    maybe_float: &Option<f64>,
    name: &'static str,
    range: Range,
) -> Result<f32, String>
where
    Range: RangeBounds<f32> + std::fmt::Debug,
{
    maybe_float
        .map(|v| v as f32)
        .ok_or_else(|| format!("{} property must be a number value", name))
        .and_then(|v| {
            if range.contains(&v) {
                Ok(v)
            } else {
                Err(format!(
                    "{} property must be in range [{:?}] but is '{}'",
                    name, range, v
                ))
            }
        })
}

fn integer_value_in_range<Range>(
    value: i32,
    name: &'static str,
    range: Range,
) -> Result<i32, String>
where
    Range: RangeBounds<i32> + std::fmt::Debug,
{
    if range.contains(&value) {
        Ok(value)
    } else {
        Err(format!(
            "{} property must be in range [{:?}] but is '{}'",
            name, range, value
        ))
    }
}

/// Apply cycle targets as note properties to the given note events
pub(crate) fn apply_cycle_note_properties(
    note_events: &mut [Option<NoteEvent>],
    targets: &[CycleTarget],
) -> Result<(), String> {
    // quickly return if there are no targets or notes to process
    if targets.is_empty() || note_events.is_empty() {
        return Ok(());
    }
    // apply for all non empty note events
    for target in targets {
        match target {
            CycleTarget::Index(index) => {
                let index = integer_value_in_range(*index, "instrument", 0..)?;
                let instrument = InstrumentId::from(index as usize);
                for note_event in note_events.iter_mut().flatten() {
                    note_event.instrument = Some(instrument);
                }
            }
            CycleTarget::Named(name, value) => match name.as_bytes() {
                b"v" => {
                    let volume = float_value_in_range(value, "volume", 0.0..=1.0)?;
                    for note_event in note_events.iter_mut().flatten() {
                        note_event.volume = volume;
                    }
                }
                b"p" => {
                    let panning = float_value_in_range(value, "panning", -1.0..=1.0)?;
                    for note_event in note_events.iter_mut().flatten() {
                        note_event.panning = panning;
                    }
                }
                b"d" => {
                    let delay = float_value_in_range(value, "delay", 0.0..1.0)?;
                    for note_event in note_events.iter_mut().flatten() {
                        note_event.delay = delay;
                    }
                }
                b"g" => {
                    let glide = float_value_in_range(value, "glide", 0.0..)?;
                    for note_event in note_events.iter_mut().flatten() {
                        note_event.glide = Some(glide);
                    }
                }
                _ => {
                    return Err(
                        format!("invalid note property: '{name}'. ")
                            + "expecting only number values with  "
                            + "'#' (instrument), 'v' (volume), 'p' (panning), 'd' (delay) or 'g' (glide) "
                            + "prefixes here.");
                }
            },
        }
    }
    Ok(())
}

// -------------------------------------------------------------------------------------------------

/// Helper struct to convert time tagged events from Cycle into a `Vec<EmitterEvent>`
pub(crate) struct CycleNoteEvents {
    // collected events for a given time span per channels
    events: Vec<(Fraction, Fraction, Vec<Option<Event>>)>,
    // max note event count per channel
    event_counts: Vec<usize>,
}

impl CycleNoteEvents {
    /// Create a new, empty list of events.
    pub fn new() -> Self {
        let events = Vec::with_capacity(16);
        let event_counts = Vec::with_capacity(3);
        Self {
            events,
            event_counts,
        }
    }

    /// Add a single note event stack from a cycle channel event.
    pub fn add(
        &mut self,
        channel: usize,
        start: Fraction,
        length: Fraction,
        note_events: Vec<Option<NoteEvent>>,
    ) {
        // memorize max event count per channel
        if self.event_counts.len() <= channel {
            self.event_counts.resize(channel + 1, 0);
        }
        self.event_counts[channel] = self.event_counts[channel].max(note_events.len());
        // insert events into existing time slot or a new one
        match self
            .events
            .binary_search_by(|(time, _, _)| time.cmp(&start))
        {
            Ok(pos) => {
                // use min length of all notes in stack
                let event_length = &mut self.events[pos].1;
                *event_length = (*event_length).min(length);
                // add new notes to existing events
                let timed_event = &mut self.events[pos].2;
                timed_event.resize(channel + 1, None);
                timed_event[channel] = Some(Event::NoteEvents(note_events));
            }
            Err(pos) => {
                // insert a new time event
                let mut timed_event = Vec::with_capacity(channel + 1);
                timed_event.resize(channel + 1, None);
                timed_event[channel] = Some(Event::NoteEvents(note_events));
                self.events.insert(pos, (start, length, timed_event))
            }
        }
    }

    /// Convert to a list of EmitterEvents.
    pub fn into_event_iter_items(self) -> Vec<EmitterEvent> {
        // max number of note events in a single merged down Event
        let max_event_count = self.event_counts.iter().sum::<usize>();
        // apply padding per channel, merge down and convert to EmitterEvent
        let mut event_iter_items: Vec<EmitterEvent> = Vec::with_capacity(self.events.len());
        for (start_time, length, mut events) in self.events.into_iter() {
            // ensure that each event in the channel, contains the same number of note events
            for (channel, mut event) in events.iter_mut().enumerate() {
                if let Some(Event::NoteEvents(note_events)) = &mut event {
                    // pad existing note events with OFFs
                    note_events.resize_with(self.event_counts[channel], || new_note(Note::OFF));
                } else if self.event_counts[channel] > 0 {
                    // pad missing note events with 'None'
                    *event = Some(Event::NoteEvents(vec![None; self.event_counts[channel]]))
                }
            }
            // merge all events that happen at the same time together
            let mut merged_note_events = Vec::with_capacity(max_event_count);
            for mut event in events.into_iter().flatten() {
                if let Event::NoteEvents(note_events) = &mut event {
                    merged_note_events.append(note_events);
                }
            }
            // convert padded, merged note events to a timed 'Event'
            let event = Event::NoteEvents(merged_note_events);
            event_iter_items.push(EmitterEvent::new_with_fraction(event, start_time, length));
        }
        event_iter_items
    }
}

// -------------------------------------------------------------------------------------------------

/// Emits events from a [`Cycle`].
///
/// Channels from cycle are merged down into note events on different voices.
/// Values in cycles can be mapped to notes with an optional mapping table.
///
/// See also [`ScriptedCycleEmitter`](`super::scripted_cycle::ScriptedCycleEmitter`)
#[derive(Clone, Debug)]
pub struct CycleEmitter {
    cycle: Cycle,
    mappings: HashMap<String, Vec<Option<NoteEvent>>>,
}

impl CycleEmitter {
    /// Create a new cycle emitter from the given precompiled cycle.
    pub(crate) fn new(cycle: Cycle) -> Self {
        let mappings = HashMap::new();
        Self { cycle, mappings }
    }

    /// Try creating a new cycle emitter from the given mini notation string.
    ///
    /// Returns error when the cycle string failed to parse.
    pub fn from_mini(input: &str) -> Result<Self, String> {
        Ok(Self::new(Cycle::from(input)?))
    }

    /// Try creating a new cycle emitter from the given mini notation string
    /// and the given seed for the cycle's random number generator.
    ///
    /// Returns error when the cycle string failed to parse.
    pub fn from_mini_with_seed(input: &str, seed: u64) -> Result<Self, String> {
        Ok(Self::new(Cycle::from(input)?.with_seed(seed)))
    }

    /// Return a new cycle with the given value mappings applied.
    pub fn with_mappings<S: Into<String> + Clone>(
        self,
        map: &[(S, Vec<Option<NoteEvent>>)],
    ) -> Self {
        let mut mappings = HashMap::new();
        for (k, v) in map.iter().cloned() {
            mappings.insert(k.into(), v);
        }
        Self { mappings, ..self }
    }

    /// Generate a note event from a single cycle event, applying mappings if necessary
    fn map_note_event(&mut self, event: CycleEvent) -> Result<Vec<Option<NoteEvent>>, String> {
        let mut note_events = {
            if let Some(note_events) = self.mappings.get(event.string()) {
                // apply custom note mappings
                note_events.clone()
            } else {
                // try converting the cycle value to a single note
                event.value().try_into()?
            }
        };
        // apply note properties from targets
        apply_cycle_note_properties(&mut note_events, event.targets())?;
        Ok(note_events)
    }

    /// Generate next batch of events from the next cycle run.
    /// Converts cycle events to note events and flattens channels into note columns.
    fn generate(&mut self) -> Vec<EmitterEvent> {
        // run the cycle event generator
        let events = {
            match self.cycle.generate() {
                Ok(events) => events,
                Err(err) => {
                    // NB: only expected error here is exceeding the event limit
                    panic!("Cycle runtime error: {err}");
                }
            }
        };
        let mut timed_note_events = CycleNoteEvents::new();
        // convert possibly mapped cycle channel items to a list of note events
        for (channel_index, channel_events) in events.into_iter().enumerate() {
            for event in channel_events.into_iter() {
                let start = event.span().start();
                let length = event.span().length();
                match self.map_note_event(event) {
                    Ok(note_events) => {
                        if !note_events.is_empty() {
                            timed_note_events.add(channel_index, start, length, note_events);
                        }
                    }
                    Err(err) => {
                        //  NB: only expected error here is a chord parser error
                        panic!("Cycle runtime error: {err}");
                    }
                }
            }
        }
        // convert timed note events into EmitterEvents
        timed_note_events.into_event_iter_items()
    }
}

impl Emitter for CycleEmitter {
    fn set_time_base(&mut self, _time_base: &BeatTimeBase) {
        // nothing to do
    }

    fn set_trigger_event(&mut self, _event: &Event) {
        // nothing to do
    }

    fn set_parameters(&mut self, _parameters: ParameterSet) {
        // nothing to do
    }

    fn run(&mut self, _pulse: RhythmEvent, emit_event: bool) -> Option<Vec<EmitterEvent>> {
        if emit_event {
            Some(self.generate())
        } else {
            None
        }
    }

    fn advance(&mut self, _pulse: RhythmEvent, emit_event: bool) {
        if emit_event {
            self.cycle.advance();
        }
    }

    fn duplicate(&self) -> Box<dyn Emitter> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        self.cycle.reset();
    }
}

// -------------------------------------------------------------------------------------------------

pub fn new_cycle_emitter(input: &str) -> Result<CycleEmitter, String> {
    CycleEmitter::from_mini(input)
}

pub fn new_cycle_emitter_with_seed(input: &str, seed: u64) -> Result<CycleEmitter, String> {
    CycleEmitter::from_mini_with_seed(input, seed)
}
