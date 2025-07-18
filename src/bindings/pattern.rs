use std::{cell::RefCell, rc::Rc};

use mlua::prelude::*;

use crate::{
    bindings::{cycle::CycleUserData, unwrap::emitter_from_value, LuaTimeoutHook},
    event::InstrumentId,
    pattern::{beat_time::BeatTimePattern, second_time::SecondTimePattern, Pattern},
    BeatTimeBase,
};

// ---------------------------------------------------------------------------------------------

mod beat_time;
mod second_time;

// ---------------------------------------------------------------------------------------------

// unwrap a BeatTimePattern or SecondTimePattern from the given LuaValue,
// which is expected to be a user data
pub(crate) fn pattern_from_userdata(
    lua: &Lua,
    timeout_hook: &LuaTimeoutHook,
    value: &LuaValue,
    time_base: &BeatTimeBase,
    instrument: Option<InstrumentId>,
) -> LuaResult<Rc<RefCell<dyn Pattern>>> {
    if let Some(user_data) = value.as_userdata() {
        if user_data.is::<BeatTimePattern>() {
            // NB: take instead of cloning: pattern userdata has no other usage than being defined
            Ok(Rc::new(RefCell::new(
                user_data
                    .take::<BeatTimePattern>()?
                    .with_instrument(instrument),
            )))
        } else if user_data.is::<SecondTimePattern>() {
            Ok(Rc::new(RefCell::new(
                // NB: take instead of cloning: pattern userdata has no other usage than being defined
                user_data
                    .take::<SecondTimePattern>()?
                    .with_instrument(instrument),
            )))
        } else if user_data.is::<CycleUserData>() {
            // create a default pattern from the given cycle
            Ok(Rc::new(RefCell::new(
                BeatTimePattern::new(*time_base, crate::BeatTimeStep::Bar(1.0))
                    .with_instrument(instrument)
                    .trigger_dyn(emitter_from_value(lua, timeout_hook, value, time_base)?),
            )))
        } else {
            Err(LuaError::FromLuaConversionError {
                from: "userdata",
                to: "pattern".to_string(),
                message: Some(
                    "script must return a pattern or cycle, got some other userdata instead"
                        .to_string(),
                ),
            })
        }
    } else {
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "pattern".to_string(),
            message: Some("script must return a pattern or cycle".to_string()),
        })
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use crate::{
        bindings::*,
        event::{Event, NoteEvent},
        note::Note,
        pattern::{beat_time::BeatTimePattern, second_time::SecondTimePattern, PatternEvent},
        time::BeatTimeStep,
        RhythmEvent,
    };

    fn new_test_engine(
        beats_per_min: f32,
        beats_per_bar: u32,
        samples_per_sec: u32,
    ) -> Result<(Lua, LuaTimeoutHook), LuaError> {
        let (mut lua, mut timeout_hook) = new_engine()?;
        register_bindings(
            &mut lua,
            &timeout_hook,
            &BeatTimeBase {
                beats_per_min,
                beats_per_bar,
                samples_per_sec,
            },
        )?;
        timeout_hook.reset();
        Ok((lua, timeout_hook))
    }

    #[test]
    fn beat_time() -> LuaResult<()> {
        let (lua, _) = new_test_engine(120.0, 4, 44100)?;

        // BeatTimePattern
        let beat_time_pattern = lua
            .load(
                r#"
                pattern {
                    unit = "beats",
                    resolution = 0.5,
                    offset = "2",
                    pulse = {1,0,1,0},
                    event = "c6"
                }
            "#,
            )
            .eval::<LuaValue>()
            .unwrap();
        let beat_time_pattern = beat_time_pattern
            .as_userdata()
            .unwrap()
            .borrow_mut::<BeatTimePattern>();
        assert!(beat_time_pattern.is_ok());
        let mut beat_time_pattern = beat_time_pattern.unwrap();
        assert_eq!(beat_time_pattern.step(), BeatTimeStep::Beats(0.5));
        assert_eq!(beat_time_pattern.offset(), BeatTimeStep::Beats(1.0));
        let rhythm = beat_time_pattern.rhythm_mut();
        assert_eq!(
            vec![rhythm.run(), rhythm.run(), rhythm.run(), rhythm.run()],
            vec![
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                })
            ]
        );

        let event = beat_time_pattern.next();
        assert_eq!(
            event,
            Some(PatternEvent {
                time: 22050,
                event: Some(Event::NoteEvents(vec![Some(NoteEvent {
                    instrument: None,
                    note: Note::C6,
                    volume: 1.0,
                    panning: 0.0,
                    delay: 0.0
                })])),
                duration: 11025
            })
        );
        Ok(())
    }

    #[test]
    fn beat_time_callbacks() -> LuaResult<()> {
        let (lua, _) = new_test_engine(120.0, 4, 44100)?;

        let trigger_event = Event::NoteEvents(vec![Some(NoteEvent {
            note: Note::A4,
            instrument: None,
            volume: 0.5,
            panning: 0.0,
            delay: 0.25,
        })]);

        // BeatTimePattern function Context
        let beat_time_pattern = lua
            .load(
                r#"
                return pattern {
                    unit = "1/4",
                    pulse = function()
                      local pulse_step, pulse_time_step = 1, 0.0 
                      local function validate_context(context) 
                        assert(context.beats_per_min == 120)
                        assert(context.beats_per_bar == 4)
                        assert(context.samples_per_sec == 44100)
                        local trigger_notes = context.trigger.notes
                        assert(#trigger_notes == 1)
                        assert(trigger_notes[1].key == "A4")
                        assert(trigger_notes[1].volume == 0.5)
                        assert(trigger_notes[1].panning == 0.0)
                        assert(trigger_notes[1].delay == 0.25)
                        assert(context.pulse_step == pulse_step)
                        assert(context.pulse_time_step == pulse_time_step)
                      end
                      return function(context)
                        validate_context(context)
                        pulse_step = pulse_step + 2
                        pulse_time_step = pulse_time_step + 1.0
                        return {1, 0}
                      end
                    end,
                    gate = function(context) 
                      assert(context.beats_per_min == 120)
                      assert(context.beats_per_bar == 4)
                      assert(context.samples_per_sec == 44100)
                      local pulse_step, pulse_time_step = 1, 0.0 
                      local function validate_context(context) 
                        assert(context.beats_per_min == 120)
                        assert(context.beats_per_bar == 4)
                        assert(context.samples_per_sec == 44100)
                        assert(#context.trigger.notes == 1 and 
                          context.trigger.notes[1].key == "A4")
                        assert(context.pulse_step == pulse_step)
                        assert(context.pulse_time_step == pulse_time_step)
                      end
                      return function(context)
                        validate_context(context)
                        pulse_step = pulse_step + 2
                        pulse_time_step = pulse_time_step + 1
                        return true
                      end
                    end,
                    event = function(context)
                      assert(context.beats_per_min == 120)
                      assert(context.beats_per_bar == 4)
                      assert(context.samples_per_sec == 44100)
                      local pulse_step, pulse_time_step = 1, 0.0 
                      local step = 1 
                      local function validate_context(context) 
                        assert(context.playback == "running")
                        assert(context.beats_per_min == 120)
                        assert(context.beats_per_bar == 4)
                        assert(context.samples_per_sec == 44100)
                        assert(#context.trigger.notes == 1 and 
                          context.trigger.notes[1].key == "A4")
                        assert(context.pulse_step == pulse_step)
                        assert(context.pulse_time_step == pulse_time_step)
                        assert(context.step == step)
                      end
                      return function(context)
                        validate_context(context)
                        pulse_step = pulse_step + 2
                        pulse_time_step = pulse_time_step + 1
                        step = step + 1
                        return "c4"
                      end
                    end
                }
            "#,
            )
            .eval::<LuaValue>()
            .unwrap();

        let beat_time_pattern = beat_time_pattern
            .as_userdata()
            .unwrap()
            .borrow_mut::<BeatTimePattern>();
        assert!(beat_time_pattern.is_ok());
        let mut beat_time_pattern = beat_time_pattern.unwrap();

        beat_time_pattern.set_trigger_event(&trigger_event);

        let event = beat_time_pattern.next();
        assert_eq!(
            event,
            Some(PatternEvent {
                time: 0,
                event: Some(Event::NoteEvents(vec![Some(NoteEvent {
                    instrument: None,
                    note: Note::C4,
                    volume: 1.0,
                    panning: 0.0,
                    delay: 0.0
                })])),
                duration: 11025,
            })
        );

        assert!(beat_time_pattern.next().unwrap().event.is_none());
        for _ in 0..10 {
            assert!(beat_time_pattern.next().unwrap().event.is_some());
            assert!(beat_time_pattern.next().unwrap().event.is_none());
        }
        Ok(())
    }

    #[test]
    fn second_time() -> LuaResult<()> {
        let (lua, _) = new_test_engine(130.0, 8, 48000)?;

        // SecondTimePattern
        let second_time_pattern = lua
            .load(
                r#"
                pattern {
                    unit = "seconds",
                    resolution = 2,
                    offset = 3,
                    pulse = {1,0,1,0},
                    event = {"c5", "c5 v0.4", {"c7", "c7 v1.0"}}
                }
            "#,
            )
            .eval::<LuaValue>()
            .unwrap();

        let second_time_pattern = second_time_pattern
            .as_userdata()
            .unwrap()
            .borrow_mut::<SecondTimePattern>();
        assert!(second_time_pattern.is_ok());
        let mut second_time_pattern = second_time_pattern.unwrap();
        assert!((second_time_pattern.step() - 2.0).abs() < f64::EPSILON);
        assert!((second_time_pattern.offset() - 6.0).abs() < f64::EPSILON);
        let rhythm = second_time_pattern.rhythm_mut();
        assert_eq!(
            vec![rhythm.run(), rhythm.run(), rhythm.run(), rhythm.run()],
            vec![
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 1.0,
                    step_time: 1.0,
                }),
                Some(RhythmEvent {
                    value: 0.0,
                    step_time: 1.0,
                })
            ]
        );
        Ok(())
    }

    #[test]
    fn second_time_callbacks() -> LuaResult<()> {
        let (lua, _) = new_test_engine(130.0, 8, 48000)?;

        let trigger_event = Event::NoteEvents(vec![Some(NoteEvent {
            note: Note::C4,
            instrument: None,
            volume: 0.25,
            panning: 0.5,
            delay: 0.75,
        })]);

        // SecondTimePattern function Context
        let second_time_rhythm = lua
            .load(
                r#"
                return pattern {
                    unit = "ms",
                    pulse = function(context)
                      return 1
                    end,
                    gate = function(context)
                      return true
                    end,
                    event = function(context)
                      return "c4"
                    end
                }
            "#,
            )
            .eval::<LuaValue>()
            .unwrap();
        let second_time_pattern = second_time_rhythm
            .as_userdata()
            .unwrap()
            .borrow_mut::<SecondTimePattern>();
        assert!(second_time_pattern.is_ok());

        let mut second_time_pattern = second_time_pattern.unwrap();
        second_time_pattern.set_trigger_event(&trigger_event);

        let event = second_time_pattern.next();
        assert_eq!(
            event,
            Some(PatternEvent {
                time: 0,
                event: Some(Event::NoteEvents(vec![Some(NoteEvent {
                    instrument: None,
                    note: Note::C4,
                    volume: 1.0,
                    panning: 0.0,
                    delay: 0.0
                })],),),
                duration: 48
            })
        );
        Ok(())
    }
}
