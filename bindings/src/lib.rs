#![allow(clippy::missing_safety_doc)]

extern crate alloc;

use std::{
    cell::RefCell,
    ffi::{c_char, c_void, CStr, CString},
    mem::ManuallyDrop,
    panic, ptr,
    rc::Rc,
};

use panic_message::panic_message;
use static_assertions::const_assert_eq;

mod pattrns {
    // wrap pattrns types into a pattrns:: namespace
    pub(super) use pattrns::prelude::*;
}

// -------------------------------------------------------------------------------------------------

mod allocator;

// -------------------------------------------------------------------------------------------------

// helper function to create a new raw CString from strings which may contain inner \0 chars.
unsafe fn new_raw_cstring(str: &str) -> *mut c_char {
    if str.contains('\0') {
        CString::from_vec_unchecked(str.replace('\0', "\\0").into()).into_raw()
    } else {
        CString::from_vec_unchecked(str.into()).into_raw()
    }
}

// helper function to drop a string created with `new_raw_cstring`
unsafe fn drop_raw_cstring(chars: *const c_char) {
    if !chars.is_null() {
        drop(CString::from_raw(chars as *mut c_char))
    }
}

/// Helper macro to handle errors in pattrns functions: Deals with panics and lua callback errors.
///
/// On panic, the panic message is returned as result error. When the block did not panic, it
/// checks for lua callback errors and then returns them as result errors.
/// If no callback errors happened, it returns the block's return value as it is, which again can
/// be an error or not.
///
/// The result type we're using here in the bindings are unfortunately not a Result template,
/// but a custom error to get them bound to C++, so the blocks Result type must be passed as
/// first argument to be macro.
macro_rules! try_catch {
    ($result_type:ident, $block:block) => {{
        match panic::catch_unwind(panic::AssertUnwindSafe(|| {
            // clear previous lua callback errors, if any
            pattrns::clear_lua_callback_errors();
            // evaluate block
            let result = $block;
            // when the block caused a callback error, return the error
            if let Some(lua_error) = pattrns::has_lua_callback_errors() {
                $result_type::Error(new_raw_cstring(&lua_error.to_string()))
            } else {
                // else return the block's return value
                result
            }
        })) {
            Ok(value) => value,
            Err(payload) => $result_type::Error(new_raw_cstring(&format!(
                "Ouch. Internal error, please report: {}",
                panic_message(&payload)
            ))),
        }
    }};
}

// -------------------------------------------------------------------------------------------------

/// Instrument_id value which refers to an unset, undefined id.
pub const NO_INSTRUMENT_ID: u32 = u32::MAX;
/// Parameter change value which refers to an empty, undefined parameter.
pub const NO_PARAMETER_ID: u32 = u32::MAX;

/// Glide value which refers to an unset, undefined glide value.
pub const NO_GLIDE_VALUE: f32 = -1.0;

/// Note value which refers to an empty, undefined note.
pub const EMPTY_NOTE: u8 = 0xFE;
const_assert_eq!(EMPTY_NOTE, pattrns::Note::EMPTY as u8);
/// Note value which should turn off notes playing on the same column
pub const NOTE_OFF: u8 = 0xFF;
const_assert_eq!(NOTE_OFF, pattrns::Note::OFF as u8);

// -------------------------------------------------------------------------------------------------

/// C lang compatible representation of a rust `Result<f64>`.
/// Error strings must be released manually with `drop_error_string`.
#[repr(C)]
pub enum F64Result {
    Error(*const c_char),
    Value(f64),
}

/// C lang compatible representation of a rust `Result<u32>`.
/// Error strings must be released manually with `drop_error_string`.
#[repr(C)]
pub enum UInt32Result {
    Error(*const c_char),
    Value(u32),
}

/// C lang compatible representation of a rust `Result<()>`.
/// Error strings must be released manually with `drop_error_string`.
#[repr(C)]
pub enum VoidResult {
    Error(*const c_char),
    Ok(()),
}

/// Delete an error string from the Result wrappers.
#[no_mangle]
pub unsafe extern "C" fn drop_error_string(error: *const c_char) {
    drop_raw_cstring(error)
}

// -------------------------------------------------------------------------------------------------

/// C lang compatible representation of a rust `pattrns::NoteEvent`.
#[repr(C)]
pub struct NoteEvent {
    pub note: u8,
    pub instrument: u32,
    pub glide: f32,
    pub volume: f32,
    pub panning: f32,
    pub delay: f32,
}

impl Default for NoteEvent {
    // create a new empty, note event
    fn default() -> Self {
        Self {
            note: EMPTY_NOTE,
            instrument: NO_INSTRUMENT_ID,
            glide: NO_GLIDE_VALUE,
            volume: 1.0,
            panning: 0.0,
            delay: 0.0,
        }
    }
}

impl From<&pattrns::NoteEvent> for NoteEvent {
    fn from(value: &pattrns::NoteEvent) -> Self {
        let note = value.note as u8;
        let instrument = value
            .instrument
            .map_or(NO_INSTRUMENT_ID, |id| usize::from(id) as u32);
        let glide = value.glide.map_or(NO_GLIDE_VALUE, |value| value);
        let volume = value.volume;
        let panning = value.panning;
        let delay = value.delay;
        Self {
            note,
            instrument,
            glide,
            volume,
            panning,
            delay,
        }
    }
}

impl From<&NoteEvent> for pattrns::NoteEvent {
    fn from(value: &NoteEvent) -> Self {
        pattrns::NoteEvent {
            note: pattrns::Note::from(value.note),
            instrument: match value.instrument {
                NO_INSTRUMENT_ID => None,
                _ => Some(pattrns::InstrumentId::from(value.instrument as usize)),
            },
            glide: match value.glide {
                NO_GLIDE_VALUE => None,
                _ => Some(value.glide),
            },
            volume: value.volume,
            panning: value.panning,
            delay: value.delay,
        }
    }
}

/// C lang compatible representation of a rust `Vec<pattrns::NoteEvent>`.
#[repr(C)]
pub struct NoteEvents {
    pub events_ptr: *const NoteEvent,
    pub events_len: u32,
}

impl Default for NoteEvents {
    fn default() -> Self {
        Self {
            events_ptr: ptr::null(),
            events_len: 0,
        }
    }
}

impl From<&[Option<pattrns::NoteEvent>]> for NoteEvents {
    fn from(note_events: &[Option<pattrns::NoteEvent>]) -> Self {
        // create a raw vector of Note Events and prevent the temp vector from
        // being destroyed. we'll do so when dropping Self.
        let mut note_events_vector = ManuallyDrop::new(
            note_events
                .iter()
                .map(|note_event| match note_event {
                    Some(event) => NoteEvent::from(event),
                    None => NoteEvent::default(),
                })
                .collect::<Vec<_>>(),
        );
        note_events_vector.shrink_to_fit(); // make capacity = len
        let events_ptr = note_events_vector.as_ptr();
        let events_len = note_events_vector.len() as u32;
        Self {
            events_ptr,
            events_len,
        }
    }
}

impl Drop for NoteEvents {
    fn drop(&mut self) {
        if !self.events_ptr.is_null() {
            unsafe {
                drop(Vec::from_raw_parts(
                    self.events_ptr.cast_mut(),
                    self.events_len as usize,
                    self.events_len as usize,
                ));
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[repr(C)]
/// C lang compatible representation of a rust `pattrns::ParameterChangeEvent`.
pub struct ParameterChangeEvent {
    pub parameter: u32,
    pub value: f32,
}

impl From<&pattrns::ParameterChangeEvent> for ParameterChangeEvent {
    fn from(value: &pattrns::ParameterChangeEvent) -> Self {
        let parameter = value
            .parameter
            .map_or(NO_PARAMETER_ID, |id| usize::from(id) as u32);
        let value = value.value;
        Self { parameter, value }
    }
}

#[repr(C)]
/// C lang compatible representation of a rust `Vec<pattrns::ParameterChangeEvent>`.
pub struct ParameterChangeEvents {
    pub events_ptr: *const ParameterChangeEvent,
    pub events_len: u32,
}

impl Default for ParameterChangeEvents {
    fn default() -> Self {
        Self {
            events_ptr: ptr::null(),
            events_len: 0,
        }
    }
}

impl From<&[pattrns::ParameterChangeEvent]> for ParameterChangeEvents {
    fn from(parameter_change_events: &[pattrns::ParameterChangeEvent]) -> Self {
        // create a raw vector of Parameter Change Events and prevent the temp vector
        // from being destroyed. we'll do so when dropping Self.
        let mut parameter_change_events_vector = ManuallyDrop::new(
            parameter_change_events
                .iter()
                .map(ParameterChangeEvent::from)
                .collect::<Vec<_>>(),
        );
        parameter_change_events_vector.shrink_to_fit(); // make capacity = len
        let events_ptr = parameter_change_events_vector.as_ptr();
        let events_len = parameter_change_events_vector.len() as u32;
        Self {
            events_ptr,
            events_len,
        }
    }
}

impl Drop for ParameterChangeEvents {
    fn drop(&mut self) {
        if !self.events_ptr.is_null() {
            unsafe {
                drop(Vec::from_raw_parts(
                    self.events_ptr.cast_mut(),
                    self.events_len as usize,
                    self.events_len as usize,
                ));
            }
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[repr(C)]
/// C lang compatible representation of a rust `pattrns::BeatTimeBase`.
pub struct Timebase {
    pub bpm: f32,
    pub bpb: u32,
    pub sample_rate: u32,
}

impl From<Timebase> for pattrns::BeatTimeBase {
    fn from(value: Timebase) -> Self {
        pattrns::BeatTimeBase {
            beats_per_min: value.bpm,
            beats_per_bar: value.bpb,
            samples_per_sec: value.sample_rate,
        }
    }
}

// -------------------------------------------------------------------------------------------------

#[repr(C)]
/// C lang compatible representation of a rust `pattrns::ParameterType`.
pub enum ParameterType {
    Boolean,
    Integer,
    Float,
    Enum,
}

impl From<pattrns::ParameterType> for ParameterType {
    fn from(value: pattrns::ParameterType) -> Self {
        match value {
            pattrns::ParameterType::Boolean => ParameterType::Boolean,
            pattrns::ParameterType::Integer => ParameterType::Integer,
            pattrns::ParameterType::Float => ParameterType::Float,
            pattrns::ParameterType::Enum => ParameterType::Enum,
        }
    }
}

/// C lang compatible representation of a rust `Vec<String>` using a C Array.
#[repr(C)]
pub struct ValueStrings {
    pub strings_ptr: *const *const c_char,
    pub strings_len: u32,
}

impl Drop for ValueStrings {
    fn drop(&mut self) {
        if !self.strings_ptr.is_null() {
            unsafe {
                let mut vec = Vec::from_raw_parts(
                    self.strings_ptr.cast_mut(),
                    self.strings_len as usize,
                    self.strings_len as usize,
                );
                for str in &mut vec {
                    drop_raw_cstring(*str);
                }
                drop(vec);
            }
        }
    }
}

impl From<&[String]> for ValueStrings {
    fn from(strings: &[String]) -> Self {
        unsafe {
            // create a raw vector of raw strings and prevent the temp vector from
            // being destroyed. we'll do so when dropping Self.
            let mut strings_vector = ManuallyDrop::new(
                strings
                    .iter()
                    .map(|p| new_raw_cstring(p) as *const c_char)
                    .collect::<Vec<_>>(),
            );
            strings_vector.shrink_to_fit(); // make capacity = len
            let strings_ptr = strings_vector.as_ptr();
            let strings_len = strings_vector.len() as u32;
            Self {
                strings_ptr,
                strings_len,
            }
        }
    }
}

#[repr(C)]
/// C lang compatible representation of a rust `pattrns::Parameter`.
/// Ensure strings are not used after the parameters array got dropped.
pub struct Parameter {
    pub id: *const c_char,
    pub name: *const c_char,
    pub description: *const c_char,
    pub parameter_type: ParameterType,
    pub min: f64,
    pub max: f64,
    pub default: f64,
    pub value: f64,
    pub value_strings: ValueStrings,
}

impl From<&Rc<RefCell<pattrns::Parameter>>> for Parameter {
    fn from(value: &Rc<RefCell<pattrns::Parameter>>) -> Self {
        unsafe {
            let value = value.borrow();
            Parameter {
                id: new_raw_cstring(value.id()),
                name: new_raw_cstring(value.name()),
                description: new_raw_cstring(value.description()),
                parameter_type: value.parameter_type().into(),
                min: *value.range().start(),
                max: *value.range().end(),
                value: value.value(),
                default: value.default(),
                value_strings: value.value_strings().into(),
            }
        }
    }
}

impl Drop for Parameter {
    fn drop(&mut self) {
        unsafe {
            drop_raw_cstring(self.id);
            drop_raw_cstring(self.name);
            drop_raw_cstring(self.description);
        }
    }
}

/// C lang compatible representation of a rust `ParameterMap` using a C Array.
#[repr(C)]
pub struct ParameterSet {
    pub parameters_ptr: *const Parameter,
    pub parameters_len: u32,
}

impl From<&[Rc<RefCell<pattrns::Parameter>>]> for ParameterSet {
    fn from(parameters: &[Rc<RefCell<pattrns::Parameter>>]) -> Self {
        // create a raw vector of Note Events and prevent the temp vector from
        // being destroyed. we'll do so when dropping Self.
        let mut parameters_vector =
            ManuallyDrop::new(parameters.iter().map(|p| p.into()).collect::<Vec<_>>());
        parameters_vector.shrink_to_fit(); // make capacity = len
        let parameters_ptr = parameters_vector.as_ptr();
        let parameters_len = parameters_vector.len() as u32;
        Self {
            parameters_ptr,
            parameters_len,
        }
    }
}

impl Drop for ParameterSet {
    fn drop(&mut self) {
        if !self.parameters_ptr.is_null() {
            unsafe {
                drop(Vec::from_raw_parts(
                    self.parameters_ptr.cast_mut(),
                    self.parameters_len as usize,
                    self.parameters_len as usize,
                ));
            }
        }
    }
}

/// C lang compatible representation of a rust `Result<ParameterSet>`.
/// Error strings must be released manually with `drop_error_string`.
/// Values must be released manually with `drop_parameter_set`.
#[repr(C)]
pub enum ParameterSetResult {
    Error(*const c_char),
    Value(*mut ParameterSet),
}

#[no_mangle]
/// Drop array of input parameters, created via `pattern_parameters`
pub unsafe extern "C" fn drop_parameter_set(parameters: *mut ParameterSet) {
    if !parameters.is_null() {
        drop(Box::from_raw(parameters));
    }
}

// -------------------------------------------------------------------------------------------------

/// C lang compatible pattern event representation, as passed to the consumer
/// callback in `run_pattern` and `run_pattern_until_time`.
#[repr(C)]
pub struct PatternPlaybackEvent {
    pub sample_time: u64,
    pub duration_in_samples: u64,
    pub note_events: NoteEvents,
    pub parameter_change_events: ParameterChangeEvents,
}

impl PatternPlaybackEvent {
    /// Convert and forward a single event to the given callback
    fn forward_to_callback(
        callback_context: *mut c_void,
        callback: extern "C" fn(*mut c_void, &Self),
        item: pattrns::PatternEvent,
    ) {
        // NB: make sure event wrappers are valid/alive as long as the callback is called
        let (note_events, parameter_change_events) = if let Some(event) = item.event {
            match event {
                pattrns::Event::NoteEvents(note_events) => (
                    NoteEvents::from(note_events.as_slice()),
                    ParameterChangeEvents::default(),
                ),
                pattrns::Event::ParameterChangeEvent(parameter_change_event) => (
                    NoteEvents::default(),
                    ParameterChangeEvents::from([parameter_change_event].as_slice()),
                ),
            }
        } else {
            (NoteEvents::default(), ParameterChangeEvents::default())
        };
        let playback_event = Self {
            sample_time: item.time,
            duration_in_samples: item.duration,
            note_events,
            parameter_change_events,
        };
        callback(callback_context, &playback_event);
    }
}

/// C lang compatible representation of a rust `pattrns::Pattern`.
// NB: not #[repr(C)] to force cbindgen to export an opaque type
pub struct Pattern {
    pattern: Rc<RefCell<dyn pattrns::Pattern>>,
}

/// C lang compatible Result<Pattern, String> representation for new_pattern_from_string/file.
/// Error Strings must be deleted with `drop_error_string`.
/// Pattern values must be deleted with `drop_pattern`,
#[repr(C)]
pub enum PatternResult {
    Error(*const c_char),
    Value(*mut Pattern),
}

#[no_mangle]
/// Create a new pattern from the given script file path, using the given beat time and instrument.
/// The returned pattern result must be deleted via `drop_pattern` or `drop_error_string`.
pub unsafe extern "C" fn new_pattern_from_file(
    time_base: Timebase,
    instrument_id: *const u32,
    file_name: *const c_char,
) -> PatternResult {
    try_catch!(PatternResult, {
        let file_name = CStr::from_ptr(file_name).to_string_lossy();
        let result = pattrns::new_pattern_from_file(
            time_base.into(),
            if instrument_id.is_null() {
                None
            } else {
                Some(pattrns::InstrumentId::from(*instrument_id as usize))
            },
            file_name.into_owned().as_str(),
        );
        match result {
            Ok(pattern) => PatternResult::Value(Box::into_raw(Box::new(Pattern { pattern }))),
            Err(err) => PatternResult::Error(new_raw_cstring(&err.to_string())),
        }
    })
}

#[no_mangle]
/// Create a new pattern from the given script contents, using the given beat time and instrument.
/// The returned pattern result must be deleted via `drop_pattern` or `drop_error_string`.
pub unsafe extern "C" fn new_pattern_from_string(
    time_base: Timebase,
    instrument_id: *const u32,
    content: *const c_char,
    content_name: *const c_char,
) -> PatternResult {
    try_catch!(PatternResult, {
        let result = pattrns::new_pattern_from_string(
            time_base.into(),
            if instrument_id.is_null() {
                None
            } else {
                Some(pattrns::InstrumentId::from(*instrument_id as usize))
            },
            unsafe { &CStr::from_ptr(content).to_string_lossy() },
            unsafe { &CStr::from_ptr(content_name).to_string_lossy() },
        );
        match result {
            Ok(pattern) => PatternResult::Value(Box::into_raw(Box::new(Pattern { pattern }))),
            Err(err) => PatternResult::Error(new_raw_cstring(&err.to_string())),
        }
    })
}

#[no_mangle]
/// Create a new resetted clone from an existing pattern with the given timebase and instrument id.
/// The returned pattern result must be deleted via `drop_pattern` or `drop_error_string`.
pub unsafe extern "C" fn new_pattern_instance(
    this: *mut Pattern,
    time_base: Timebase,
) -> PatternResult {
    if this.is_null() {
        return PatternResult::Error(new_raw_cstring("Trying to clone a pattern from a null ptr"));
    }
    try_catch!(PatternResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        // create a clone
        let pattern = this.pattern.borrow().duplicate();
        // and reinitialize it
        {
            let mut pattern = pattern.borrow_mut();
            pattern.set_time_base(&time_base.into());
            pattern.reset();
        }
        // return result with the new boxed pattern
        PatternResult::Value(Box::into_raw(Box::new(Pattern { pattern })))
    })
}

#[no_mangle]
/// Get parameters of a pattern.
/// The returned result must be deleted via `drop_parameter_set` or `drop_error_string`.
pub unsafe extern "C" fn pattern_parameters(this: *mut Pattern) -> ParameterSetResult {
    if this.is_null() {
        return ParameterSetResult::Error(new_raw_cstring(
            "Trying to get input parameters from a null ptr",
        ));
    }
    try_catch!(ParameterSetResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let pattern = this.pattern.borrow();
        ParameterSetResult::Value(Box::into_raw(Box::new(ParameterSet::from(
            pattern.parameters(),
        ))))
    })
}

#[no_mangle]
/// Set a single parameter value of a pattern.
pub unsafe extern "C" fn set_pattern_parameter_value(
    this: *mut Pattern,
    id: *const c_char,
    value: f64,
) -> VoidResult {
    if this.is_null() {
        return VoidResult::Error(new_raw_cstring(
            "Trying to set an input parameter value for a null ptr",
        ));
    }
    try_catch!(VoidResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let pattern = this.pattern.borrow();
        let id = CStr::from_ptr(id).to_string_lossy();
        if let Some(parameter) = pattern.parameters().iter().find(|p| p.borrow().id() == id) {
            let mut parameter = parameter.borrow_mut();
            if !parameter.range().contains(&value) {
                return VoidResult::Error(new_raw_cstring("Input parameter value is out of range"));
            }
            parameter.set_value(value);
            VoidResult::Ok(())
        } else {
            VoidResult::Error(new_raw_cstring(
                "Trying to access and unknown input parameter",
            ))
        }
    })
}

#[no_mangle]
/// Get length in samples of a pattern's step.
pub unsafe extern "C" fn pattern_samples_per_step(this: *mut Pattern) -> F64Result {
    if this.is_null() {
        return F64Result::Error(new_raw_cstring(
            "Trying to get samples per step from a null ptr",
        ));
    }
    try_catch!(F64Result, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let pattern = this.pattern.borrow();
        let samples_per_step = pattern.step_length();
        F64Result::Value(samples_per_step)
    })
}

#[no_mangle]
/// Get length of the pattern's rhythm (a full cycle, in steps).
pub unsafe extern "C" fn pattern_step_count(this: *mut Pattern) -> UInt32Result {
    if this.is_null() {
        return UInt32Result::Error(new_raw_cstring(
            "Trying to get pattern length from a null ptr rhythm",
        ));
    }
    try_catch!(UInt32Result, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let pattern = this.pattern.borrow();
        let step_count = pattern.step_count();
        UInt32Result::Value(step_count as u32)
    })
}

#[no_mangle]
/// Set a new time base for a pattern.
pub unsafe extern "C" fn set_pattern_time_base(
    this: *mut Pattern,
    time_base: Timebase,
) -> VoidResult {
    try_catch!(VoidResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let mut pattern = this.pattern.borrow_mut();
        pattern.set_time_base(&time_base.into());
        VoidResult::Ok(())
    })
}

#[no_mangle]
/// Set trigger events for a pattern.
pub unsafe extern "C" fn set_pattern_trigger_event(
    this: *mut Pattern,
    note_events_ptr: *const NoteEvent,
    note_events_len: u32,
) -> VoidResult {
    try_catch!(VoidResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let mut pattern = this.pattern.borrow_mut();
        let event = {
            if note_events_ptr.is_null() || note_events_len == 0 {
                pattrns::Event::NoteEvents(vec![])
            } else {
                let note_events = ManuallyDrop::new(Vec::from_raw_parts(
                    note_events_ptr.cast_mut(),
                    note_events_len as usize,
                    note_events_len as usize,
                ));
                pattrns::Event::NoteEvents(
                    note_events
                        .iter()
                        .map(|e| Some(e.into()))
                        .collect::<Vec<_>>(),
                )
            }
        };
        pattern.set_trigger_event(&event);
        VoidResult::Ok(())
    })
}

#[no_mangle]
/// Run pattern, consuming the single next due event only.
/// NB: Events are only valid within the callback, so they must be consumed
/// or copied when used outside of the callback.
pub unsafe extern "C" fn run_pattern(
    this: *mut Pattern,
    callback_context: *mut c_void,
    callback: extern "C" fn(*mut c_void, &PatternPlaybackEvent),
) -> VoidResult {
    try_catch!(VoidResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let mut pattern = this.pattern.borrow_mut();
        if let Some(item) = pattern.run_until_time(pattrns::SampleTime::MAX) {
            PatternPlaybackEvent::forward_to_callback(callback_context, callback, item);
        }
        VoidResult::Ok(())
    })
}

#[no_mangle]
/// Run pattern, consuming all events which the pattern generated up to given sample time.
/// NB: Events are only valid within the callback, so they must be consumed
/// or copied when used outside of the callback.
pub unsafe extern "C" fn run_pattern_until_time(
    this: *mut Pattern,
    time: u64,
    callback_context: *mut c_void,
    callback: extern "C" fn(*mut c_void, &PatternPlaybackEvent),
) -> VoidResult {
    try_catch!(VoidResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let mut pattern = this.pattern.borrow_mut();
        while let Some(item) = pattern.run_until_time(time) {
            debug_assert!(item.time < time);
            PatternPlaybackEvent::forward_to_callback(callback_context, callback, item);
        }
        VoidResult::Ok(())
    })
}

#[no_mangle]
/// Run/seek pattern, discarding all events up to the given time.
pub unsafe extern "C" fn advance_pattern_until_time(this: *mut Pattern, time: u64) -> VoidResult {
    try_catch!(VoidResult, {
        let this = ManuallyDrop::new(Box::from_raw(this));
        let mut pattern = this.pattern.borrow_mut();
        pattern.advance_until_time(time);
        VoidResult::Ok(())
    })
}

#[no_mangle]
/// Delete a pattern which got allocated via `new_pattern_from_string/file`.
pub unsafe extern "C" fn drop_pattern(pattern: *mut Pattern) {
    if !pattern.is_null() {
        drop(Box::from_raw(pattern));
    }
}
