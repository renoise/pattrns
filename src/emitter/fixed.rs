use crate::{
    event::{
        new_empty_note, new_note, new_note_vector, new_parameter_change,
        new_polyphonic_note_sequence,
    },
    BeatTimeBase, Emitter, EmitterEvent, Event, Note, NoteEvent, ParameterChangeEvent, ParameterId,
    ParameterSet, RhythmEvent,
};

// -------------------------------------------------------------------------------------------------

/// Continuously emits a single or vector of static event values.
#[derive(Clone, Debug)]
pub struct FixedEmitter {
    events: Vec<Event>,
    event_index: usize,
}

impl FixedEmitter {
    pub fn new(events: Vec<Event>) -> Self {
        let mut events = events;
        Self::normalize_events(&mut events);
        let event_index = 0;
        Self {
            events,
            event_index,
        }
    }

    /// Access to the event that we're triggering.
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// Add note-offs for all notes in the given event list.
    pub(crate) fn normalize_events(events: &mut Vec<Event>) {
        let mut note_event_state = Vec::<Option<NoteEvent>>::new();
        for event in &mut *events {
            Self::normalize_event(event, &mut note_event_state);
        }
        if events.len() > 1 {
            Self::normalize_event(events.first_mut().unwrap(), &mut note_event_state)
        }
    }

    /// Add note-offs for all notes in the given event to stop pending notes from last runs.
    /// This will only pad, resize the given event's note list with note-offs where necessary.
    pub(crate) fn normalize_event(
        event: &mut Event,
        note_event_state: &mut Vec<Option<NoteEvent>>,
    ) {
        if let Event::NoteEvents(note_events) = event {
            if !note_events.iter().all(|n| n.is_none()) {
                // auto-close previous note's note-ons, unless there is some explicit content
                while note_events.len() < note_event_state.len() {
                    if note_event_state[note_events.len()]
                        .as_ref()
                        .is_some_and(|n| n.note.is_note_on())
                    {
                        note_events.push(new_note(Note::OFF));
                    } else {
                        note_events.push(None);
                    }
                }
                // update note state
                if note_event_state.len() < note_events.len() {
                    note_event_state.resize(note_events.len(), None);
                }
                for (note_event_state, note_event) in
                    note_event_state.iter_mut().zip(note_events.iter())
                {
                    if let Some(note_event) = note_event {
                        if note_event.note.is_note_on() {
                            *note_event_state = Some(note_event.clone());
                        } else {
                            *note_event_state = None;
                        }
                    }
                }
            }
            // remove trailing none's from note state
            while note_event_state.last().is_some_and(|n| n.is_none()) {
                note_event_state.pop();
            }
        }
    }
}

impl Default for FixedEmitter {
    fn default() -> Self {
        Self::new(vec![Event::NoteEvents(vec![Some((Note::C4).into())])])
    }
}
impl Emitter for FixedEmitter {
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
        if !emit_event || self.events.is_empty() {
            return None;
        }
        let event = self.events[self.event_index].clone();
        self.event_index = (self.event_index + 1) % self.events.len();
        Some(vec![EmitterEvent::new(event)])
    }

    fn advance(&mut self, _pulse: RhythmEvent, emit_event: bool) {
        if !emit_event || self.events.is_empty() {
            return;
        }
        self.event_index = (self.event_index + 1) % self.events.len();
    }

    fn duplicate(&self) -> Box<dyn Emitter> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        // reset step counter
        self.event_index = 0;
    }
}

// -------------------------------------------------------------------------------------------------

/// Convert [`Event`]s into [`FixedEmitter`]s.
pub trait ToFixedEmitter {
    fn to_emitter(self) -> FixedEmitter;
}

impl ToFixedEmitter for NoteEvent {
    /// Wrap a [`NoteEvent`] to a new [`FixedEmitter`]
    /// resulting into a single monophonic event.
    fn to_emitter(self) -> FixedEmitter {
        FixedEmitter::new(vec![Event::NoteEvents(vec![Some(self)])])
    }
}
impl ToFixedEmitter for Option<NoteEvent> {
    /// Wrap a [`NoteEvent`] to a new [`FixedEmitter`]
    /// resulting into a single monophonic event.
    fn to_emitter(self) -> FixedEmitter {
        FixedEmitter::new(vec![Event::NoteEvents(vec![self])])
    }
}

impl ToFixedEmitter for Vec<NoteEvent> {
    /// Wrap a vector of [`NoteEvent`]s to a new [`FixedEmitter`].
    /// resulting into a single polyphonic event.
    fn to_emitter(self) -> FixedEmitter {
        FixedEmitter::new(vec![Event::NoteEvents(
            self.iter().map(|v| Some(v.clone())).collect::<Vec<_>>(),
        )])
    }
}
impl ToFixedEmitter for Vec<Option<NoteEvent>> {
    /// Wrap a vector of [`NoteEvent`]s to a new [`FixedEmitter`].
    /// resulting into a single polyphonic event.
    fn to_emitter(self) -> FixedEmitter {
        FixedEmitter::new(vec![Event::NoteEvents(self)])
    }
}

impl ToFixedEmitter for ParameterChangeEvent {
    /// Wrap a [`ParameterChangeEvent`] into a new [`FixedEmitter`].
    fn to_emitter(self) -> FixedEmitter {
        FixedEmitter::new(vec![Event::ParameterChangeEvent(self)])
    }
}

// -------------------------------------------------------------------------------------------------

/// Convert a sequence of [`Event`]s into a [`FixedEmitter`].
pub trait ToFixedEmitterSequence {
    fn to_sequence_emitter(self) -> FixedEmitter;
}

impl ToFixedEmitterSequence for Vec<Option<NoteEvent>> {
    /// Wrap a vector of [`NoteEvent`]s to a new [`FixedEmitter`]
    /// resulting into a sequence of single note events.
    fn to_sequence_emitter(self) -> FixedEmitter {
        let mut sequence = Vec::with_capacity(self.len());
        for note in self {
            sequence.push(Event::NoteEvents(vec![note]));
        }
        FixedEmitter::new(sequence)
    }
}

impl ToFixedEmitterSequence for Vec<Vec<Option<NoteEvent>>> {
    /// Wrap a vector of vectors of [`NoteEvent`]s to a new [`FixedEmitter`]
    /// resulting into a sequence of polyphonic note events.
    fn to_sequence_emitter(self) -> FixedEmitter {
        let mut sequence = Vec::with_capacity(self.len());
        for notes in self {
            sequence.push(Event::NoteEvents(notes));
        }
        FixedEmitter::new(sequence)
    }
}

impl ToFixedEmitterSequence for Vec<ParameterChangeEvent> {
    /// Wrap a [`ParameterChangeEvent`] into a new [`FixedEmitter`]
    fn to_sequence_emitter(self) -> FixedEmitter {
        let mut sequence = Vec::with_capacity(self.len());
        for p in self {
            sequence.push(Event::ParameterChangeEvent(p));
        }
        FixedEmitter::new(sequence)
    }
}

// -------------------------------------------------------------------------------------------------

/// Shortcut for creating an [`Emitter`] which produces an empty note.
pub fn new_empty_note_emitter() -> FixedEmitter {
    new_empty_note().to_emitter()
}

/// Shortcut for creating an [`Emitter`] which produces a single note.
pub fn new_note_emitter<E: Into<NoteEvent>>(event: E) -> FixedEmitter {
    new_note(event).to_emitter()
}

/// Shortcut for creating an [`Emitter`] which produces a sequence of single note.
pub fn new_note_sequence_emitter<E: Into<NoteEvent>>(sequence: Vec<Option<E>>) -> FixedEmitter {
    new_note_vector(sequence).to_sequence_emitter()
}

/// Shortcut for creating an [`Emitter`] which produces a single note stack (a chord).
pub fn new_polyphonic_note_emitter<E: Into<NoteEvent>>(
    polyphonic_events: Vec<Option<E>>,
) -> FixedEmitter {
    new_note_vector(polyphonic_events).to_emitter()
}

/// Shortcut for creating an [`Emitter`] which produces a sequence of single notes or note stacks.
pub fn new_polyphonic_note_sequence_emitter<E: Into<NoteEvent>>(
    polyphonic_sequence: Vec<Vec<Option<E>>>,
) -> FixedEmitter {
    new_polyphonic_note_sequence(polyphonic_sequence).to_sequence_emitter()
}

/// Shortcut for creating an [`Emitter`] which produces parameter changes.
pub fn new_parameter_change_emitter<Parameter: Into<Option<ParameterId>>>(
    parameter: Parameter,
    value: f32,
) -> FixedEmitter {
    new_parameter_change(parameter, value).to_emitter()
}
