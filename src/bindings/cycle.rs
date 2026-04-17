use std::collections::HashMap;

use mlua::prelude::*;

use crate::{emitter::scripted_cycle::UserMapping, event::NoteEvent, tidal::Cycle};

use super::unwrap::{assign_cycle_vars_from_table, bad_argument_error, note_events_from_value};

// ---------------------------------------------------------------------------------------------

/// Cycle Userdata in bindings
#[derive(Clone, Debug)]
pub struct CycleUserData {
    pub cycle: Cycle,
    pub mapping: UserMapping<Vec<Option<NoteEvent>>, LuaFunction>,
    pub variables_function: Option<LuaFunction>,
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

        let variables_function = None;
        Ok(CycleUserData {
            cycle,
            mapping: UserMapping::Table(HashMap::new()),
            variables_function,
        })
    }
}

impl LuaUserData for CycleUserData {
    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method_mut("map", |_lua, this, value: LuaValue| match value {
            LuaValue::Function(func) => Ok(Self {
                mapping: UserMapping::Function(func),
                ..this.clone()
            }),
            LuaValue::Table(table) => {
                let mut mappings = HashMap::new();
                for (k, v) in table.pairs::<LuaValue, LuaValue>().flatten() {
                    mappings.insert(k.to_string()?, note_events_from_value(&v, None)?);
                }
                Ok(Self {
                    mapping: UserMapping::Table(mappings),
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

        methods.add_method_mut("var", |_lua, this, value: LuaValue| match value {
            LuaValue::Table(table) => {
                let mut cloned = this.clone();
                assign_cycle_vars_from_table(&mut cloned.cycle, table)?;
                Ok(cloned)
            }
            LuaValue::Function(func) => {
                this.cycle.clear_vars();
                Ok(Self {
                    variables_function: Some(func),
                    ..this.clone()
                })
            }
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
    use std::collections::HashMap;

    use super::*;

    use crate::{
        bindings::*,
        emitter::{cycle::CycleEmitter, scripted_cycle::ScriptedCycleEmitter},
        event::new_note,
        CycleSubCycle, Emitter, Event, Note, RhythmEvent,
    };

    use pretty_assertions::assert_eq;

    impl<V: Clone, F: Clone> UserMapping<V, F> {
        fn as_vec(&self) -> Vec<(String, V)> {
            match self.clone() {
                Self::Table(map) => map.iter().map(|(s, v)| (s.clone(), v.clone())).collect(),
                Self::Function(_) => vec![],
            }
        }
    }

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

        let mapped_cycle = evaluate_cycle_userdata(
            &lua,
            r#"cycle("a b c"):var({x = "c0", y = "b4:v0.5", z = "[a b c]"})"#,
        )?;

        assert_eq!(
            mapped_cycle.cycle.get_var("x").expect("x should exist"),
            CycleSubCycle::from("c0").expect("valid subcycle")
        );

        assert_eq!(
            mapped_cycle.cycle.get_var("y").expect("y should exist"),
            CycleSubCycle::from("b4:v0.5").expect("valid subcycle")
        );

        assert!(mapped_cycle.cycle.get_var("a").is_none());
        assert!(mapped_cycle.cycle.get_var("z").is_some());

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

        {
            let mapped_cycle = evaluate_cycle_userdata(
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
            let variables_callback =
                LuaCallback::new(&lua, mapped_cycle.variables_function.unwrap().clone())?;
            let mut event_iter = ScriptedCycleEmitter::new(mapped_cycle.cycle)
                .with_variables_callback(variables_callback, &timeout_hook)?;
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
        }

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
            mapped_cycle.mapping,
            UserMapping::Table(HashMap::from([
                ("a".to_string(), vec![new_note(Note::C0)]),
                ("b".to_string(), vec![new_note(Note::C4)]),
                ("c".to_string(), vec![new_note(Note::C6)]),
            ]))
        );

        // check if mappings are applied correctly
        let mut event_iter =
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mapping.as_vec());
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
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mapping.as_vec());
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
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mapping.as_vec());
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
            CycleEmitter::new(mapped_cycle.cycle).with_mappings(&mapped_cycle.mapping.as_vec());
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
        let mapping_callback = LuaCallback::new(
            &lua,
            match mapped_cycle.mapping {
                UserMapping::Table(_) => panic!("lua function for mapping was defined"),
                UserMapping::Function(f) => f.clone(),
            },
        )?;
        let mut event_iter = ScriptedCycleEmitter::new(mapped_cycle.cycle).with_mapping_callback(
            mapping_callback,
            &timeout_hook,
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
