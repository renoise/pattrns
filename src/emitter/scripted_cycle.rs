use std::{cell::RefCell, collections::HashMap, rc::Rc};

use num_traits::ToPrimitive;

use mlua::prelude::{LuaError, LuaResult, LuaValue};

use crate::{
    bindings::{
        add_lua_callback_error, assign_cycle_vars_from_table, note_events_from_value,
        ContextPlaybackState, LuaCallback, LuaTimeoutHook,
    },
    emitter::cycle::{apply_cycle_note_properties, CycleNoteEvents},
    BeatTimeBase, Cycle, CycleEvent, CycleSubCycle, CycleValue, Emitter, EmitterEvent, Event,
    NoteEvent, Parameter, ParameterSet, RhythmEvent,
};

// -------------------------------------------------------------------------------------------------

/// Custom, user defined event mappings in cycles. Either a dynamic Lua function or static mapping table.
/// By default an empty table (no mappings applied).
#[derive(Debug)]
pub(crate) enum ScriptedCycleMapping<V> {
    Table(HashMap<String, V>),
    Function(LuaCallback),
}

impl<V> Clone for ScriptedCycleMapping<V>
where
    V: Clone,
{
    fn clone(&self) -> Self {
        match self {
            Self::Table(t) => Self::Table(t.clone()),
            Self::Function(f) => Self::Function(f.clone()),
        }
    }
}

impl<V> Default for ScriptedCycleMapping<V> {
    fn default() -> Self {
        Self::Table(HashMap::default())
    }
}

impl<V> ScriptedCycleMapping<V> {
    #[allow(unused)]
    pub fn map(&self) -> HashMap<String, V>
    where
        V: Clone,
    {
        match self {
            Self::Table(map) => map.clone(),
            Self::Function(_) => HashMap::new(),
        }
    }

    pub fn callback(&self) -> Option<&LuaCallback> {
        match self {
            Self::Function(f) => Some(f),
            Self::Table(_) => None,
        }
    }

    pub fn callback_mut(&mut self) -> Option<&mut LuaCallback> {
        match self {
            Self::Function(f) => Some(f),
            Self::Table(_) => None,
        }
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
    variable_callback: Option<LuaCallback>,
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
            variable_callback: None,
            timeout_hook: None,
            channel_steps: vec![],
        }
    }

    /// Return a new cycle with the given variable callback.
    pub(crate) fn with_variable_callback(
        self,
        variables_callback: LuaCallback,
        timeout_hook: &LuaTimeoutHook,
        time_base: &BeatTimeBase,
    ) -> LuaResult<Self> {
        // create a new timeout_hook instance and reset it before calling the function
        let mut timeout_hook = timeout_hook.clone();
        timeout_hook.reset();
        // initialize emitter context for the function
        let playback_state = ContextPlaybackState::Running;
        let iteration = 0;
        let mut variable_callback = variables_callback;
        variable_callback.set_cycle_var_context(
            playback_state,
            time_base,
            &self.parameters.iter().map(|(p, _)| Rc::clone(p)).collect(),
            iteration,
        )?;
        Ok(Self {
            variable_callback: Some(variable_callback),
            timeout_hook: Some(timeout_hook),
            ..self
        })
    }

    /// Return a new cycle with the given value mapping table.
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
        let parameters = vec![];
        // initialize emitter context for the function
        let playback_state = ContextPlaybackState::Running;
        let channel = 0;
        let step = 0;
        let step_length = 0.0;
        let mut mapping_callback = mapping_callback;
        mapping_callback.set_cycle_map_context(
            playback_state,
            time_base,
            channel,
            step,
            step_length,
        )?;
        let mappings = ScriptedCycleMapping::Function(mapping_callback);
        let channel_steps = vec![];
        Ok(Self {
            mappings,
            timeout_hook: Some(timeout_hook),
            parameters,
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
                    )?;
                    // call mapping function
                    let result = mapping_callback.call_with_arg(event.as_str().as_ref())?;
                    note_events_from_value(&result, None)?
                }
                ScriptedCycleMapping::Table(mappings) => {
                    if let Some(note_events) = mappings.get(event.as_str().as_ref()) {
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
            && self.mappings.callback().is_none()
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
        // inject parameter values into cycle as variables
        for (parameter_ref, enum_values) in &self.parameters {
            let parameter = parameter_ref.borrow();
            self.cycle
                .set_var(parameter.id(), parameter.into_var(enum_values));
        }

        // reset timeouts for mapping or variable callbacks
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }

        // run var callback
        if let Some(variables_callback) = &mut self.variable_callback {
            // update context
            if let Err(err) = variables_callback
                .set_context_playback_state(ContextPlaybackState::Running)
                .and(variables_callback.set_context_parameters(
                    &self.parameters.iter().map(|(p, _)| Rc::clone(p)).collect(),
                ))
                .and(variables_callback.set_context_cycle_iteration(self.cycle.iteration()))
            {
                variables_callback.handle_error(&err);
            }
            // run
            match variables_callback.call() {
                Err(err) => {
                    variables_callback.handle_error(&err);
                }
                Ok(value) => match value {
                    LuaValue::Table(table) => {
                        if let Err(err) = assign_cycle_vars_from_table(&mut self.cycle, table) {
                            add_lua_callback_error(None, None, "vars".to_string(), err);
                        }
                    }
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

        // set mapping callback playback state
        if let Some(callback) = self.mappings.callback_mut() {
            if let Err(err) = callback.set_context_playback_state(ContextPlaybackState::Running) {
                callback.handle_error(&err);
            }
        }

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
                match self.cycle_to_note_event(channel_index, channel_step, step_length, event) {
                    Err(err) => {
                        if let Some(callback) = self.mappings.callback() {
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
        let has_stateful_callbacks = self
            .variable_callback
            .as_ref()
            .is_some_and(|f| f.is_stateful().unwrap_or(true))
            || self
                .mappings
                .callback()
                .is_some_and(|f| f.is_stateful().unwrap_or(true));

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
        for (parameter_ref, enum_values) in &self.parameters {
            let parameter = parameter_ref.borrow();
            self.cycle
                .set_var(parameter.id(), parameter.into_var(enum_values));
        }

        // invoke variables callback
        if let Some(variables_callback) = &mut self.variable_callback {
            // update context
            if let Err(err) = variables_callback
                .set_context_playback_state(ContextPlaybackState::Running)
                .and(variables_callback.set_context_parameters(
                    &self.parameters.iter().map(|(p, _)| Rc::clone(p)).collect(),
                ))
                .and(variables_callback.set_context_cycle_iteration(self.cycle.iteration()))
            {
                variables_callback.handle_error(&err);
            }
            // run
            match variables_callback.call() {
                Err(err) => {
                    variables_callback.handle_error(&err);
                }
                Ok(value) => match value {
                    LuaValue::Table(table) => {
                        if let Err(err) = assign_cycle_vars_from_table(&mut self.cycle, table) {
                            add_lua_callback_error(None, None, "vars".to_string(), err);
                        }
                    }
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
                if let Err(err) =
                    mapping_callback.set_context_playback_state(ContextPlaybackState::Seeking)
                {
                    mapping_callback.handle_error(&err);
                }
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
                            if let Err(err) = mapping_callback.set_context_cycle_step(
                                channel_index,
                                channel_step,
                                step_length,
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
}

impl Emitter for ScriptedCycleEmitter {
    fn set_time_base(&mut self, time_base: &BeatTimeBase) {
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }
        if let Some(callback) = &mut self.variable_callback {
            if let Err(err) = callback.set_context_time_base(time_base) {
                callback.handle_error(&err);
            }
        }
        if let Some(callback) = self.mappings.callback_mut() {
            if let Err(err) = callback.set_context_time_base(time_base) {
                callback.handle_error(&err);
            }
        }
    }

    fn set_trigger_event(&mut self, event: &Event) {
        if let Some(timeout_hook) = &mut self.timeout_hook {
            timeout_hook.reset();
        }
        if let Some(callback) = &mut self.variable_callback {
            if let Err(err) = callback.set_context_trigger_event(event) {
                callback.handle_error(&err);
            }
        }
        if let Some(callback) = self.mappings.callback_mut() {
            if let Err(err) = callback.set_context_trigger_event(event) {
                callback.handle_error(&err);
            }
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
        // memorize parameters
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
        if let Some(callback) = &mut self.variable_callback {
            if let Err(err) = callback.set_context_parameters(&parameters) {
                callback.handle_error(&err);
            }
        }
        // pass parameters to the mapping callback context
        if let Some(callback) = self.mappings.callback_mut() {
            if let Err(err) = callback.set_context_parameters(&parameters) {
                callback.handle_error(&err);
            }
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
        if let Some(callback) = &mut self.variable_callback {
            // reset iteration counter
            let iteration = 0;
            if let Err(err) = callback.set_context_cycle_iteration(iteration) {
                callback.handle_error(&err);
            }
            // restore function
            if let Err(err) = callback.reset() {
                callback.handle_error(&err);
            }
        }
        if let Some(callback) = self.mappings.callback_mut() {
            // reset step counter
            let channel = 0;
            let step = 0;
            let step_length = 0.0;
            self.channel_steps.clear();
            if let Err(err) = callback.set_context_cycle_step(channel, step, step_length) {
                callback.handle_error(&err);
            }
            // restore function
            if let Err(err) = callback.reset() {
                callback.handle_error(&err);
            }
        }
    }
}
