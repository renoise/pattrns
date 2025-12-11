//! Experimental imperative-style music sequence generator engine with optional Lua bindings.

// -------------------------------------------------------------------------------------------------

// Clippy lints

#![warn(clippy::clone_on_ref_ptr)]
// Useful, but also annoying: enable and check every now and then
// #![warn(clippy::pedantic)]
#![allow(clippy::must_use_candidate)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::uninlined_format_args)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_lossless)]

// -------------------------------------------------------------------------------------------------

#[cfg(all(
    feature = "scripting",
    not(any(
        feature = "lua",
        feature = "lua-jit",
        feature = "luau",
        feature = "luau-jit"
    ))
))]
compile_error!(
    "When enabling the `scripting` feature, enable one of the lua interpreter features as well: `lua`, `lua-jit`, `luau` or `lua-jit`"
);

// -------------------------------------------------------------------------------------------------

// Internal mods
mod emitter;
mod event;
mod gate;
mod note;
mod parameter;
mod pattern;
mod phrase;
mod pulse;
mod rhythm;
mod sequence;
mod tidal;
mod time;

// Re-Exported basic Traits and Types
pub use crate::{
    emitter::{Emitter, EmitterEvent},
    event::{Event, EventTransform, InstrumentId, NoteEvent, ParameterChangeEvent, ParameterId},
    gate::Gate,
    note::{chord::Chord, scale::Scale, Note},
    parameter::{Parameter, ParameterSet, ParameterType},
    pattern::{Pattern, PatternEvent},
    phrase::{PatternSlot, Phrase},
    pulse::Pulse,
    rhythm::{Rhythm, RhythmEvent},
    sequence::Sequence,
    tidal::{
        Cycle, Event as CycleEvent, Span as CycleSpan, Target as CycleTarget, Value as CycleValue,
    },
    time::{
        BeatTimeBase, BeatTimeStep, ExactSampleTime, SampleTime, SampleTimeBase, SampleTimeDisplay,
        SecondTimeBase,
    },
};

/// Default [`Pattern`] impls.
pub mod patterns {
    pub use super::pattern::{beat_time::BeatTimePattern, second_time::SecondTimePattern};
}

/// Default [`Rhythm`] impls.
pub mod rhythms {
    pub use super::rhythm::{empty::EmptyRhythm, fixed::FixedRhythm};

    #[cfg(feature = "scripting")]
    pub use crate::rhythm::scripted::ScriptedRhythm;
}

/// Default [`Gate`] impls.
pub mod gates {
    pub use super::gate::{probability::ProbabilityGate, threshold::ThresholdGate};

    #[cfg(feature = "scripting")]
    pub use super::gate::scripted::ScriptedGate;
}

/// Default [`Emitter`] impls.
pub mod emitters {
    pub use super::emitter::{
        cycle::CycleEmitter, empty::EmptyEmitter, fixed::FixedEmitter, mutated::MutatedEmitter,
    };

    #[cfg(feature = "scripting")]
    pub use super::emitter::{scripted::ScriptedEmitter, scripted_cycle::ScriptedCycleEmitter};
}

// Public modules
#[cfg(feature = "scripting")]
pub mod bindings;
#[cfg(feature = "player")]
pub mod player;

// Prelude
pub mod prelude;
