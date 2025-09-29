use mlua::prelude::*;

use super::unwrap::{
    amplify_array_from_value, bad_argument_error, chord_events_from_intervals,
    chord_events_from_mode, delay_array_from_value, glide_array_from_value,
    instrument_array_from_value, note_events_from_value, panning_array_from_value,
    sequence_from_value, transpose_steps_array_from_value, volume_array_from_value,
};

use crate::{
    event::{InstrumentId, NoteEvent},
    note::Note,
};

// ---------------------------------------------------------------------------------------------

/// Note Userdata in bindings
#[derive(Clone, Debug)]
pub struct NoteUserData {
    pub notes: Vec<Option<NoteEvent>>,
}

impl NoteUserData {
    pub fn from(args: LuaMultiValue) -> LuaResult<Self> {
        // a single value, probably a sequence
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
                    // flatten sequence events into a single array
                    notes.append(&mut note_events_from_value(arg, Some(index))?);
                }
                Ok(NoteUserData { notes })
            } else {
                Ok(NoteUserData {
                    notes: note_events_from_value(&arg, None)?,
                })
            }
        // multiple values, maybe of different type
        } else {
            let mut notes = vec![];
            for (index, arg) in args.iter().enumerate() {
                notes.append(&mut note_events_from_value(arg, Some(index))?);
            }
            Ok(NoteUserData { notes })
        }
    }

    pub fn from_chord(note: &LuaValue, mode_or_intervals: &LuaValue) -> LuaResult<Self> {
        if let Some(mode) = mode_or_intervals.as_string() {
            let notes = chord_events_from_mode(note, &mode.to_string_lossy())?;
            Ok(Self { notes })
        } else if let Some(table) = mode_or_intervals.as_table() {
            let intervals = table
                .clone()
                .sequence_values::<i32>()
                .collect::<LuaResult<Vec<i32>>>()?;
            let notes = chord_events_from_intervals(note, &intervals)?;
            Ok(Self { notes })
        } else {
            Err(bad_argument_error(
                "chord",
                "mode",
                1,
                "expecting a mode string or an interval array as second argument",
            ))
        }
    }
}

impl LuaUserData for NoteUserData {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("notes", |lua, this| -> LuaResult<LuaTable> {
            let sequence = lua.create_table()?;
            for (index, note_event) in this.notes.iter().enumerate() {
                if let Some(note_event) = note_event {
                    sequence.set(index + 1, note_event.clone().into_lua(lua)?)?;
                } else {
                    sequence.set(index + 1, LuaValue::Table(lua.create_table()?))?;
                }
            }
            Ok(sequence)
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_function(
            "transpose",
            |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
                let mut this = ud.borrow_mut::<Self>()?;
                let steps = transpose_steps_array_from_value(lua, value, this.notes.len())?;
                for (note, step) in this.notes.iter_mut().zip(steps.into_iter()) {
                    if let Some(note) = note {
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
            for (note, volume) in this.notes.iter_mut().zip(volumes.into_iter()) {
                if volume < 0.0 {
                    return Err(bad_argument_error(
                        "amplify",
                        "value",
                        1,
                        "amplify value must be >= 0.0",
                    ));
                }
                if let Some(note) = note {
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
                for (note, instrument) in this.notes.iter_mut().zip(instruments.into_iter()) {
                    if instrument < 0 {
                        return Err(bad_argument_error(
                            "instrument",
                            "value",
                            1,
                            "instrument must be >= 0",
                        ));
                    }
                    if let Some(note) = note {
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
            for (note, volume) in this.notes.iter_mut().zip(volumes.into_iter()) {
                if !(0.0..=1.0).contains(&volume) {
                    return Err(bad_argument_error(
                        "volume",
                        "value",
                        1,
                        "volume must be in range [0.0..=1.0]",
                    ));
                }
                if let Some(note) = note {
                    note.volume = volume;
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function("panning", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let pannings = panning_array_from_value(lua, value, this.notes.len())?;
            for (note, panning) in this.notes.iter_mut().zip(pannings.into_iter()) {
                if !(-1.0..=1.0).contains(&panning) {
                    return Err(bad_argument_error(
                        "panning",
                        "value",
                        1,
                        "panning must be in range [-1.0..=1.0]",
                    ));
                }
                if let Some(note) = note {
                    note.panning = panning;
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function("delay", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let delays = delay_array_from_value(lua, value, this.notes.len())?;
            for (note, delay) in this.notes.iter_mut().zip(delays.into_iter()) {
                if !(0.0..=1.0).contains(&delay) {
                    return Err(bad_argument_error(
                        "delay",
                        "value",
                        1,
                        "delay must be in range [-1.0..=1.0]",
                    ));
                }
                if let Some(note) = note {
                    note.delay = delay;
                }
            }
            drop(this);
            Ok(ud)
        });

        methods.add_function("glide", |lua, (ud, value): (LuaAnyUserData, LuaValue)| {
            let mut this = ud.borrow_mut::<Self>()?;
            let glides = glide_array_from_value(lua, value, this.notes.len())?;
            for (note, glide) in this.notes.iter_mut().zip(glides.into_iter()) {
                if let Some(glide) = glide {
                    if !(0.0..).contains(&glide) {
                        return Err(bad_argument_error(
                            "glide",
                            "value",
                            1,
                            "glide must be in range [0.0..]",
                        ));
                    }
                }
                if let Some(note) = note {
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

    use pretty_assertions::assert_eq;

    fn new_test_engine() -> LuaResult<(Lua, LuaTimeoutHook)> {
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
        timeout_hook.reset();
        Ok((lua, timeout_hook))
    }

    fn evaluate_note_userdata(lua: &Lua, expression: &str) -> LuaResult<NoteUserData> {
        Ok(lua
            .load(expression)
            .eval::<LuaValue>()?
            .as_userdata()
            .ok_or(LuaError::RuntimeError("No user data".to_string()))?
            .borrow::<NoteUserData>()?
            .clone())
    }

    fn evaluate_number(lua: &Lua, expression: &str) -> LuaResult<LuaInteger> {
        lua.load(expression)
            .eval::<LuaValue>()?
            .as_integer()
            .ok_or(LuaError::RuntimeError("No integer value".to_string()))
    }

    #[test]
    fn note() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // Empty Note
        let note_event = evaluate_note_userdata(&lua, r#"note("---")"#)?;
        assert_eq!(note_event.notes, vec![None]);

        // Note Off
        assert!(evaluate_note_userdata(&lua, r#"note("off")"#).is_ok());
        let note_event = evaluate_note_userdata(&lua, r#"note("OFF")"#)?;
        assert_eq!(note_event.notes, vec![new_note(Note::OFF)]);

        // Note string
        assert!(evaluate_note_userdata(&lua, r#"note("X#1")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("0.5")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("0xfff")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 -0.5")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1", 0.5)"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1")"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 #a")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 #-10")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 v-2.0")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 p-1.0")"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 d-1.0")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 g-1.0")"#).is_err());
        let note_event = evaluate_note_userdata(&lua, r#"note("C#1 #2 v0.5 p0.1 d0.2 g5")"#)?;
        assert_eq!(
            note_event.notes,
            vec![new_note((
                Note::Cs1,
                InstrumentId::from(2),
                Some(5.0),
                0.5,
                0.1,
                0.2,
            ))]
        );
        let note_event = evaluate_note_userdata(&lua, r#"note("C#1 d0.2")"#)?;
        assert_eq!(
            note_event.notes,
            vec![new_note((Note::Cs1, None, None, 1.0, 0.0, 0.2))]
        );

        // Note string array
        assert!(evaluate_note_userdata(&lua, r#"note({"X#1"})"#).is_err());
        let note_event = evaluate_note_userdata(&lua, r#"note({"C#1"})"#)?;
        assert_eq!(note_event.notes, vec![new_note("c#1")]);

        assert!(evaluate_note_userdata(&lua, r#"note("X#1")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("C#1 abc")"#).is_err());
        let note_event = evaluate_note_userdata(&lua, r#"note({"C#1 v0.5", "C5"})"#)?;
        assert_eq!(
            note_event.notes,
            vec![
                new_note(("c#1", None, None, 0.5)),
                new_note(("c5", None, None, 1.0))
            ]
        );

        // Note int
        let note_event = evaluate_note_userdata(&lua, r#"note(0x32)"#)?;
        assert_eq!(note_event.notes, vec![new_note("d4")]);

        // Note int string
        let note_event = evaluate_note_userdata(&lua, r#"note("32")"#)?;
        assert_eq!(note_event.notes, vec![new_note("g#2")]);
        let note_event = evaluate_note_userdata(&lua, r#"note("0x32")"#)?;
        assert_eq!(note_event.notes, vec![new_note("d4")]);

        // Note int array
        let note_event = evaluate_note_userdata(&lua, r#"note({0x32, 48})"#)?;
        assert_eq!(note_event.notes, vec![new_note("d4"), new_note("c4")]);

        // Note int string array
        let note_event = evaluate_note_userdata(&lua, r#"note({"0X32", "48"})"#)?;
        assert_eq!(note_event.notes, vec![new_note("d4"), new_note("c4")]);

        // Note table
        assert!(evaluate_note_userdata(&lua, r#"note({volume = 0.5})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note({key = "xxx", volume = 0.5})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note({key = "C#1", volume = "abc"})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note({key = "C#1", volume = -1})"#).is_err());
        let note_event = evaluate_note_userdata(&lua, r#"note({key = "c8"})"#)?;
        assert_eq!(note_event.notes, vec![new_note("c8")]);
        let note_event = evaluate_note_userdata(&lua, r#"note({key = "G8", volume = 0.5})"#)?;
        assert_eq!(note_event.notes, vec![new_note(("g8", None, None, 0.5))]);
        let note_event = evaluate_note_userdata(&lua, r#"note({key = 60})"#)?;
        assert_eq!(note_event.notes, vec![new_note("c5")]);
        let note_event = evaluate_note_userdata(&lua, r#"note({key = "60"})"#)?;
        assert_eq!(note_event.notes, vec![new_note("c5")]);

        // Note table or array
        let poly_note_event = evaluate_note_userdata(
            &lua,
            r#"note({{key = "C#1", volume = 0.5}, {key = "G2", volume = 0.75}, {}})"#,
        )?;
        assert_eq!(
            poly_note_event.notes,
            vec![
                new_note(("c#1", None, None, 0.5)),
                new_note(("g2", None, None, 0.75)),
                None
            ]
        );

        // nested notes
        assert_eq!(
            evaluate_note_userdata(
                &lua,
                r#"note(note("c4 #100 g8 v0.2 p0.3 d0.4", "", "e4").notes)"#
            )?
            .notes,
            vec![
                new_note(("c4", InstrumentId::from(100), Some(8.0), 0.2, 0.3, 0.4,)),
                None,
                new_note("e4"),
            ]
        );

        Ok(())
    }

    #[test]
    fn note_chord() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // Note chord
        assert!(evaluate_note_userdata(&lua, r#"note("c12'maj")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("j'maj")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("128'maj")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4'invalid")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4'maj'")"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4'maj xx")"#).is_err());

        assert!(
            evaluate_note_userdata(&lua, r#"note(string.format("c4'%s", chord_names()[1]))"#)
                .is_ok()
        );

        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c'maj")"#)?.notes,
            vec![new_note("c4"), new_note("e4"), new_note("g4"),]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("60'maj")"#)?.notes,
            vec![new_note("c5"), new_note("e5"), new_note("g5"),]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("0x30'maj")"#)?.notes,
            vec![new_note("c4"), new_note("e4"), new_note("g4"),]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c7'maj v0.2")"#)?.notes,
            vec![
                new_note(("c7", None, None, 0.2)),
                new_note(("e7", None, None, 0.2)),
                new_note(("g7", None, None, 0.2)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4'm v0.2", "c7")"#)?.notes,
            vec![
                new_note(("c4", None, None, 0.2)),
                new_note(("d#4", None, None, 0.2)),
                new_note(("g4", None, None, 0.2)),
                new_note("c7"),
            ]
        );

        Ok(())
    }

    #[test]
    fn note_transpose() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // transpose
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):transpose(12)"#)?.notes,
            vec![new_note("c5"), new_note("d5"), new_note("e5"),]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):transpose({2, 4})"#)?.notes,
            vec![new_note("d_4"), new_note("f#4"), new_note("e_4"),]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "c4"):transpose({-1000, 1000})"#)?.notes,
            vec![new_note(0x0_u8), new_note(0x7f_u8),]
        );

        Ok(())
    }

    #[test]
    fn note_volume() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // volume
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):volume(1.0)"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):volume()"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):volume(-1)"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):volume({})"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):volume({"wurst"})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):volume({-1})"#).is_err());
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4 v0.5", "d4 v0.5", "e4 v0.5"):volume(0.2)"#)?
                .notes,
            vec![
                new_note(("c4", None, None, 0.2)),
                new_note(("d4", None, None, 0.2)),
                new_note(("e4", None, None, 0.2)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(
                &lua,
                r#"note("c4 v0.5", "d4 v0.5", "e4 v0.5"):volume({0.0, 0.0})"#
            )?
            .notes,
            vec![
                new_note(("c4", None, None, 0.0)),
                new_note(("d4", None, None, 0.0)),
                new_note(("e4", None, None, 0.5)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(
                &lua,
                r#"note("c4 v0.5", "d4 v0.5", "e4 v0.5"):volume({0.1, 0.2, 0.3})"#
            )?
            .notes,
            vec![
                new_note(("c4", None, None, 0.1)),
                new_note(("d4", None, None, 0.2)),
                new_note(("e4", None, None, 0.3)),
            ]
        );

        // amplify
        assert_eq!(
            evaluate_note_userdata(
                &lua,
                r#"note("c4 v0.5", "d4 v0.5", "e4 v0.5"):amplify(200.0)"#
            )?
            .notes,
            vec![
                new_note(("c4", None, None, 1.0)),
                new_note(("d4", None, None, 1.0)),
                new_note(("e4", None, None, 1.0)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(
                &lua,
                r#"note("c4 v0.25", "d4 v0.25", "e4 v0.5"):amplify({2.0, 4.0})"#
            )?
            .notes,
            vec![
                new_note(("c4", None, None, 0.5)),
                new_note(("d4", None, None, 1.0)),
                new_note(("e4", None, None, 0.5)),
            ]
        );
        Ok(())
    }

    #[test]
    fn note_panning() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // panning
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):panning(1.0)"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):panning()"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):panning(-2)"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):panning({})"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):panning({"wurst"})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):panning({2})"#).is_err());
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):panning(-1.0)"#)?.notes,
            vec![
                new_note(("c4", None, None, 1.0, -1.0)),
                new_note(("d4", None, None, 1.0, -1.0)),
                new_note(("e4", None, None, 1.0, -1.0)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):panning({-1.0, 1.0})"#)?.notes,
            vec![
                new_note(("c4", None, None, 1.0, -1.0)),
                new_note(("d4", None, None, 1.0, 1.0)),
                new_note(("e4", None, None, 1.0, 0.0)),
            ]
        );
        Ok(())
    }

    #[test]
    fn note_delay() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // delay
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):delay(1.0)"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):delay()"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):delay(-1)"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):delay({})"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):delay({"wurst"})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):delay({2})"#).is_err());
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):delay(0.75)"#)?.notes,
            vec![
                new_note(("c4", None, None, 1.0, 0.0, 0.75)),
                new_note(("d4", None, None, 1.0, 0.0, 0.75)),
                new_note(("e4", None, None, 1.0, 0.0, 0.75)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):delay({0.25, 0.5})"#)?.notes,
            vec![
                new_note(("c4", None, None, 1.0, 0.0, 0.25)),
                new_note(("d4", None, None, 1.0, 0.0, 0.5)),
                new_note(("e4", None, None, 1.0, 0.0)),
            ]
        );

        Ok(())
    }

    #[test]
    fn note_glide() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // delay
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):glide(1.0)"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):glide()"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):glide(-1)"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):glide({})"#).is_ok());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):glide({"wurst"})"#).is_err());
        assert!(evaluate_note_userdata(&lua, r#"note("c4"):glide({nil, 1})"#).is_ok());
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):glide(0.5)"#)?.notes,
            vec![
                new_note(("c4", None, Some(0.5), 1.0, 0.0, 0.0,)),
                new_note(("d4", None, Some(0.5), 1.0, 0.0, 0.0,)),
                new_note(("e4", None, Some(0.5), 1.0, 0.0, 0.0,)),
            ]
        );
        assert_eq!(
            evaluate_note_userdata(&lua, r#"note("c4", "d4", "e4"):glide({0.25, nil, 0.5})"#)?
                .notes,
            vec![
                new_note(("c4", None, Some(0.25), 1.0, 0.0, 0.0,)),
                new_note(("d4", None, None, 1.0, 0.0, 0.0)),
                new_note(("e4", None, Some(0.5), 1.0, 0.0, 0.0)),
            ]
        );

        Ok(())
    }

    #[test]
    fn note_numbers() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        // Note number
        assert!(evaluate_number(
            &lua,
            r#"note_number("x1")#).is_err());
        assert!(evaluate_number(&lua, r#"note_number({})"#
        )
        .is_err());
        assert!(evaluate_number(&lua, r#"note_number(0x80)"#).is_err());
        assert!(evaluate_number(&lua, r#"note_number(-1)"#).is_err());
        assert_eq!(evaluate_number(&lua, r#"note_number("c4")"#)?, 48);
        assert_eq!(evaluate_number(&lua, r#"note_number("---")"#)?, 0xFE);
        assert_eq!(evaluate_number(&lua, r#"note_number(nil)"#)?, 0xFE);
        assert_eq!(evaluate_number(&lua, r#"note_number("off")"#)?, 0xFF);

        Ok(())
    }
}
