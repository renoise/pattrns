use std::{cell::RefCell, collections::HashMap, rc::Rc};

use num_traits::ToPrimitive;

use mlua::prelude::{LuaError, LuaResult, LuaValue};

use crate::{
    bindings::{
        add_lua_callback_error, note_events_from_value, subcycle_values_from_table,
        ContextPlaybackState, LuaCallback, LuaTimeoutHook,
    },
    emitter::cycle::{apply_cycle_note_properties, CycleNoteEvents},
    BeatTimeBase, Cycle, CycleEvent, CycleSubCycle, CycleValue, Emitter, EmitterEvent, Event,
    NoteEvent, Parameter, ParameterSet, RhythmEvent,
};

// -------------------------------------------------------------------------------------------------

/// Custom, user defined event mappings or variables in cycles. Either a dynamic Lua callback
/// or static hashmap of values.
///
/// By default an empty table (no custom mappings applied).
#[derive(Debug, Clone)]
pub(crate) enum ScriptedCycleMapping<V: Clone> {
    Table(HashMap<String, V>),
    Function(LuaCallback),
}

impl<V: Clone> Default for ScriptedCycleMapping<V> {
    fn default() -> Self {
        Self::Table(HashMap::default())
    }
}

// -------------------------------------------------------------------------------------------------

/// Emits a vector of [`EmitterEvent`]s from a [`Cycle`].
///
/// Channels from cycle are merged down into note events on different voices.
/// Values in cycles can be mapped to notes with an optional mapping table or
/// callbacks from scripts.
///
/// See also [`CycleEmitter`](`super::cycle::CycleEmitter`)
#[derive(Clone, Debug)]
pub struct ScriptedCycleEmitter {
    cycle: Cycle,
    parameters: Vec<(Rc<RefCell<Parameter>>, Vec<CycleSubCycle>)>,
    mappings: ScriptedCycleMapping<Vec<Option<NoteEvent>>>,
    variables: ScriptedCycleMapping<CycleSubCycle>,
    timeout_hook: Option<LuaTimeoutHook>,
    channel_steps: Vec<usize>,
}

impl ScriptedCycleEmitter {
    /// Creates a new cycle emitter without mapping and variable overrides.
    pub(crate) fn new(cycle: Cycle) -> Self {
        Self {
            cycle,
            parameters: vec![],
            mappings: ScriptedCycleMapping::default(),
            variables: ScriptedCycleMapping::default(),
            timeout_hook: None,
            channel_steps: vec![],
        }
    }

    /// Return a new cycle with the given static variables table.
    pub(crate) fn with_variables(self, variables: HashMap<String, CycleSubCycle>) -> Self {
        // apply variables to cycle
        let mut cycle = self.cycle;
        for (name, subcycle) in variables.iter() {
            cycle.set_var(name, subcycle.clone());
        }
        // return a new emitter from the modified cycle and variables
        let variables = ScriptedCycleMapping::Table(variables);
        Self {
            cycle,
            variables,
            ..self
        }
    }

    /// Return a new cycle with the given variable callback.
    pub(crate) fn with_variables_callback(
        self,
        variables_callback: LuaCallback,
        timeout_hook: &LuaTimeoutHook,
        time_base: &BeatTimeBase,
    ) -> LuaResult<Self> {
        // create a new timeout_hook instance and reset it before calling the function
        let mut timeout_hook = timeout_hook.clone();
        timeout_hook.reset();
        let timeout_hook = Some(timeout_hook);
        // initialize emitter context for the function
        let playback_state = ContextPlaybackState::Running;
        let iteration = 0;
        let mut variables_callback = variables_callback;
        variables_callback.set_cycle_var_context(
            playback_state,
            time_base,
            &self.parameters.iter().map(|(p, _)| Rc::clone(p)).collect(),
            iteration,
        )?;
        // clear existing variables in cycle
        let mut cycle = self.cycle;
        cycle.clear_vars();
        // return a new emitter from the modified cycle and variables
        let variables = ScriptedCycleMapping::Function(variables_callback);
        Ok(Self {
            cycle,
            variables,
            timeout_hook,
            ..self
        })
    }

    /// Return a new cycle with the given static mapping table.
    pub(crate) fn with_mappings(self, mappings: HashMap<String, Vec<Option<NoteEvent>>>) -> Self {
        let mappings = ScriptedCycleMapping::Table(mappings);
        Self { mappings, ..self }
    }

    /// Return a new cycle with the given mapping callback.
    pub(crate) fn with_mapping_callback(
        self,
        mapping_callback: LuaCallback,
        timeout_hook: &LuaTimeoutHook,
        time_base: &BeatTimeBase,
    ) -> LuaResult<Self> {
        let mut timeout_hook = timeout_hook.clone();
        timeout_hook.reset();
        let timeout_hook = Some(timeout_hook);
        // initialize emitter context for the function
        let mut mapping_callback = mapping_callback;
        mapping_callback.init_cycle_map_context(time_base)?;
        let mappings = ScriptedCycleMapping::Function(mapping_callback);
        let channel_steps = vec![];
        Ok(Self {
            mappings,
            timeout_hook,
            channel_steps,
            ..self
        })
    }

    /// Generate a note event stack from a single cycle event, applying mappings if necessary.
    fn cycle_to_note_event(
        &mut self,
        channel_index: usize,
        channel_step: usize,
        step_length: f64,
        step_time: f64,
        event: CycleEvent,
    ) -> LuaResult<Vec<Option<NoteEvent>>> {
        let mut note_events = {
            match &mut self.mappings {
                ScriptedCycleMapping::Function(mapping_callback) => {
                    // update step in context
                    mapping_callback.set_context_cycle_step(
                        channel_index,
                        channel_step,
                        step_length,
                        step_time,
                    )?;
                    // call mapping function
                    let result = mapping_callback.call_with_arg(event.as_str().as_ref())?;
                    note_events_from_value(&result, None)?
                }
                ScriptedCycleMapping::Table(map) => {
                    if let Some(note_events) = map.get(event.as_str().as_ref()) {
                        // apply custom note mapping
                        note_events.clone()
                    } else {
                        // try converting the cycle value to a single note
                        event.value().try_into().map_err(LuaError::RuntimeError)?
                    }
                }
            }
        };
        // verify that all identifiers are mapped
        if (note_events.is_empty() || note_events.iter().all(|f| f.is_none()))
            && !matches!(self.mappings, ScriptedCycleMapping::Function(_))
            && !matches!(event.value(), CycleValue::Rest | CycleValue::Hold)
        {
            return Err(LuaError::runtime(format!(
                "invalid/unknown identifier in cycle: '{}'. please check for typos or add a custom mapping for it.",
                event.as_str()
            )));
        }
        // apply note properties from targets
        apply_cycle_note_properties(&mut note_events, event.targets())
            .map_err(|err| LuaError::RuntimeError(err.to_string()))?;

        Ok(note_events)
    }

    /// Generate next batch of events from the next cycle run.
    /// Converts cycle events to note events and flattens channels into note columns.
    fn generate(&mut self) -> Vec<EmitterEvent> {
        // reset timeouts for mapping or variable callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }

        // inject parameter values into cycle as variables
        self.apply_parameter_variables();

        // inject var callback values into cycle, if present
        self.apply_variables_callback();

        // set mapping callback playback state
        if let ScriptedCycleMapping::Function(callback) = &mut self.mappings {
            callback.handle(|c| {
                c.set_context_playback_state(ContextPlaybackState::Running)
                    .and(c.set_context_cycle_iteration(self.cycle.iteration()))
            });
        }
        // run the cycle event generator
        let events = {
            match self.cycle.generate() {
                Ok(events) => events,
                Err(err) => {
                    let source = self.cycle.source().clone();
                    let source_line = None;
                    add_lua_callback_error(
                        source,
                        source_line,
                        "generate".to_string(),
                        LuaError::RuntimeError(err),
                    );
                    // skip processing events
                    return vec![];
                }
            }
        };

        // convert possibly mapped cycle channel items to a list of note events
        let mut timed_note_events = CycleNoteEvents::new();
        for (channel_index, channel_events) in events.into_iter().enumerate() {
            if self.channel_steps.len() <= channel_index {
                self.channel_steps.resize(channel_index + 1, 0);
            }
            for event in channel_events.into_iter() {
                // increase step counter
                let channel_step = self.channel_steps[channel_index];
                self.channel_steps[channel_index] += 1;
                // convert cycle to note event
                let start = event.span().start();
                let length = event.span().length();
                let step_length = length.to_f64().unwrap_or(0.0);
                let step_time = start.to_f64().unwrap_or(0.0);
                match self.cycle_to_note_event(
                    channel_index,
                    channel_step,
                    step_length,
                    step_time,
                    event,
                ) {
                    Err(err) => {
                        if let ScriptedCycleMapping::Function(callback) = &self.mappings {
                            callback.handle_error(&err)
                        } else {
                            let source = self.cycle.source().clone();
                            let source_line = None;
                            add_lua_callback_error(source, source_line, "map".to_string(), err);
                        }
                    }
                    Ok(note_events) => {
                        if !note_events.is_empty() {
                            timed_note_events.add(channel_index, start, length, note_events);
                        }
                    }
                }
            }
        }

        // convert timed note events into EmitterEvents
        timed_note_events.into_event_iter_items()
    }

    /// Skip next batch of events from the cycle.
    /// This maintains cycle mapping callback states as well, if needed.
    fn advance(&mut self) {
        let has_stateful_callbacks = match &self.variables {
            ScriptedCycleMapping::Function(f) => f.is_stateful().unwrap_or(true),
            ScriptedCycleMapping::Table(_) => false,
        } || match &self.mappings {
            ScriptedCycleMapping::Function(f) => f.is_stateful().unwrap_or(true),
            ScriptedCycleMapping::Table(_) => false,
        };

        if !has_stateful_callbacks {
            // can simply advance the cycle
            self.cycle.advance();
            self.channel_steps.clear();
            return;
        }

        // reset timeouts for mapping or variable callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }

        // inject parameter values into cycle as variables
        self.apply_parameter_variables();

        // inject var callback values into cycle, if present
        self.apply_variables_callback();

        // run the cycle event generator
        let events = {
            match self.cycle.generate() {
                Ok(events) => events,
                Err(err) => {
                    let source = self.cycle.source().clone();
                    let source_line = None;
                    add_lua_callback_error(
                        source,
                        source_line,
                        "advance".to_string(),
                        LuaError::RuntimeError(err),
                    );
                    return;
                }
            }
        };

        // dry-run mappings
        match &mut self.mappings {
            ScriptedCycleMapping::Function(mapping_callback) => {
                // set playback state
                mapping_callback
                    .handle(|c| c.set_context_playback_state(ContextPlaybackState::Seeking));

                if mapping_callback.is_stateful().unwrap_or(true) {
                    // run stateful callbacks but ignore results
                    for (channel_index, channel_events) in events.into_iter().enumerate() {
                        if self.channel_steps.len() <= channel_index {
                            self.channel_steps.resize(channel_index + 1, 0);
                        }
                        for event in channel_events.into_iter() {
                            // move step counter
                            let channel_step = self.channel_steps[channel_index];
                            self.channel_steps[channel_index] += 1;
                            // update step in context
                            let step_length = event.span().length().to_f64().unwrap_or(0.0);
                            let step_time = event.span().start().to_f64().unwrap_or(0.0);
                            if let Err(err) = mapping_callback.set_context_cycle_step(
                                channel_index,
                                channel_step,
                                step_length,
                                step_time,
                            ) {
                                mapping_callback.handle_error(&err);
                                return;
                            }
                            // call mapping function
                            if let Err(err) =
                                mapping_callback.call_with_arg(event.as_str().as_ref())
                            {
                                mapping_callback.handle_error(&err);
                                return;
                            }
                        }
                    }
                } else {
                    // advance channel_steps for generated events
                    for (channel_index, channel_events) in events.into_iter().enumerate() {
                        if self.channel_steps.len() <= channel_index {
                            self.channel_steps.resize(channel_index + 1, 0);
                        }
                        self.channel_steps[channel_index] += channel_events.len();
                    }
                }
            }
            ScriptedCycleMapping::Table(_) => {
                // advance channel_steps for generated events
                for (channel_index, channel_events) in events.into_iter().enumerate() {
                    if self.channel_steps.len() <= channel_index {
                        self.channel_steps.resize(channel_index + 1, 0);
                    }
                    self.channel_steps[channel_index] += channel_events.len();
                }
            }
        }
    }

    fn apply_parameter_variables(&mut self) {
        if let ScriptedCycleMapping::Table(map) = &self.variables {
            for (parameter_ref, enum_values) in &self.parameters {
                let parameter = parameter_ref.borrow();
                // var overrides parameter variables, so skip this parameter
                if !map.contains_key(parameter.id()) {
                    self.cycle
                        .set_var(parameter.id(), parameter.into_var(enum_values));
                }
            }
        } else {
            for (parameter_ref, enum_values) in &self.parameters {
                let parameter = parameter_ref.borrow();
                self.cycle
                    .set_var(parameter.id(), parameter.into_var(enum_values));
            }
        }
    }

    fn apply_variables_callback(&mut self) {
        if let ScriptedCycleMapping::Function(callback) = &mut self.variables {
            // update context
            callback.handle(|c| {
                c.set_context_playback_state(ContextPlaybackState::Running)
                    .and(c.set_context_parameters(
                        &self.parameters.iter().map(|(p, _)| Rc::clone(p)).collect(),
                    ))
                    .and(c.set_context_cycle_iteration(self.cycle.iteration()))
            });
            // run
            match callback.call() {
                Err(err) => {
                    callback.handle_error(&err);
                }
                Ok(value) => match value {
                    LuaValue::Table(table) => match subcycle_values_from_table(table) {
                        Ok(variables) => {
                            for (k, v) in variables.into_iter() {
                                self.cycle.set_var(&k, v);
                            }
                        }
                        Err(err) => {
                            add_lua_callback_error(None, None, "vars".to_string(), err);
                        }
                    },
                    _ => {
                        add_lua_callback_error(
                            None,
                            None,
                            "vars".to_string(),
                            LuaError::RuntimeError(
                                "vars should return a table of variables".to_string(),
                            ),
                        );
                    }
                },
            }
        }
    }
}

impl Emitter for ScriptedCycleEmitter {
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        // reset timeouts for callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }
        // pass time base to callbacks
        if let ScriptedCycleMapping::Function(callback) = &mut self.variables {
            callback.handle(|c| c.set_context_time_base(time_base));
        }
        if let ScriptedCycleMapping::Function(callback) = &mut self.mappings {
            callback.handle(|c| c.set_context_time_base(time_base));
        }
    }

    fn set_trigger_event(&mut self, event: &Event) {
        // reset timeouts for callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }
        // pass event to callbacks
        if let ScriptedCycleMapping::Function(callback) = &mut self.variables {
            callback.handle(|c| c.set_context_trigger_event(event));
        }
        if let ScriptedCycleMapping::Function(callback) = &mut self.mappings {
            callback.handle(|c| c.set_context_trigger_event(event));
        }
    }

    fn set_parameters(&mut self, parameters: ParameterSet) {
        // reset timeouts for callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }

        // parse and unwrap cycle subcycle values from enum parameters
        let unwrap_sub_cycle_result = |sub_cycle: Result<CycleSubCycle, String>| -> CycleSubCycle {
            sub_cycle.unwrap_or_else(|err| {
                // forwarding parse error as runtime errors
                add_lua_callback_error(
                    None,
                    None,
                    "cycle".to_string(),
                    LuaError::RuntimeError(err),
                );
                // return rest value to prevent further runtime errors, which would mask the original error
                CycleSubCycle::rest()
            })
        };
        self.parameters = parameters
            .iter()
            .map(|parameter| {
                (
                    Rc::clone(parameter),
                    parameter
                        .borrow()
                        .parse_subcycles()
                        .into_iter()
                        .map(unwrap_sub_cycle_result)
                        .collect(),
                )
            })
            .collect();

        // pass parameters to the variables callback context
        if let ScriptedCycleMapping::Function(callback) = &mut self.variables {
            callback.handle(|c| c.set_context_parameters(&parameters));
        }

        // pass parameters to the mapping callback context
        if let ScriptedCycleMapping::Function(callback) = &mut self.mappings {
            callback.handle(|c| c.set_context_parameters(&parameters));
        }
    }

    fn run(&mut self, _pulse: RhythmEvent, emit_event: bool) -> Option<Vec<EmitterEvent>> {
        if emit_event {
            Some(self.generate())
        } else {
            None
        }
    }

    fn advance(&mut self, _pulse: RhythmEvent, emit_event: bool) {
        if emit_event {
            self.advance();
        }
    }

    fn duplicate(&self) -> Box<dyn Emitter> {
        Box::new(self.clone())
    }

    fn reset(&mut self) {
        // reset cycle
        self.cycle.reset();

        // reset timeouts for callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }

        // reset callbacks
        if let ScriptedCycleMapping::Function(callback) = &mut self.variables {
            // reset iteration counter
            let iteration = 0;
            callback.handle(|c| c.set_context_cycle_iteration(iteration));
            // restore function
            callback.handle(|c| c.reset());
        }
        if let ScriptedCycleMapping::Function(callback) = &mut self.mappings {
            // reset step counter
            let channel = 0;
            let step = 0;
            let step_length = 0.0;
            let step_time = 0.0;
            self.channel_steps.clear();
            callback.handle(|c| c.set_context_cycle_step(channel, step, step_length, step_time));
            // restore function
            callback.handle(|c| c.reset());
        }
    }
}
