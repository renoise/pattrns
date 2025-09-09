use mlua::prelude::*;

use super::unwrap::{
    amplify_array_from_value, bad_argument_error, delay_array_from_value,
    instrument_array_from_value, note_events_from_value, panning_array_from_value,
    sequence_from_value, transpose_steps_array_from_value, volume_array_from_value,
};

use crate::{
    bindings::unwrap::glide_array_from_value,
    event::{InstrumentId, NoteEvent},
    note::Note,
};

// ---------------------------------------------------------------------------------------------

// Sequence
#[derive(Clone, Debug)]
pub struct SequenceUserData {
    pub notes: Vec<Vec<Option<NoteEvent>>>,
}

impl SequenceUserData {
    pub fn from(args: LuaMultiValue) -> LuaResult<Self> {
        // a single value, probably a sequence array
        if args.len() == 1 {
            let arg = args
                .front()
                .ok_or(LuaError::RuntimeError(
                    "Failed to access table content".to_string(),
                ))
                .cloned()?;
            if let Some(sequence) = sequence_from_value(&arg.clone()) {
                let mut notes = vec![];
                for (index, arg) in sequence.iter().enumerate() {
                    // add each sequence item as separate sequence event
                    notes.push(note_events_from_value(arg, Some(index))?);
                }
                Ok(SequenceUserData { notes })
            } else {
                Ok(SequenceUserData {
                    notes: vec![note_events_from_value(&arg, None)?],
                })
            }
        // multiple values, maybe of different type
        } else {
            let mut notes = vec![];
            for (index, arg) in args.iter().enumerate() {
                notes.push(note_events_from_value(arg, Some(index))?);
            }
            Ok(SequenceUserData { notes })
        }
    }
}

impl LuaUserData for SequenceUserData {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("notes", |lua, this| -> LuaResult<LuaTable> {
            let sequence = lua.create_table()?;
            for (index, note_events) in this.notes.iter().enumerate() {
                let notes = lua.create_table()?;
                for (index, note_event) in note_events.iter().enumerate() {
                    if let Some(note_event) = note_event {
                        notes.set(index + 1, note_event.clone().into_lua(lua)?)?;
                    } else {
                        notes.set(index + 1, LuaValue::Table(lua.create_table()?))?;
                    }
                }
                sequence.set(index + 1, notes)?;
            }
            Ok(sequence)
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_function(
            "transpose",
            |lua, (ud, volume): (LuaAnyUserData, LuaValue)| {
                let mut this = ud.borrow_mut::<Self>()?;
                let steps = transpose_steps_array_from_value(lua, volume, this.notes.len())?;
                for (notes, step) in this.notes.iter_mut().zip(steps.into_iter()) {
                    for note in notes.iter_mut().flatten() {
                        if note.note.is_note_on() {
                            let transposed_note =
                                (u8::from(note.note) as i32 + step).clamp(0, 0x7f);
                            note.note = Note::from(transposed_note as u8);
                        }
                    }
                }
                drop(this);
                Ok(ud)
            },
        );

        methods.add_function("amplify", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let volumes = amplify_array_from_value(lua, value, this.notes.len())?;
            for (notes, volume) in this.notes.iter_mut().zip(volumes) {
                if volume < 0.0 {
                    return Err(bad_argument_error(
                        "amplify",
                        "volume",
                        1,
                        "amplify value must be >= 0.0",
                    ));
                }
                for note in notes.iter_mut().flatten() {
                    note.volume = (note.volume * volume).clamp(0.0, 1.0);
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function(
            "instrument",
            |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
                let mut this = ud.borrow_mut::<Self>()?;
                let instruments = instrument_array_from_value(lua, value, this.notes.len())?;
                for (notes, instrument) in this.notes.iter_mut().zip(instruments) {
                    if instrument < 0 {
                        return Err(bad_argument_error(
                            "instrument",
                            "value",
                            1,
                            "instrument must be >= 0",
                        ));
                    }
                    for note in notes.iter_mut().flatten() {
                        note.instrument = Some(InstrumentId::from(instrument as usize));
                    }
                }
                drop(this);
                Ok(ud)
            },
        );

        methods.add_function("volume", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let volumes = volume_array_from_value(lua, value, this.notes.len())?;
            for (notes, volume) in this.notes.iter_mut().zip(volumes) {
                if !(0.0..=1.0).contains(&volume) {
                    return Err(bad_argument_error(
                        "volume",
                        "value",
                        1,
                        "volume must be in range [0.0..=1.0]",
                    ));
                }
                for note in notes.iter_mut().flatten() {
                    note.volume = volume;
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function("panning", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let pannings = panning_array_from_value(lua, value, this.notes.len())?;
            for (notes, panning) in this.notes.iter_mut().zip(pannings) {
                if !(-1.0..=1.0).contains(&panning) {
                    return Err(bad_argument_error(
                        "panning",
                        "value",
                        1,
                        "panning must be in range [-1.0..=1.0]",
                    ));
                }
                for note in notes.iter_mut().flatten() {
                    note.panning = panning;
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function("delay", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let delays = delay_array_from_value(lua, value, this.notes.len())?;
            for (notes, delay) in this.notes.iter_mut().zip(delays) {
                if !(0.0..=1.0).contains(&delay) {
                    return Err(bad_argument_error(
                        "delay",
                        "value",
                        1,
                        "delay must be in range [-1.0..=1.0]",
                    ));
                }
                for note in notes.iter_mut().flatten() {
                    note.delay = delay;
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function("glide", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let glides = glide_array_from_value(lua, value, this.notes.len())?;
            for (notes, glide) in this.notes.iter_mut().zip(glides) {
                if !(0.0..).contains(&glide) {
                    return Err(bad_argument_error(
                        "glide",
                        "value",
                        1,
                        "glide must be in range [0.0..]",
                    ));
                }
                for note in notes.iter_mut().flatten() {
                    note.glide = glide;
                }
            }
            drop(this);
            Ok(ud)
        });
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use crate::{bindings::*, event::new_note};

    fn evaluate_sequence_userdata(lua: &Lua, expression: &str) -> LuaResult<SequenceUserData> {
        Ok(lua
            .load(expression)
            .eval::<LuaValue>()?
            .as_userdata()
            .ok_or(LuaError::RuntimeError("No user data".to_string()))?
            .borrow::<SequenceUserData>()?
            .clone())
    }

    #[test]
    fn sequence() -> LuaResult<()> {
        // create a new engine and register bindings
        let (mut lua, mut timeout_hook) = new_engine()?;
        register_bindings(
            &mut lua,
            &timeout_hook,
            &BeatTimeBase {
                beats_per_min: 120.0,
                beats_per_bar: 4,
                samples_per_sec: 44100,
            },
        )?;

        // reset timeout
        timeout_hook.reset();

        // Note Sequence
        let note_sequence_event =
            evaluate_sequence_userdata(&lua, r#"sequence({"C#1 v0.5"}, "---", "G_2")"#)?;
        assert_eq!(
            note_sequence_event.notes,
            vec![
                vec![new_note(("c#1", None, 0.5))],
                vec![None],
                vec![new_note(("g2", None, 1.0))]
            ]
        );
        let poly_note_sequence_event = evaluate_sequence_userdata(
            &lua,
            r#"sequence(
                    {"C#1", "", "G_2 v0.75"},
                    {"A#5 v0.2", "---", {key = "B_1", volume = 0.1}}
                )"#,
        )?;
        assert_eq!(
            poly_note_sequence_event.notes,
            vec![
                vec![
                    new_note(("c#1", None, 1.0)),
                    None,
                    new_note(("g2", None, 0.75)),
                ],
                vec![
                    new_note(("a#5", None, 0.2)),
                    None,
                    new_note(("b1", None, 0.1))
                ]
            ]
        );

        let chord_sequence_event = evaluate_sequence_userdata(
            &lua, //
            r#"sequence("c'maj")"#,
        )?;
        assert_eq!(
            chord_sequence_event.notes,
            vec![vec![new_note("c4"), new_note("e4"), new_note("g4"),],]
        );

        let poly_chord_sequence_event = evaluate_sequence_userdata(
            &lua,
            r#"sequence("c'maj", {"as5 v0.2", "---", {key = "B_1", volume = 0.1}})"#,
        )?;
        assert_eq!(
            poly_chord_sequence_event.notes,
            vec![
                vec![new_note("c4"), new_note("e4"), new_note("g4"),],
                vec![
                    new_note(("a#5", None, 0.2)),
                    None,
                    new_note(("b1", None, 0.1))
                ]
            ]
        );

        let note_sequence_event =
            evaluate_sequence_userdata(&lua, r#"sequence{note{"c"}, note("d", "e"), {"f"}}"#)?;
        assert_eq!(
            note_sequence_event.notes,
            vec![
                vec![new_note("c"),],
                vec![new_note("d"), new_note("e"),],
                vec![new_note("f"),]
            ]
        );

        Ok(())
    }

    #[test]
    fn sequence_methods() -> LuaResult<()> {
        // create a new engine and register bindings
        let (mut lua, mut timeout_hook) = new_engine()?;
        register_bindings(
            &mut lua,
            &timeout_hook,
            &BeatTimeBase {
                beats_per_min: 120.0,
                beats_per_bar: 4,
                samples_per_sec: 44100,
            },
        )?;

        // reset timeout
        timeout_hook.reset();

        // notes
        assert_eq!(
            evaluate_sequence_userdata(
                &lua,
                r#"sequence(sequence{{"c4 #1 v0.2 p0.3 d0.4", "d4"}, {}, {"e4"}}.notes)"#
            )?
            .notes,
            vec![
                vec![
                    new_note(("c4", InstrumentId::from(1), 0.2, 0.3, 0.4)),
                    new_note("d4"),
                ],
                vec![None],
                vec![new_note("e4")],
            ]
        );

        // xxx
        assert!(evaluate_sequence_userdata(
            &lua, //
            r#"sequence("c", "d", "f"):transpose(1)"#
        )
        .is_ok());
        assert!(evaluate_sequence_userdata(
            &lua, //
            r#"sequence("c'maj"):volume(0.2)"#
        )
        .is_ok());
        assert!(evaluate_sequence_userdata(
            &lua, //
            r#"sequence(12, 24, 48):panning(0.0)"#
        )
        .is_ok());
        assert!(
            evaluate_sequence_userdata(&lua, r#"sequence({key = "c"}, "d", "f"):delay(0.0)"#)
                .is_ok()
        );
        assert!(
            evaluate_sequence_userdata(&lua, r#"sequence({key = "c"}, "d", "f"):glide(2.0)"#)
                .is_ok()
        );
        assert!(evaluate_sequence_userdata(
            &lua, //
            r#"sequence("c", "d", "f"):transpose({1, 2})"#
        )
        .is_ok());
        assert!(
            evaluate_sequence_userdata(&lua, r#"sequence("c", "d", "f"):volume({0.5, 1.0})"#)
                .is_ok()
        );
        assert!(
            evaluate_sequence_userdata(&lua, r#"sequence("c", "d", "f"):panning({0.0, 1.0})"#)
                .is_ok()
        );
        assert!(
            evaluate_sequence_userdata(&lua, r#"sequence("c", "d", "f"):delay({0.0, 0.25})"#)
                .is_ok()
        );
        assert!(
            evaluate_sequence_userdata(&lua, r#"sequence("c", "d", "f"):glide({0.0, 0.5})"#)
                .is_ok()
        );
        assert!(evaluate_sequence_userdata(
            &lua,
            r#"sequence("c", "d", "f"):volume(1.0):delay({0.0, 0.25})"#
        )
        .is_ok());

        Ok(())
    }
}
