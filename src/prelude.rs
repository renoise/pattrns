//! Alleviates imports of common traits, when using pattrns as library.
//!
//! ```
//! # #![allow(unused_imports)]
//! use pattrns::prelude::*;
//! ```

pub use super::{
    // all public types to create emitters, gates and patterns
    emitter::{
        cycle::{new_cycle_emitter, new_cycle_emitter_with_seed, CycleEmitter},
        fixed::{ToFixedEmitter, ToFixedEmitterSequence},
        mutated::ToMutatedEmitter,
        new_empty_note_emitter, new_note_emitter, new_note_sequence_emitter,
        new_parameter_change_emitter, new_polyphonic_note_emitter,
        new_polyphonic_note_sequence_emitter,
    },
    event::{new_empty_note, new_note, InstrumentId, NoteEvent, ParameterChangeEvent, ParameterId},
    gate::{probability::ProbabilityGate, threshold::ThresholdGate},
    pattern::{beat_time::BeatTimePattern, second_time::SecondTimePattern},
    rhythm::{euclidean, fixed::ToFixedRhythm},
    time::{BeatTimeStep, SecondTimeStep},
    // all public basic types
    BeatTimeBase,
    Chord,
    Cycle,
    CycleEvent,
    CycleSpan,
    CycleTarget,
    CycleValue,
    Emitter,
    EmitterEvent,
    Event,
    EventTransform,
    Gate,
    Note,
    Parameter,
    ParameterSet,
    ParameterType,
    Pattern,
    PatternEvent,
    PatternSlot,
    Phrase,
    Pulse,
    Rhythm,
    SampleTime,
    SampleTimeBase,
    Scale,
    SecondTimeBase,
    Sequence,
};

#[cfg(feature = "scripting")]
// all public scripting types
pub use super::{
    bindings::{
        clear_lua_callback_errors, has_lua_callback_errors, lua_callback_errors,
        new_pattern_from_file, new_pattern_from_string, pattern_script_file_extensions,
    },
    emitter::{scripted::ScriptedEmitter, scripted_cycle::ScriptedCycleEmitter},
    gate::scripted::ScriptedGate,
    rhythm::scripted::ScriptedRhythm,
};

#[cfg(feature = "player")]
// all public player types
pub use super::player::{NewNoteAction, SamplePlaybackContext, SamplePlayer, SamplePool};
