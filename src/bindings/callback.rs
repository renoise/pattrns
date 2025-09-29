use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use mlua::prelude::*;

use lazy_static::lazy_static;
use std::sync::RwLock;

use crate::{BeatTimeBase, Event, Parameter, ParameterSet, RhythmEvent};

// -------------------------------------------------------------------------------------------------

lazy_static! {
    static ref LUA_CALLBACK_ERRORS: RwLock<Vec<LuaError>> = Vec::new().into();
}

/// Returns some error if there are any Lua callback errors, with the !first! error that happened.
/// Use `lua_callback_errors` to get fetch all errors since the errors got cleared.
///
/// ### Panics
/// Panics if accessing the global lua callback error vector fails.
pub fn has_lua_callback_errors() -> Option<LuaError> {
    LUA_CALLBACK_ERRORS
        .read()
        .expect("Failed to lock Lua callback error vector")
        .first()
        .cloned()
}

/// Returns all Lua callback errors, if any. Check with `has_lua_callback_errors()` to avoid
/// possible vec clone overhead, if that's relevant.
///
/// ### Panics
/// Panics if accessing the global lua callback error vector failed.
pub fn lua_callback_errors() -> Vec<LuaError> {
    LUA_CALLBACK_ERRORS
        .read()
        .expect("Failed to lock Lua callback error vector")
        .clone()
}

/// Clears all Lua callback errors.
///
/// ### Panics
/// Panics if accessing the global lua callback error vector failed.
pub fn clear_lua_callback_errors() {
    LUA_CALLBACK_ERRORS
        .write()
        .expect("Failed to lock Lua callback error vector")
        .clear();
}

/// Add/signal a new Lua callback errors.
///
/// ### Panics
/// Panics if accessing the global lua callback error vector failed.
pub fn add_lua_callback_error(name: &str, err: &LuaError) {
    log::warn!("Lua callback '{}' failed to evaluate:\n{}", name, err);
    LUA_CALLBACK_ERRORS
        .write()
        .expect("Failed to lock Lua callback error vector")
        .push(err.clone());
}

// -------------------------------------------------------------------------------------------------

/// Playback state in LuaCallback context.
pub(crate) enum ContextPlaybackState {
    Seeking,
    Running,
}

impl ContextPlaybackState {
    fn into_bytes_string(self) -> &'static [u8] {
        match self {
            Self::Seeking => b"seeking",
            Self::Running => b"running",
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// Lazily evaluates a lua function the first time it's called, to either use it as a iterator,
/// a function which returns a function, or directly as it is.
///
/// When calling the function the signature of the function is `fn(context): LuaResult`;
/// The passed context is created as an empty table with the callback, and should be filled up
/// with values before it's called.
///
/// Errors from callbacks should be handled by calling `self.handle_error` so external clients
/// can deal with them later, as appropriate.
///
/// By memorizing the original generator function and environment, it also can be reset to its
/// initial state by calling the original generator function again to fetch a new freshly
/// initialized function.
///
/// TODO: Upvalues of generators or simple functions could actually be collected and restored
/// too, but this uses debug functionality and may break some upvalues.
#[derive(Debug)]
pub(crate) struct LuaCallback {
    environment: Option<LuaTable>,
    context: LuaAnyUserData,
    generator: Option<LuaFunction>,
    function: LuaFunction,
    initialized: bool,
    #[allow(unused)]
    lua: Lua,
}

impl Clone for LuaCallback {
    fn clone(&self) -> Self {
        // reuse existing interpreter, function and environment refs, but create a new unique
        // context instance for every new callback clone, so new instances can have unique contexts
        let new_context = self
            .context
            .borrow::<CallbackContext>()
            .expect("Failed to borrow LuaCallback context data")
            .clone();
        let new_context_userdata = self
            .lua
            .create_userdata(new_context)
            .expect("Failed to create a new LuaCallback context userdata");
        Self {
            environment: self.environment.clone(),
            context: new_context_userdata,
            generator: self.generator.clone(),
            function: self.function.clone(),
            initialized: self.initialized,
            lua: self.lua.clone(),
        }
    }
}

impl LuaCallback {
    /// Create a new Callback from a lua function.
    pub fn new(lua: &Lua, function: LuaFunction) -> LuaResult<Self> {
        // create a strong lua ref, to ensure the function stays valid
        let lua = lua.clone();
        // create a new callback context
        let context = lua.create_userdata(CallbackContext::new())?;
        // and memorize the function without calling it
        let environment = function.environment();
        let generator = None;
        let initialized = false;
        Ok(Self {
            environment,
            context,
            generator,
            function,
            initialized,
            lua,
        })
    }

    /// Returns true if the callback is a generator.
    ///
    /// To test this, the callback must have run at least once, so it returns None if it never has.
    pub fn is_stateful(&self) -> Option<bool> {
        if self.initialized {
            Some(self.generator.is_some())
        } else {
            None
        }
    }

    /// Name of the inner function for errors. Usually will be an anonymous function.
    pub fn name(&self) -> String {
        self.function
            .info()
            .name
            .unwrap_or("anonymous function".to_string())
    }

    /// Sets the emitters playback state for the callback.
    pub fn set_context_playback_state(
        &mut self,
        playback_state: ContextPlaybackState,
    ) -> LuaResult<()> {
        let values = &mut self.context.borrow_mut::<CallbackContext>()?.values;
        values.insert(b"playback", playback_state.into_bytes_string().into());
        Ok(())
    }

    /// Sets the emitter time base context for the callback.
    pub fn set_context_time_base(&mut self, time_base: &BeatTimeBase) -> LuaResult<()> {
        let values = &mut self.context.borrow_mut::<CallbackContext>()?.values;
        values.insert(b"beats_per_min", time_base.beats_per_min.into());
        values.insert(b"beats_per_min", time_base.beats_per_min.into());
        values.insert(b"beats_per_bar", time_base.beats_per_bar.into());
        values.insert(b"samples_per_sec", time_base.samples_per_sec.into());
        Ok(())
    }

    /// Sets parameter context for the callback.
    pub fn set_context_parameters(&mut self, parameters: ParameterSet) -> LuaResult<()> {
        let inputs_context = &mut self.context.borrow_mut::<CallbackContext>()?.inputs_context;
        let mut parameters_map = HashMap::new();
        for parameter in &parameters {
            let parameter = Rc::clone(parameter);
            let parameter_id = parameter.borrow().id().as_bytes().to_vec();
            parameters_map.insert(parameter_id, parameter);
        }
        inputs_context.parameters_map = Rc::new(parameters_map);
        Ok(())
    }

    /// Sets the event which triggered the pattern for the callback context.
    pub fn set_context_trigger_event(&mut self, event: &Event) -> LuaResult<()> {
        let trigger_context = &mut self
            .context
            .borrow_mut::<CallbackContext>()?
            .trigger_context;
        trigger_context.event = Some(event.clone());
        Ok(())
    }

    /// Sets the pulse value emitter context for the callback.
    pub fn set_context_pulse_value(&mut self, pulse: RhythmEvent) -> LuaResult<()> {
        let values = &mut self.context.borrow_mut::<CallbackContext>()?.values;
        values.insert(b"pulse_value", pulse.value.into());
        values.insert(b"pulse_time", pulse.step_time.into());
        Ok(())
    }

    /// Sets the pulse step emitter context for the callback.
    pub fn set_context_pulse_step(
        &mut self,
        pulse_step: usize,
        pulse_time_step: f64,
    ) -> LuaResult<()> {
        let values = &mut self.context.borrow_mut::<CallbackContext>()?.values;
        values.insert(b"pulse_step", (pulse_step + 1).into());
        values.insert(b"pulse_time_step", pulse_time_step.into());
        Ok(())
    }

    /// Sets the step emitter context for the callback.
    pub fn set_context_step(&mut self, step: usize) -> LuaResult<()> {
        let values = &mut self.context.borrow_mut::<CallbackContext>()?.values;
        values.insert(b"step", (step + 1).into());
        Ok(())
    }

    /// Sets the cycle context step value for the callback.
    pub fn set_context_cycle_step(
        &mut self,
        channel: usize,
        step: usize,
        step_length: f64,
    ) -> LuaResult<()> {
        let values = &mut self.context.borrow_mut::<CallbackContext>()?.values;
        values.insert(b"channel", (channel + 1).into());
        values.insert(b"step", (step + 1).into());
        values.insert(b"step_length", step_length.into());
        Ok(())
    }

    /// Sets the rhythm context for the callback.
    pub fn set_rhythm_context(
        &mut self,
        time_base: &BeatTimeBase,
        pulse_step: usize,
        pulse_time_step: f64,
    ) -> LuaResult<()> {
        self.set_context_time_base(time_base)?;
        self.set_context_pulse_step(pulse_step, pulse_time_step)?;
        Ok(())
    }

    /// Sets the gate context for the callback.
    pub fn set_gate_context(
        &mut self,
        time_base: &BeatTimeBase,
        pulse: RhythmEvent,
        pulse_step: usize,
        pulse_time_step: f64,
    ) -> LuaResult<()> {
        self.set_rhythm_context(time_base, pulse_step, pulse_time_step)?;
        self.set_context_pulse_value(pulse)?;
        Ok(())
    }

    /// Sets the emitter context for the callback.
    pub fn set_emitter_context(
        &mut self,
        playback_state: ContextPlaybackState,
        time_base: &BeatTimeBase,
        pulse: RhythmEvent,
        pulse_step: usize,
        pulse_time_step: f64,
        step: usize,
    ) -> LuaResult<()> {
        self.set_context_playback_state(playback_state)?;
        self.set_gate_context(time_base, pulse, pulse_step, pulse_time_step)?;
        self.set_context_step(step)?;
        Ok(())
    }

    /// Sets the cycle context for the callback.
    pub fn set_cycle_context(
        &mut self,
        playback_state: ContextPlaybackState,
        time_base: &BeatTimeBase,
        channel: usize,
        step: usize,
        step_length: f64,
    ) -> LuaResult<()> {
        self.set_context_playback_state(playback_state)?;
        self.set_context_time_base(time_base)?;
        self.set_context_cycle_step(channel, step, step_length)?;
        Ok(())
    }

    /// Invoke the Lua function or generator and return its result as LuaValue.
    pub fn call(&mut self) -> LuaResult<LuaValue> {
        self.call_with_arg(LuaValue::Nil)
    }

    /// Invoke the Lua function or generator with an additional argument and return its result as LuaValue.
    pub fn call_with_arg<A: IntoLua + Clone>(&mut self, arg: A) -> LuaResult<LuaValue> {
        if self.initialized {
            self.function.call((&self.context, arg))
        } else {
            self.initialized = true;
            let result = self
                .function
                .call::<LuaValue>((&self.context, arg.clone()))?;
            if let Some(inner_function) = result.as_function().cloned() {
                // function returned a function -> is a generator. use the inner function instead.
                let environment = self.function.environment();
                self.environment = environment;
                self.generator = Some(std::mem::replace(&mut self.function, inner_function));
                self.function.call::<LuaValue>((&self.context, arg))
            } else {
                // function returned some value. use this function directly.
                self.environment = None;
                self.generator = None;
                Ok(result)
            }
        }
    }

    /// Report a Lua callback errors. The error will be logged and usually cleared after
    /// the next callback call.
    pub fn handle_error(&self, err: &LuaError) {
        add_lua_callback_error(&self.name(), err)
    }

    /// Reset the callback function or iterator to its initial state.
    pub fn reset(&mut self) -> LuaResult<()> {
        // resetting only is necessary when we got initialized
        if self.initialized {
            if let Some(function_generator) = &self.generator {
                // restore generator environment
                if let Some(env) = &self.environment {
                    function_generator.set_environment(env.clone())?;
                }
                // then fetch a new fresh function from the generator
                let value = function_generator.call::<LuaValue>(&self.context)?;
                if let Some(function) = value.as_function() {
                    self.function = function.clone();
                } else {
                    return Err(LuaError::runtime(format!(
                        "Failed to reset custom generator function '{}' \
                         Expected a function as return value, got a '{}'",
                        self.name(),
                        value.type_name()
                    )));
                }
            }
        }
        Ok(())
    }
}

// -------------------------------------------------------------------------------------------------

/// Memorizes an optional set of values that are passed along as context with the callback.
///
/// NB: CallbackTriggersContext and CallbackInputsContext are not LuaOwnedAnyUserData.
/// A userdata ref would cause reference cycles that would prevent destroying the Lua instance...
#[derive(Debug, Clone)]
struct CallbackContext {
    values: HashMap<&'static [u8], ContextValue>,
    trigger_context: CallbackTriggerContext,
    inputs_context: CallbackInputsContext,
}

impl CallbackContext {
    fn new() -> Self {
        Self {
            values: HashMap::new(),
            trigger_context: CallbackTriggerContext::new(),
            inputs_context: CallbackInputsContext::new(),
        }
    }
}

impl LuaUserData for CallbackContext {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field_with("__index", |lua| {
            lua.create_function(|lua, (this, key): (LuaUserDataRef<Self>, LuaString)| {
                // values (most likely, least overhead)
                if let Some(value) = this.values.get(key.as_bytes().as_ref()) {
                    value.into_lua(lua)
                }
                // parameter value table (also likely, small overhead)
                else if key == b"parameter" {
                    lua.create_userdata(this.inputs_context.clone())?
                        .into_lua(lua)
                }
                // trigger event values (also, medium overhead - creates copies)
                else if key == b"trigger" {
                    this.trigger_context.clone().into_lua(lua)
                } else {
                    Err(mlua::Error::RuntimeError(format!(
                        "undefined field '{}' in context",
                        key.to_string_lossy()
                    )))
                }
            })
        });
        fields.add_meta_field_with("__newindex", |lua| {
            lua.create_function(|_lua: &Lua, _: LuaValue| -> LuaResult<LuaFunction> {
                Err(mlua::Error::RuntimeError(
                    "context is read-only and thus can't be modified".to_string(),
                ))
            })
        });
    }
}

// -------------------------------------------------------------------------------------------------

/// Memorizes an optional set of input values within a CallbackContext, storing a reference to
/// a parameter map, so it's cheap to clone...
#[derive(Debug, Clone)]
struct CallbackInputsContext {
    parameters_map: Rc<HashMap<Vec<u8>, Rc<RefCell<Parameter>>>>,
}

impl CallbackInputsContext {
    fn new() -> Self {
        Self {
            parameters_map: Rc::new(HashMap::new()),
        }
    }
}

impl LuaUserData for CallbackInputsContext {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_meta_field_with("__index", |lua| {
            lua.create_function(
                |lua, (this, key): (mlua::UserDataRef<Self>, mlua::String)| {
                    if let Some(parameter) = this.parameters_map.get(key.as_bytes().as_ref()) {
                        Ok(parameter.borrow().lua_value(lua)?)
                    } else {
                        Err(mlua::Error::RuntimeError(format!(
                            "undefined parameter id '{}' in inputs context",
                            key.to_string_lossy()
                        )))
                    }
                },
            )
        });
        fields.add_meta_field_with("__newindex", |lua| {
            lua.create_function(|_lua: &Lua, _: LuaValue| -> LuaResult<LuaFunction> {
                Err(mlua::Error::RuntimeError(
                    "context inputs are read-only and thus can't be modified".to_string(),
                ))
            })
        });
    }
}

// -------------------------------------------------------------------------------------------------

/// Memorizes an optional trigger event value within a CallbackContext
#[derive(Debug, Clone)]
struct CallbackTriggerContext {
    event: Option<Event>,
}

impl CallbackTriggerContext {
    fn new() -> Self {
        Self { event: None }
    }
}

impl IntoLua for CallbackTriggerContext {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        if let Some(event) = self.event {
            event.into_lua(lua)
        } else {
            Ok(LuaValue::Nil)
        }
    }
}

// -------------------------------------------------------------------------------------------------

/// A to lua convertible value within a CallbackContext
#[derive(Debug, Copy, Clone, PartialEq)]
enum ContextValue {
    Number(LuaNumber),
    String(&'static [u8]),
}

impl IntoLua for &ContextValue {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        match *self {
            ContextValue::Number(num) => Ok(LuaValue::Number(num)),
            ContextValue::String(str) => Ok(LuaValue::String(lua.create_string(str)?)),
        }
    }
}

impl From<&'static [u8]> for ContextValue {
    fn from(val: &'static [u8]) -> Self {
        ContextValue::String(val)
    }
}

macro_rules! context_value_from_number_impl {
    ($type:ty) => {
        impl From<$type> for ContextValue {
            fn from(val: $type) -> Self {
                ContextValue::Number(val as LuaNumber)
            }
        }
    };
}

context_value_from_number_impl!(i32);
context_value_from_number_impl!(u32);
context_value_from_number_impl!(i64);
context_value_from_number_impl!(u64);
context_value_from_number_impl!(usize);
context_value_from_number_impl!(f32);
context_value_from_number_impl!(f64);

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use std::borrow::BorrowMut;

    use crate::{
        bindings::*,
        event::{Event, NoteEvent},
        note::Note,
        pattern::{
            beat_time::BeatTimePattern, second_time::SecondTimePattern, Pattern, PatternEvent,
        },
    };

    use pretty_assertions::assert_eq;

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
    fn callbacks() -> LuaResult<()> {
        let (lua, _) = new_test_engine(120.0, 4, 44100)?;

        let pattern = lua
            .load(
                r#"
                return pattern {
                    unit = "seconds",
                    pulse = function(context)
                      return (context.pulse_step == 2) and 0 or 1
                    end,
                    event = function(context)
                      local notes = {"c4", "d#4", "g4"}
                      local step = 1
                      return function(context)
                        local note = notes[step - 1 % #notes + 1]
                        step = step + 1
                        return note
                      end
                    end
                }
            "#,
            )
            .eval::<LuaValue>()?;

        let mut pattern = pattern
            .as_userdata()
            .unwrap()
            .borrow_mut::<SecondTimePattern>()?;
        let pattern = pattern.borrow_mut();
        for _ in 0..2 {
            let events = pattern.clone().take(4).collect::<Vec<_>>();
            pattern.reset();
            assert_eq!(
                events,
                vec![
                    PatternEvent {
                        event: Some(Event::NoteEvents(vec![Some((Note::C4).into())])),
                        time: 0,
                        duration: 44100
                    },
                    PatternEvent {
                        time: 44100,
                        event: None,
                        duration: 44100
                    },
                    PatternEvent {
                        time: 88200,
                        event: Some(Event::NoteEvents(vec![Some((Note::Ds4).into())])),
                        duration: 44100
                    },
                    PatternEvent {
                        time: 132300,
                        event: Some(Event::NoteEvents(vec![Some((Note::G4).into())])),
                        duration: 44100
                    }
                ]
            );
        }
        Ok(())
    }

    #[test]
    fn callback_clones() -> LuaResult<()> {
        let (lua, _) = new_test_engine(120.0, 4, 44100)?;

        // create a beat_time pattern which emits the context trigger notes
        let pattern = lua
            .load(
                r#"
                return pattern {
                    unit = "1/4",
                    event = function(context)
                      return context.trigger.notes
                    end
                }
            "#,
            )
            .eval::<LuaValue>()?;

        // create a pattern
        let mut pattern = pattern
            .as_userdata()
            .unwrap()
            .borrow_mut::<BeatTimePattern>()?;

        // create a pattern clone
        let pattern2 = pattern.duplicate();
        let mut pattern2 = (*pattern2).borrow_mut();

        // create and apply unique trigger events for both instances
        let trigger_event = Event::NoteEvents(vec![Some(NoteEvent {
            note: Note::A4,
            instrument: None,
            glide: Some(5.0),
            volume: 0.5,
            panning: 0.0,
            delay: 0.25,
        })]);
        pattern.set_trigger_event(&trigger_event);

        let trigger_event2 = Event::NoteEvents(vec![Some(NoteEvent {
            note: Note::C4,
            instrument: None,
            glide: None,
            volume: 1.0,
            panning: -1.0,
            delay: 0.5,
        })]);
        pattern2.set_trigger_event(&trigger_event2);

        // ensure clone and original pattern instance use different context values (triggers)
        let event = pattern.next();
        assert_eq!(
            event,
            Some(PatternEvent {
                time: 0,
                event: Some(trigger_event),
                duration: 22050,
            })
        );

        let event2 = pattern2.next();
        assert_eq!(
            event2,
            Some(PatternEvent {
                time: 0,
                event: Some(trigger_event2),
                duration: 22050,
            })
        );
        Ok(())
    }
}
