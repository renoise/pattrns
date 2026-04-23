use std::collections::HashMap;

use mlua::prelude::*;

use crate::{
    bindings::LuaCallback, emitter::scripted_cycle::ScriptedCycleMapping, event::NoteEvent, Cycle,
    CycleSubCycle,
};

use super::unwrap::{bad_argument_error, note_events_from_value, subcycle_values_from_table};

// ---------------------------------------------------------------------------------------------

/// Cycle Userdata in bindings
#[derive(Clone, Debug)]
pub struct CycleUserData {
    pub cycle: Cycle,
    pub mappings: ScriptedCycleMapping<Vec<Option<NoteEvent>>>,
    pub variables: ScriptedCycleMapping<CycleSubCycle>,
}

impl CycleUserData {
    pub fn from(arg: LuaString, source: &str, seed: Option<u64>) -> LuaResult<Self> {
        let mut cycle = Cycle::from(&arg.to_string_lossy()).map_err(LuaError::runtime)?;
        if !source.is_empty() {
            cycle = cycle.with_source(source);
        }
        if let Some(seed) = seed {
            cycle = cycle.with_seed(seed);
        }
        let mapping = ScriptedCycleMapping::default();
        let variables = ScriptedCycleMapping::default();
        Ok(CycleUserData {
            cycle,
            mappings: mapping,
            variables,
        })
    }
}

impl LuaUserData for CycleUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("map", |lua, this, value: LuaValue| match value {
            LuaValue::Function(func) => Ok(Self {
                mappings: ScriptedCycleMapping::Function(LuaCallback::new(lua, func)?),
                ..this.clone()
            }),
            LuaValue::Table(table) => {
                let mut mappings = HashMap::with_capacity(table.raw_len());
                for (k, v) in table.pairs::<LuaValue, LuaValue>().flatten() {
                    mappings.insert(k.to_string()?, note_events_from_value(&v, None)?);
                }
                Ok(Self {
                    mappings: ScriptedCycleMapping::Table(mappings),
                    ..this.clone()
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

        methods.add_method_mut("var", |lua, this, value: LuaValue| match value {
            LuaValue::Table(table) => Ok(Self {
                variables: ScriptedCycleMapping::Table(subcycle_values_from_table(table)?),
                ..this.clone()
            }),
            LuaValue::Function(func) => Ok(Self {
                variables: ScriptedCycleMapping::Function(LuaCallback::new(lua, func)?),
                ..this.clone()
            }),
            _ => Err(bad_argument_error(
                None,
                "var",
                1,
                format!(
                    "var argument must be a table or a function but is a '{}'",
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
    use super::*;

    use crate::{
        bindings::*, emitter::scripted_cycle::ScriptedCycleEmitter, event::new_note, CycleSubCycle,
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
    fn variables() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        let cycle_userdata = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a b c"):var({x = "c0", y = "b4:v0.5", z = "[a b c]"})"#,
        )?;
        let variables_table = match &cycle_userdata.variables {
            ScriptedCycleMapping::Table(map) => map,
            ScriptedCycleMapping::Function(_) => panic!("Expected a variables table here"),
        };

        assert!(!variables_table.contains_key("a"));
        assert_eq!(
            variables_table["x"],
            CycleSubCycle::from("c0").expect("valid subcycle")
        );
        assert_eq!(
            variables_table["y"],
            CycleSubCycle::from("b4:v0.5").expect("valid subcycle")
        );
        assert!(variables_table.contains_key("z"));

        Ok(())
    }

    #[test]
    fn variables_function() -> LuaResult<()> {
        let time_base = BeatTimeBase {
            beats_per_min: 120.0,
            beats_per_bar: 4,
            samples_per_sec: 44100,
        };

        let (lua, timeout_hook) = new_test_engine_with_timebase(&time_base)?;

        let cycle_userdata = evaluate_cycle_userdata(
            &lua,
            r#"
                cycle("a b c $x $y $z"):var(function(context)
                    return {
                        x = "d" .. context.iteration,
                        y = "e",
                        z = "f",
                    }
                end)"#,
        )?;
        let variables_callback = match &cycle_userdata.variables {
            ScriptedCycleMapping::Function(f) => f.clone(),
            ScriptedCycleMapping::Table(_) => panic!("Expected a variables callback here"),
        };

        let mut event_iter = ScriptedCycleEmitter::new(cycle_userdata.cycle)
            .with_variables_callback(variables_callback, &timeout_hook, &time_base)?;
        assert_eq!(
            event_iter
                .run(RhythmEvent::default(), true)
                .map(|events| events.into_iter().map(|e| e.event).collect::<Vec<_>>()),
            Some(vec![
                Event::NoteEvents(vec![new_note(Note::A4)]),
                Event::NoteEvents(vec![new_note(Note::B4)]),
                Event::NoteEvents(vec![new_note(Note::C4)]),
                Event::NoteEvents(vec![new_note(Note::D1)]),
                Event::NoteEvents(vec![new_note(Note::E4)]),
                Event::NoteEvents(vec![new_note(Note::F4)]),
            ])
        );

        Ok(())
    }

    #[test]
    fn mappings() -> LuaResult<()> {
        let (lua, _) = new_test_engine()?;

        let cycle_userdata = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a b c"):map({a = "c0", b = 48, c = { key = "c6" }})"#,
        )?;
        let mappings_table = match &cycle_userdata.mappings {
            ScriptedCycleMapping::Table(map) => map.clone(),
            ScriptedCycleMapping::Function(_) => panic!("Expected a mapping table to be set here"),
        };
        assert_eq!(
            mappings_table,
            HashMap::from([
                ("a".to_string(), vec![new_note(Note::C0)]),
                ("b".to_string(), vec![new_note(Note::C4)]),
                ("c".to_string(), vec![new_note(Note::C6)]),
            ])
        );

        // check if mappings are applied correctly
        let mut event_iter =
            ScriptedCycleEmitter::new(cycle_userdata.cycle).with_mappings(mappings_table);
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

        // check instrument overrides
        let cycle_userdata = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a:1 a:2 a"):map({ a = { key = 48, instrument = 66 } })"#,
        )?;
        let mappings_table = match &cycle_userdata.mappings {
            ScriptedCycleMapping::Table(map) => map.clone(),
            ScriptedCycleMapping::Function(_) => {
                panic!("Expected a mapping table to be set here")
            }
        };

        let mut event_iter =
            ScriptedCycleEmitter::new(cycle_userdata.cycle).with_mappings(mappings_table);
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
        let cycle_userdata = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a:1:v.1 a"):map({ a = { key = 48, instrument = 66, volume = 1.0 } })"#,
        )?;
        let mappings_table = match &cycle_userdata.mappings {
            ScriptedCycleMapping::Table(map) => map.clone(),
            ScriptedCycleMapping::Function(_) => {
                panic!("Expected a mapping table to be set here")
            }
        };

        let mut event_iter =
            ScriptedCycleEmitter::new(cycle_userdata.cycle).with_mappings(mappings_table);
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

        let cycle_userdata = evaluate_cycle_userdata(
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
        let mappings_callback = match &cycle_userdata.mappings {
            ScriptedCycleMapping::Function(f) => f.clone(),
            ScriptedCycleMapping::Table(_) => panic!("Expected a mapping callback to be set here"),
        };

        let mut event_iter = ScriptedCycleEmitter::new(cycle_userdata.cycle)
            .with_mapping_callback(mappings_callback, &timeout_hook, &time_base)?;
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
