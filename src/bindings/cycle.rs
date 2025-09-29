use mlua::prelude::*;

use crate::{event::NoteEvent, tidal::Cycle};

use super::unwrap::{bad_argument_error, note_events_from_value};

// ---------------------------------------------------------------------------------------------

/// Cycle Userdata in bindings
#[derive(Clone, Debug)]
pub struct CycleUserData {
    pub cycle: Cycle,
    pub mappings: Vec<(String, Vec<Option<NoteEvent>>)>,
    pub mapping_function: Option<LuaFunction>,
}

impl CycleUserData {
    pub fn from(arg: LuaString, seed: Option<u64>) -> LuaResult<Self> {
        let mut cycle = Cycle::from(&arg.to_string_lossy()).map_err(LuaError::runtime)?;
        if let Some(seed) = seed {
            cycle = cycle.with_seed(seed);
        }
        let mappings = Vec::new();
        let mapping_function = None;
        Ok(CycleUserData {
            cycle,
            mappings,
            mapping_function,
        })
    }
}

impl LuaUserData for CycleUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("map", |_lua, this, value: LuaValue| match value {
            LuaValue::Function(func) => {
                let cycle = this.cycle.clone();
                let mappings = Vec::new();
                let mapping_function = Some(func);
                Ok(CycleUserData {
                    cycle,
                    mappings,
                    mapping_function,
                })
            }
            LuaValue::Table(table) => {
                let cycle = this.cycle.clone();
                let mut mappings = Vec::new();
                for (k, v) in table.pairs::<LuaValue, LuaValue>().flatten() {
                    mappings.push((k.to_string()?, note_events_from_value(&v, None)?));
                }
                let mapping_function = None;
                Ok(CycleUserData {
                    cycle,
                    mappings,
                    mapping_function,
                })
            }
            _ => Err(bad_argument_error(
                None,
                "map",
                1,
                format!(
                    "map argument must be a table but is a '{}'",
                    value.type_name()
                )
                .as_str(),
            )),
        });
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;

    use crate::{
        bindings::*,
        emitter::{cycle::CycleEmitter, scripted_cycle::ScriptedCycleEmitter},
        event::new_note,
        Emitter, Event, Note, RhythmEvent,
    };

    use pretty_assertions::assert_eq;

    fn new_test_engine() -> LuaResult<(Lua, LuaTimeoutHook)> {
        new_test_engine_with_timebase(&BeatTimeBase {
            beats_per_min: 120.0,
            beats_per_bar: 4,
            samples_per_sec: 44100,
        })
    }

    fn new_test_engine_with_timebase(time_base: &BeatTimeBase) -> LuaResult<(Lua, LuaTimeoutHook)> {
        let (mut lua, mut timeout_hook) = new_engine()?;
        register_bindings(&mut lua, &timeout_hook, time_base)?;
        timeout_hook.reset();
        Ok((lua, timeout_hook))
    }

    fn evaluate_cycle_userdata(lua: &Lua, expression: &str) -> LuaResult<CycleUserData> {
        Ok(lua
            .load(expression)
            .eval::<LuaValue>()?
            .as_userdata()
            .ok_or(LuaError::RuntimeError("No user data".to_string()))?
            .borrow::<CycleUserData>()?
            .clone())
    }

    #[test]
    fn parse() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;
        assert!(evaluate_cycle_userdata(&lua, r#"cycle("[<")"#).is_err());
        assert!(evaluate_cycle_userdata(&lua, r#"cycle("[c4 e6]")"#).is_ok());

        Ok(())
    }

    #[test]
    fn mappings() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        let mapped_cycle = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a b c"):map({a = "c0", b = 48, c = { key = "c6" }})"#,
        )?;
        assert_eq!(
            mapped_cycle
                .mappings
                .clone()
                .into_iter()
                .collect::<HashMap<_, _>>(),
            HashMap::from([
                ("a".to_string(), vec![new_note(Note::C0)]),
                ("b".to_string(), vec![new_note(Note::C4)]),
                ("c".to_string(), vec![new_note(Note::C6)]),
            ])
        );

        // check if mappings are applied correctly
        let mut event_iter =
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mappings);
        assert_eq!(
            event_iter
                .run(RhythmEvent::default(), true)
                .map(|events| events.into_iter().map(|e| e.event).collect::<Vec<_>>()),
            Some(vec![
                Event::NoteEvents(vec![new_note(Note::C0)]),
                Event::NoteEvents(vec![new_note(Note::C4)]),
                Event::NoteEvents(vec![new_note(Note::C6)])
            ])
        );

        // check note properties
        let mapped_cycle = evaluate_cycle_userdata(&lua, r#"cycle("a:1:g0.1:v0.1:p-1.0:d0.3")"#)?;
        let mut event_iter =
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mappings);
        assert_eq!(
            event_iter
                .run(RhythmEvent::default(), true)
                .map(|events| events.into_iter().map(|e| e.event).collect::<Vec<_>>()),
            Some(vec![Event::NoteEvents(vec![new_note((
                Note::A4,
                InstrumentId::from(1),
                Some(0.1),
                0.1,
                -1.0,
                0.3,
            ))]),])
        );

        // check instrument overrides
        let mapped_cycle = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a:1 a:2 a"):map({ a = { key = 48, instrument = 66 } })"#,
        )?;
        let mut event_iter =
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mappings);
        assert_eq!(
            event_iter
                .run(RhythmEvent::default(), true)
                .map(|events| events.into_iter().map(|e| e.event).collect::<Vec<_>>()),
            Some(vec![
                Event::NoteEvents(vec![new_note((Note::C4, InstrumentId::from(1)))]),
                Event::NoteEvents(vec![new_note((Note::C4, InstrumentId::from(2)))]),
                Event::NoteEvents(vec![new_note((Note::C4, InstrumentId::from(66)))])
            ])
        );

        // check note property overrides
        let mapped_cycle = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a:1:v.1 a"):map({ a = { key = 48, instrument = 66, volume = 1.0 } })"#,
        )?;
        let mut event_iter =
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mappings);
        assert_eq!(
            event_iter
                .run(RhythmEvent::default(), true)
                .map(|events| events.into_iter().map(|e| e.event).collect::<Vec<_>>()),
            Some(vec![
                Event::NoteEvents(vec![new_note((Note::C4, InstrumentId::from(1), None, 0.1))]),
                Event::NoteEvents(vec![new_note((
                    Note::C4,
                    InstrumentId::from(66),
                    None,
                    1.0
                ))])
            ])
        );

        Ok(())
    }

    #[test]
    fn mapping_functions() -> LuaResult<()> {
        let time_base = BeatTimeBase {
            beats_per_min: 120.0,
            beats_per_bar: 4,
            samples_per_sec: 44100,
        };

        let (lua, timeout_hook) = new_test_engine_with_timebase(&time_base)?;

        let mapped_cycle = evaluate_cycle_userdata(
            &lua,
            r#"
                cycle("wurst a b c"):map(function(context, value) 
                    assert(context.beats_per_min, 120)
                    assert(context.beats_per_bar, 4)
                    assert(context.samples_per_sec, 44100)
                    if value == "wurst" then
                      return "c#4"
                    else
                      return value..4
                    end
                end)"#,
        )?;
        let mapping_callback =
            LuaCallback::new(&lua, mapped_cycle.mapping_function.unwrap().clone())?;
        let mut event_iter = ScriptedCycleEmitter::with_mapping_callback(
            mapped_cycle.cycle,
            &timeout_hook,
            mapping_callback,
            &time_base,
        )?;
        assert_eq!(
            event_iter
                .run(RhythmEvent::default(), true)
                .map(|events| events.into_iter().map(|e| e.event).collect::<Vec<_>>()),
            Some(vec![
                Event::NoteEvents(vec![new_note(Note::Cs4)]),
                Event::NoteEvents(vec![new_note(Note::A4)]),
                Event::NoteEvents(vec![new_note(Note::B4)]),
                Event::NoteEvents(vec![new_note(Note::C4)])
            ])
        );
        Ok(())
    }
}
