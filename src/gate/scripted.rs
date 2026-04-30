use mlua::prelude::LuaResult;

use crate::{
    bindings::{gate_trigger_from_value, LuaCallback, LuaTimeoutHook},
    BeatTimeBase, Event, Gate, ParameterSet, RhythmEvent,
};

// -------------------------------------------------------------------------------------------------

/// Evaluates a lua script function to generate new events.
#[derive(Debug)]
pub struct ScriptedGate {
    timeout_hook: LuaTimeoutHook,
    callback: LuaCallback,
    pulse_step: usize,
    pulse_time_step: f64,
}

impl ScriptedGate {
    pub(crate) fn new(
        timeout_hook: &LuaTimeoutHook,
        callback: LuaCallback,
        time_base: &BeatTimeBase,
    ) -> LuaResult<Self> {
        // create a new timeout_hook instance and reset it before calling the function
        let mut timeout_hook = timeout_hook.clone();
        timeout_hook.reset();
        // initialize function context
        let mut callback = callback;
        let pulse = RhythmEvent {
            value: 1.0,
            step_time: 1.0,
        };
        let pulse_step = 0;
        let pulse_time_step = 0.0;
        callback.set_gate_context(time_base, pulse, pulse_step, pulse_time_step)?;
        Ok(Self {
            timeout_hook,
            callback,
            pulse_step,
            pulse_time_step,
        })
    }

    fn next_gate_trigger_value(&mut self, pulse: &RhythmEvent) -> LuaResult<bool> {
        // reset timeout
        self.timeout_hook.reset();
        // update context
        self.callback.set_context_pulse_value(*pulse)?;
        self.callback
            .set_context_pulse_step(self.pulse_step, self.pulse_time_step)?;
        // invoke callback and evaluate the result
        gate_trigger_from_value(&self.callback.call()?)
    }
}

impl Clone for ScriptedGate {
    fn clone(&self) -> Self {
        Self {
            timeout_hook: self.timeout_hook.clone(),
            callback: self.callback.clone(),
            pulse_step: self.pulse_step,
            pulse_time_step: self.pulse_time_step,
        }
    }
}

impl Gate for ScriptedGate {
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        // reset timeout
        self.timeout_hook.reset();
        // update function context from the new time base
        self.callback.handle(|c| c.set_context_time_base(time_base));
    }

    fn set_trigger_event(&mut self, event: &Event) {
        // reset timeout
        self.timeout_hook.reset();
        // update function context from the new time base
        self.callback.handle(|c| c.set_context_trigger_event(event));
    }

    fn set_parameters(&mut self, parameters: ParameterSet) {
        // reset timeout
        self.timeout_hook.reset();
        // update function context with the new parameters
        self.callback
            .handle(|c| c.set_context_parameters(&parameters));
    }

    fn run(&mut self, pulse: &RhythmEvent) -> bool {
        // call function with context and evaluate the result
        let result = match self.next_gate_trigger_value(pulse) {
            Err(err) => {
                self.callback.handle_error(&err);
                false
            }
            Ok(value) => value,
        };
        // move step for the next iter call
        self.pulse_step += 1;
        self.pulse_time_step += pulse.step_time;
        // return function result
        result
    }

    fn duplicate(&self) -> Box<dyn Gate> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        // reset timeout
        self.timeout_hook.reset();
        // reset step counter
        self.pulse_step = 0;
        self.pulse_time_step = 0.0;
        // update step in context
        self.callback
            .handle(|c| c.set_context_pulse_step(self.pulse_step, self.pulse_time_step));
        // reset function
        self.callback.handle(|c| c.reset());
    }
}
