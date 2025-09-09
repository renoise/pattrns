//! Events and event iterators which get emitted by a `Pattern`.

use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

use crate::Note;

// -------------------------------------------------------------------------------------------------

/// Id to refer to a specific instrument/patch/sample in a [`NoteEvent`].
#[derive(
    Copy,
    Clone,
    Debug,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    Eq,
    PartialEq,
    Hash,
)]
pub struct InstrumentId(usize);

/// Id to refer to a specific parameter in a [`ParameterChangeEvent`].
#[derive(
    Copy,
    Clone,
    Debug,
    derive_more::Display,
    derive_more::From,
    derive_more::Into,
    Eq,
    PartialEq,
    Hash,
)]
pub struct ParameterId(usize);

// -------------------------------------------------------------------------------------------------

/// Single note event in a [`Event`].
#[derive(Clone, PartialEq, Debug)]
pub struct NoteEvent {
    pub note: Note,
    pub instrument: Option<InstrumentId>,
    pub volume: f32,  // [0 - 1]
    pub panning: f32, // [-1 - 1]
    pub delay: f32,   // [0 - 1]
    pub glide: f32,   // [0 - INF]
}

impl NoteEvent {
    pub fn to_string(&self, show_instruments: bool) -> String {
        if show_instruments {
            format!(
                "{} {} {:.2} {:.2} {:.2}",
                self.note,
                if let Some(instrument) = self.instrument {
                    format!("#{:02}", instrument)
                } else {
                    "NA".to_string()
                },
                self.volume,
                self.panning,
                self.delay
            )
        } else {
            format!(
                "{} {:.2} {:.2} {:.2}",
                self.note, self.volume, self.panning, self.delay
            )
        }
    }
}

impl<N: TryInto<Note>> From<N> for NoteEvent
where
    <N as TryInto<Note>>::Error: std::fmt::Debug,
{
    // Initialize from a Note
    fn from(note: N) -> Self {
        let note = note.try_into().expect("Failed to convert note");
        Self {
            note,
            instrument: None,
            volume: 1.0,
            panning: 0.0,
            delay: 0.0,
            glide: 0.0,
        }
    }
}

impl<N: TryInto<Note>, I: Into<Option<InstrumentId>>> From<(N, I)> for NoteEvent
where
    <N as TryInto<Note>>::Error: std::fmt::Debug,
{
    // Initialize from a (Note, Instrument) tuple
    fn from((note, instrument): (N, I)) -> Self {
        let note = note.try_into().expect("Failed to convert note");
        let instrument = instrument.into();
        Self {
            note,
            instrument,
            volume: 1.0,
            panning: 0.0,
            delay: 0.0,
            glide: 0.0,
        }
    }
}

impl<N: TryInto<Note>, I: Into<Option<InstrumentId>>> From<(N, I, f32)> for NoteEvent
where
    <N as TryInto<Note>>::Error: std::fmt::Debug,
{
    // Initialize from a (Instrument, Note, Volume) tuple
    fn from((note, instrument, volume): (N, I, f32)) -> Self {
        let note = note.try_into().expect("Failed to convert note");
        let instrument = instrument.into();
        let volume = volume.clamp(0.0, 1.0);
        Self {
            note,
            instrument,
            volume,
            panning: 0.0,
            delay: 0.0,
            glide: 0.0,
        }
    }
}

impl<N: TryInto<Note>, I: Into<Option<InstrumentId>>> From<(N, I, f32, f32)> for NoteEvent
where
    <N as TryInto<Note>>::Error: std::fmt::Debug,
{
    // Initialize from a (Instrument, Note, Volume, Panning) tuple
    fn from((note, instrument, volume, panning): (N, I, f32, f32)) -> Self {
        let note = note.try_into().expect("Failed to convert note");
        let instrument = instrument.into();
        let volume = volume.clamp(0.0, 1.0);
        let panning = panning.clamp(-1.0, 1.0);
        Self {
            note,
            instrument,
            volume,
            panning,
            delay: 0.0,
            glide: 0.0,
        }
    }
}

impl<N: TryInto<Note>, I: Into<Option<InstrumentId>>> From<(N, I, f32, f32, f32)> for NoteEvent
where
    <N as TryInto<Note>>::Error: std::fmt::Debug,
{
    // Initialize from a (Instrument, Note, Volume, Panning, Delay) tuple
    fn from((note, instrument, volume, panning, delay): (N, I, f32, f32, f32)) -> Self {
        let note = note.try_into().expect("Failed to convert note");
        let instrument = instrument.into();
        let volume = volume.clamp(0.0, 1.0);
        let panning = panning.clamp(-1.0, 1.0);
        let delay = delay.clamp(0.0, 1.0);
        Self {
            note,
            instrument,
            volume,
            panning,
            delay,
            glide: 0.0,
        }
    }
}

impl<N: TryInto<Note>, I: Into<Option<InstrumentId>>> From<(N, I, f32, f32, f32, f32)> for NoteEvent
where
    <N as TryInto<Note>>::Error: std::fmt::Debug,
{
    // Initialize from a (Instrument, Note, Volume, Panning, Delay) tuple
    fn from((note, instrument, volume, panning, delay, glide): (N, I, f32, f32, f32, f32)) -> Self {
        let note = note.try_into().expect("Failed to convert note");
        let instrument = instrument.into();
        let volume = volume.clamp(0.0, 1.0);
        let panning = panning.clamp(-1.0, 1.0);
        let delay = delay.clamp(0.0, 1.0);
        let glide = glide.max(0.0);
        Self {
            note,
            instrument,
            volume,
            panning,
            delay,
            glide,
        }
    }
}

impl Display for NoteEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SHOW_INSTRUMENTS: bool = true;
        f.write_fmt(format_args!("{}", self.to_string(SHOW_INSTRUMENTS)))
    }
}

/// Shortcut for creating an empty [`NoteEvent`]
pub fn new_empty_note() -> Option<NoteEvent> {
    None
}

/// Shortcut for creating a new [`NoteEvent`].
pub fn new_note<E: Into<NoteEvent>>(note_event: E) -> Option<NoteEvent> {
    Some(note_event.into())
}

/// Shortcut for creating a vector of [`NoteEvent`]s:
/// e.g. a sequence of single notes
pub fn new_note_vector<E: Into<NoteEvent>>(sequence: Vec<Option<E>>) -> Vec<Option<NoteEvent>> {
    let mut event_sequence = Vec::with_capacity(sequence.len());
    for event in sequence {
        if let Some(event) = event {
            event_sequence.push(Some(event.into()));
        } else {
            event_sequence.push(None);
        }
    }
    event_sequence
}

/// Shortcut for creating a new sequence of polyphonic [`NoteEvent`]:
/// e.g. a sequence of chords
pub fn new_polyphonic_note_sequence<E: Into<NoteEvent>>(
    polyphonic_sequence: Vec<Vec<Option<E>>>,
) -> Vec<Vec<Option<NoteEvent>>> {
    let mut polyphonic_event_sequence = Vec::with_capacity(polyphonic_sequence.len());
    for sequence in polyphonic_sequence {
        let mut event_sequence = Vec::with_capacity(sequence.len());
        for event in sequence {
            if let Some(event) = event {
                event_sequence.push(Some(event.into()));
            } else {
                event_sequence.push(None);
            }
        }
        polyphonic_event_sequence.push(event_sequence);
    }
    polyphonic_event_sequence
}

// -------------------------------------------------------------------------------------------------

/// Single parameter change event in a [`Event`].
#[derive(Clone, PartialEq, Debug)]
pub struct ParameterChangeEvent {
    pub parameter: Option<ParameterId>,
    pub value: f32,
}

impl ParameterChangeEvent {
    pub fn to_string(&self, show_parameter: bool) -> String {
        if show_parameter {
            format!(
                "{} {:.3}",
                if let Some(parameter) = self.parameter {
                    format!("{:02}", parameter)
                } else {
                    "NA".to_string()
                },
                self.value,
            )
        } else {
            format!("{:.3}", self.value)
        }
    }
}

impl Display for ParameterChangeEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SHOW_PARAMETERS: bool = true;
        f.write_fmt(format_args!("{}", self.to_string(SHOW_PARAMETERS)))
    }
}

/// Shortcut for creating a new [`ParameterChangeEvent`].
pub fn new_parameter_change<Parameter: Into<Option<ParameterId>>>(
    parameter: Parameter,
    value: f32,
) -> ParameterChangeEvent {
    let parameter: Option<ParameterId> = parameter.into();
    ParameterChangeEvent { parameter, value }
}

// -------------------------------------------------------------------------------------------------

/// Event value, produced by [`Emitter`](crate::Emitter) as [`EmitterEvent`](crate::EmitterEvent).
#[derive(Clone, PartialEq, Debug)]
pub enum Event {
    NoteEvents(Vec<Option<NoteEvent>>),
    ParameterChangeEvent(ParameterChangeEvent),
}

impl Event {
    pub fn to_string(&self, show_instruments_and_parameters: bool) -> String {
        match self {
            Event::NoteEvents(note_vector) => note_vector
                .iter()
                .map(|n| {
                    if let Some(v) = n {
                        v.to_string(show_instruments_and_parameters)
                    } else {
                        "---".to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join(" | "),
            Event::ParameterChangeEvent(change) => {
                change.to_string(show_instruments_and_parameters)
            }
        }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        const SHOW_INSTRUMENTS_AND_PARAMETERS: bool = true;
        f.write_fmt(format_args!(
            "{}",
            self.to_string(SHOW_INSTRUMENTS_AND_PARAMETERS)
        ))
    }
}

// -------------------------------------------------------------------------------------------------

/// A refcounted function which transforms emitted [`Event`] contents.
pub type EventTransform = Rc<dyn Fn(&mut Event)>;
