//! Various lua->rust conversion helpers

use std::{cell::RefCell, ops::RangeBounds, rc::Rc, sync::Arc};

use mlua::prelude::*;

use crate::{
    bindings::{
        callback::LuaCallback, cycle::CycleUserData, note::NoteUserData,
        parameter::ParameterUserData, sequence::SequenceUserData, LuaTimeoutHook,
    },
    prelude::*,
};

// ---------------------------------------------------------------------------------------------

// Error helpers
pub(crate) fn bad_argument_error<'a, 'b, S1: Into<Option<&'a str>>, S2: Into<Option<&'b str>>>(
    func: S1,
    arg_name: S2,
    arg_index: usize,
    message: &str,
) -> LuaError {
    LuaError::BadArgument {
        to: func.into().map(String::from),
        name: arg_name.into().map(String::from),
        pos: arg_index,
        cause: Arc::new(LuaError::RuntimeError(message.to_string())),
    }
}

// -------------------------------------------------------------------------------------------------

// Check for known table properties
pub(crate) fn validate_table_properties(table: &LuaTable, properties: &[&str]) -> LuaResult<()> {
    for (key, _) in table.clone().pairs::<LuaValue, LuaValue>().flatten() {
        if let Some(key) = key.as_string().map(|s| s.to_string_lossy()) {
            if !properties.contains(&key.as_str()) {
                return Err(LuaError::RuntimeError(format!(
                    "invalid/unknown table property: '{}'. valid properties are: '{}'",
                    key,
                    properties.join(", ")
                )));
            }
        } else {
            return Err(LuaError::RuntimeError(format!(
                "invalid/unknown property of type '{}' in table",
                key.type_name(),
            )));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------------------------

impl IntoLua for Note {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        self.to_string().into_lua(lua)
    }
}

impl FromLua for Note {
    fn from_lua(value: LuaValue, _lua: &Lua) -> LuaResult<Self> {
        match value {
            LuaValue::Integer(note_value) => Ok(Note::from(note_value as u8)),
            LuaValue::String(str) => {
                Note::try_from(&str.to_string_lossy() as &str).map_err(|err| {
                    LuaError::FromLuaConversionError {
                        from: "string",
                        to: "note".to_string(),
                        message: Some(err.to_string()),
                    }
                })
            }
            _ => Err(LuaError::FromLuaConversionError {
                from: value.type_name(),
                to: "note".to_string(),
                message: Some("expected a note number or note string".to_string()),
            }),
        }
    }
}

// ---------------------------------------------------------------------------------------------

impl IntoLua for NoteEvent {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        table.set("key", self.note.into_lua(lua)?)?;
        if let Some(instrument) = self.instrument {
            table.set(
                "instrument",
                LuaInteger::try_from(usize::from(instrument)).unwrap_or(LuaInteger::MAX),
            )?;
        }
        if let Some(glide) = self.glide {
            table.set("glide", glide as f64)?;
        }
        table.set("volume", self.volume as f64)?;
        table.set("panning", self.panning as f64)?;
        table.set("delay", self.delay as f64)?;
        Ok(LuaValue::Table(table))
    }
}

// ---------------------------------------------------------------------------------------------

impl IntoLua for ParameterChangeEvent {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        let table = lua.create_table()?;
        if let Some(id) = self.parameter {
            table.set("parameter", usize::from(id).into_lua(lua)?)?;
        }
        table.set("value", self.value as f64)?;
        Ok(LuaValue::Table(table))
    }
}

// ---------------------------------------------------------------------------------------------

impl IntoLua for Event {
    fn into_lua(self, lua: &Lua) -> LuaResult<LuaValue> {
        match self {
            Event::NoteEvents(note_events) => NoteUserData { notes: note_events }.into_lua(lua),
            Event::ParameterChangeEvent(parameter_change_event) => {
                // TODO: ParameterChangeUserData
                parameter_change_event.into_lua(lua)
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------

// Check if a lua value is a string, without using implicit conversions.
pub(crate) fn string_from_value(
    value: &LuaValue,
    function: &str,
    arg_name: &str,
    arg_index: usize,
) -> LuaResult<String> {
    if let Some(string) = value.as_string().map(|s| s.to_string_lossy()) {
        Ok(string)
    } else {
        Err(bad_argument_error(
            function,
            arg_name,
            arg_index,
            "expecting a string or nil",
        ))
    }
}

// Check if a lua value is a string - without using implicit conversions - or nil.
pub(crate) fn optional_string_from_value(
    value: &LuaValue,
    function: &str,
    arg_name: &str,
    arg_index: usize,
) -> LuaResult<String> {
    if let Some(string) = value.as_string().map(|s| s.to_string_lossy()) {
        Ok(string)
    } else if value.is_nil() {
        Ok(String::new())
    } else {
        Err(bad_argument_error(
            function,
            arg_name,
            arg_index,
            "expecting a string or nil",
        ))
    }
}

// ---------------------------------------------------------------------------------------------

// Check if a lua value is a sequence (an array alike table).
pub(crate) fn sequence_from_value(value: &LuaValue) -> Option<Vec<LuaValue>> {
    if let Some(table) = value.as_table() {
        sequence_from_table(table)
    } else {
        None
    }
}

// Check if a lua table is a sequence (an array alike table).
pub(crate) fn sequence_from_table(table: &LuaTable) -> Option<Vec<LuaValue>> {
    let sequence = table
        .clone()
        .sequence_values::<LuaValue>()
        .collect::<Vec<_>>();
    if !sequence.is_empty() {
        return Some(sequence.into_iter().map(Result::unwrap).collect());
    }
    None
}

// ---------------------------------------------------------------------------------------------

fn optional_float_array_from_value<Range>(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
    name: &str,
    range: Range,
) -> LuaResult<Vec<Option<f32>>>
where
    Range: RangeBounds<f32> + std::fmt::Debug,
{
    // NB: use pairs instead of sequence values to not stop at nil values in the array
    let mut values: Vec<Option<f32>> = vec![];
    if let Some(value_table) = value.as_table() {
        for value in value_table.pairs::<LuaInteger, LuaValue>() {
            let (index, value) = value?;
            if index >= 1 {
                values.resize(index as usize, None);
                if value != LuaValue::Nil {
                    values[index as usize - 1] = Some(f32::from_lua(value, lua)?);
                }
            }
        }
    } else {
        let value = f32::from_lua(value, lua)?;
        values = (0..array_len)
            .map(|_| Some(value))
            .collect::<Vec<Option<f32>>>();
    }
    for value in values.iter().flatten() {
        if !range.contains(value) {
            return Err(bad_argument_error(
                None,
                name,
                1,
                format!("{} must be in range [{:?}] but is '{}'", name, range, value).as_str(),
            ));
        }
    }
    Ok(values)
}

fn float_array_from_value<Range>(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
    name: &str,
    range: Range,
) -> LuaResult<Vec<f32>>
where
    Range: RangeBounds<f32> + std::fmt::Debug,
{
    let values;
    if let Some(value_table) = value.as_table() {
        values = value_table
            .clone()
            .sequence_values::<f32>()
            .collect::<LuaResult<Vec<f32>>>()?;
    } else {
        let value = f32::from_lua(value, lua)?;
        values = (0..array_len).map(|_| value).collect::<Vec<f32>>();
    }
    for value in &values {
        if !range.contains(value) {
            return Err(bad_argument_error(
                None,
                name,
                1,
                format!("{} must be in range [{:?}] but is '{}'", name, range, value).as_str(),
            ));
        }
    }
    Ok(values)
}

fn integer_array_from_value<Range>(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
    name: &str,
    range: Range,
) -> LuaResult<Vec<i32>>
where
    Range: RangeBounds<i32> + std::fmt::Debug,
{
    let values;
    if let Some(value_table) = value.as_table() {
        values = value_table
            .clone()
            .sequence_values::<i32>()
            .collect::<LuaResult<Vec<i32>>>()?;
    } else {
        let value = i32::from_lua(value, lua)?;
        values = (0..array_len).map(|_| value).collect::<Vec<i32>>();
    }
    for value in &values {
        if !range.contains(value) {
            return Err(bad_argument_error(
                None,
                name,
                1,
                format!("{} must be in range [{:?}] but is '{}'", name, range, value).as_str(),
            ));
        }
    }
    Ok(values)
}

pub(crate) fn transpose_steps_array_from_value(
    lua: &Lua,
    step: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<i32>> {
    integer_array_from_value(lua, step, array_len, "transpose_step", i32::MIN..i32::MAX)
}

pub(crate) fn instrument_array_from_value(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<i32>> {
    integer_array_from_value(lua, value, array_len, "instrument", 0..=i32::MAX)
}

pub(crate) fn amplify_array_from_value(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<f32>> {
    float_array_from_value(lua, value, array_len, "volume", 0.0..=f32::MAX)
}

pub(crate) fn volume_array_from_value(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<f32>> {
    float_array_from_value(lua, value, array_len, "volume", 0.0..=1.0)
}

pub(crate) fn panning_array_from_value(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<f32>> {
    float_array_from_value(lua, value, array_len, "panning", -1.0..=1.0)
}

pub(crate) fn delay_array_from_value(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<f32>> {
    float_array_from_value(lua, value, array_len, "delay", 0.0..=1.0)
}

pub(crate) fn glide_array_from_value(
    lua: &Lua,
    value: LuaValue,
    array_len: usize,
) -> LuaResult<Vec<Option<f32>>> {
    optional_float_array_from_value(lua, value, array_len, "glide", 0.0..)
}

// ---------------------------------------------------------------------------------------------

fn optional_float_value_from_table<Range>(
    table: &LuaTable,
    name: &'static str,
    range: Range,
) -> LuaResult<Option<f32>>
where
    Range: RangeBounds<f32> + std::fmt::Debug,
{
    let value = table.get::<LuaValue>(name)?;
    if value.is_nil() {
        Ok(None)
    } else if let Some(value) = value
        .as_number()
        .or(value.as_integer().map(|i| i as LuaNumber))
    {
        if range.contains(&(value as f32)) {
            Ok(Some(value as f32))
        } else {
            Err(LuaError::RuntimeError(format!(
                "{} property must be in range [{:?}] but is '{}'",
                name, range, value
            )))
        }
    } else {
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "number".to_string(),
            message: Some(format!("'{}' property must be a number", name)),
        })
    }
}

fn float_value_from_table<Range>(
    table: &LuaTable,
    name: &'static str,
    range: Range,
    default: f32,
) -> LuaResult<f32>
where
    Range: RangeBounds<f32> + std::fmt::Debug,
{
    Ok(optional_float_value_from_table(table, name, range)?.unwrap_or(default))
}

pub(crate) fn instrument_value_from_table(table: &LuaTable) -> LuaResult<Option<InstrumentId>> {
    let value = table.get::<LuaValue>("instrument")?;
    if value.is_nil() {
        Ok(None)
    } else if let Some(value) = value.as_integer() {
        if value >= 0 {
            Ok(Some(InstrumentId::from(value as usize)))
        } else {
            Err(LuaError::RuntimeError(format!(
                "'instrument' property must be > 0 but is '{}'",
                value
            )))
        }
    } else {
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "number".to_string(),
            message: Some("'instrument' property must be an integer".to_string()),
        })
    }
}

pub(crate) fn volume_value_from_table(table: &LuaTable) -> LuaResult<f32> {
    float_value_from_table(table, "volume", 0.0..=1.0, 1.0)
}

pub(crate) fn panning_value_from_table(table: &LuaTable) -> LuaResult<f32> {
    float_value_from_table(table, "panning", -1.0..=1.0, 0.0)
}

pub(crate) fn delay_value_from_table(table: &LuaTable) -> LuaResult<f32> {
    float_value_from_table(table, "delay", 0.0..1.0, 0.0)
}

pub(crate) fn glide_value_from_table(table: &LuaTable) -> LuaResult<Option<f32>> {
    optional_float_value_from_table(table, "glide", 0.0..)
}

fn float_value_from_string<Range>(
    str: &str,
    name: &'static str,
    range: Range,
    default: f32,
) -> LuaResult<f32>
where
    Range: RangeBounds<f32> + std::fmt::Debug,
{
    if str.is_empty() {
        Ok(default)
    } else {
        let value;
        if let Ok(int) = str.parse::<i32>() {
            value = int as f32;
        } else if let Ok(float) = str.parse::<f32>() {
            value = float;
        } else {
            return Err(LuaError::FromLuaConversionError {
                from: "string",
                to: "number".to_string(),
                message: Some(format!("{} property '{}' is not a number", name, str)),
            });
        }
        if range.contains(&value) {
            Ok(value)
        } else {
            Err(LuaError::RuntimeError(format!(
                "{} property must be in range [{:?}] but is '{}'",
                name, range, value
            )))
        }
    }
}

pub(crate) fn instrument_value_from_string(str: &str) -> LuaResult<Option<InstrumentId>> {
    if str.is_empty() {
        Ok(None)
    } else if let Ok(value) = str.parse::<LuaInteger>() {
        if value < 0 {
            return Err(LuaError::RuntimeError(format!(
                "instrument property must be >= 0 but is '{}'",
                value
            )));
        }
        Ok(Some(InstrumentId::from(value as usize)))
    } else {
        Err(LuaError::FromLuaConversionError {
            from: "string",
            to: "number".to_string(),
            message: Some(format!("instrument property '{}' is not a number", str)),
        })
    }
}

pub(crate) fn volume_value_from_string(str: &str) -> LuaResult<f32> {
    float_value_from_string(str, "volume", 0.0..=1.0, 1.0)
}

pub(crate) fn panning_value_from_string(str: &str) -> LuaResult<f32> {
    float_value_from_string(str, "panning", -1.0..=1.0, 0.0)
}

pub(crate) fn delay_value_from_string(str: &str) -> LuaResult<f32> {
    float_value_from_string(str, "delay", 0.0..1.0, 0.0)
}

pub(crate) fn glide_value_from_string(str: &str) -> LuaResult<f32> {
    float_value_from_string(str, "glide", 0.0.., 0.0)
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn is_empty_note_string(s: &str) -> bool {
    matches!(s, "" | "-" | "--" | "---" | "." | ".." | "...")
}

// ---------------------------------------------------------------------------------------------

pub(crate) fn note_degree_from_value(arg: &LuaValue, arg_index: usize) -> LuaResult<usize> {
    let degree_error = || {
        Err(bad_argument_error(
            "chord",
            "degree",
            arg_index,
            "degree must be an integer or roman number string in range [1, 7] \
              (e.g. 3, 5, or 'iii' or 'V')",
        ))
    };
    if let Some(value) = arg.as_usize() {
        if !(1..=7).contains(&value) {
            degree_error()
        } else {
            Ok(value)
        }
    } else if let Some(value) = arg.as_string().map(|s| s.to_string_lossy()) {
        match value.to_lowercase().as_str() {
            "i" => Ok(1),
            "ii" => Ok(2),
            "iii" => Ok(3),
            "iv" => Ok(4),
            "v" => Ok(5),
            "vi" => Ok(6),
            "vii" => Ok(7),
            _ => degree_error(),
        }
    } else {
        degree_error()
    }
}

pub(crate) fn note_event_from_number(note_value: LuaInteger) -> LuaResult<Option<NoteEvent>> {
    match note_value {
        0..=0x7f | 0xFE | 0xFF => Ok(new_note(note_value as u8)),
        _ => Err(LuaError::RuntimeError(format!(
            "note number must be 0xFF for empty notes or 0xFE for off notes or must be in range [0..=0x7f], but is: '{}'",
            note_value
        ))),
    }
}

pub(crate) fn note_event_from_string(str: &str) -> LuaResult<Option<NoteEvent>> {
    let mut white_space_splits = str.split(' ').filter(|v| !v.is_empty());
    let note_part = white_space_splits.next().unwrap_or("");
    if is_empty_note_string(note_part) {
        Ok(None)
    } else {
        let note =
            Note::try_from(note_part).map_err(|err| LuaError::RuntimeError(err.to_string()))?;
        let mut instrument = None;
        let mut volume = 1.0;
        let mut panning = 0.0;
        let mut delay = 0.0;
        let mut glide = None;
        for split in white_space_splits {
            if let Some(instrument_str) = split.strip_prefix('#') {
                instrument = instrument_value_from_string(instrument_str)?;
            } else if let Some(volume_str) = split.strip_prefix('v') {
                volume = volume_value_from_string(volume_str)?;
            } else if let Some(panning_str) = split.strip_prefix('p') {
                panning = panning_value_from_string(panning_str)?;
            } else if let Some(delay_str) = split.strip_prefix('d') {
                delay = delay_value_from_string(delay_str)?;
            } else if let Some(glide_str) = split.strip_prefix('g') {
                glide = Some(glide_value_from_string(glide_str)?);
            } else {
                return Err(LuaError::RuntimeError(
                    format!("invalid note property: '{split}'. ")
                        + "expecting only number values with  "
                        + "'#' (instrument), 'v' (volume), 'p' (panning), 'd' (delay) or 'g' (glide) "
                        + "prefixes here.",
                ));
            }
        }
        Ok(Some(NoteEvent {
            note,
            instrument,
            glide,
            volume,
            panning,
            delay,
        }))
    }
}

pub(crate) fn note_event_from_table_map(table: &LuaTable) -> LuaResult<Option<NoteEvent>> {
    if table.is_empty() {
        return Ok(None);
    }
    let key = table.get::<LuaValue>("key")?;
    if key.is_nil() {
        Err(LuaError::RuntimeError(
            "missing 'key' property in note table".to_string(),
        ))
    } else {
        let instrument = instrument_value_from_table(table)?;
        let volume = volume_value_from_table(table)?;
        let panning = panning_value_from_table(table)?;
        let delay = delay_value_from_table(table)?;
        let glide = glide_value_from_table(table)?;
        // { key = 60, [volume = 1.0, panning = 0.0, delay = 0.0] }
        if let Some(note_value) = key.as_i32() {
            Ok(Some(NoteEvent {
                note: Note::from(note_value as u8),
                instrument,
                glide,
                volume,
                panning,
                delay,
            }))
        }
        // { key = "C4", [instrument = 1, volume = 1.0, panning = 0.0, delay = 0.0] }
        else if let Some(note_str) = key.as_string().map(|s| s.to_string_lossy()) {
            let note = Note::try_from(&*note_str)
                .map_err(|err| LuaError::RuntimeError(err.to_string()))?;
            Ok(Some(NoteEvent {
                note,
                instrument,
                glide,
                volume,
                panning,
                delay,
            }))
        } else {
            Err(LuaError::FromLuaConversionError {
                from: key.type_name(),
                to: "note".to_string(),
                message: Some("invalid 'key' property in note table".to_string()),
            })
        }
    }
}

pub(crate) fn note_event_from_value(
    arg: &LuaValue,
    arg_index: Option<usize>,
) -> LuaResult<Option<NoteEvent>> {
    match arg {
        LuaValue::Nil => Ok(None),
        LuaValue::Integer(note_value) => note_event_from_number(*note_value),
        LuaValue::String(str) => note_event_from_string(&str.to_string_lossy()),
        LuaValue::Table(table) => note_event_from_table_map(table),
        _ => Err(LuaError::FromLuaConversionError {
            from: arg.type_name(),
            to: "note".to_string(),
            message: if let Some(index) = arg_index {
                Some(format!("arg #{} is not a valid note property", index + 1).to_string())
            } else {
                Some("invalid note property".to_string())
            },
        }),
    }
}

pub(crate) fn note_events_from_value(
    arg: &LuaValue,
    arg_index: Option<usize>,
) -> LuaResult<Vec<Option<NoteEvent>>> {
    match arg {
        LuaValue::UserData(userdata) => {
            if userdata.is::<NoteUserData>() {
                let chord = userdata.borrow::<NoteUserData>()?;
                Ok(chord.notes.clone())
            } else if userdata.is::<SequenceUserData>() {
                Err(LuaError::FromLuaConversionError {
                    from: "userdata",
                    to: "note".to_string(),
                    message: Some("can't nest sequences in sequences".to_string()),
                })
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "userdata",
                    to: "note".to_string(),
                    message: if let Some(index) = arg_index {
                        Some(
                            format!(
                                "user data at #{} can't be converted to a note array",
                                index + 1
                            )
                            .to_string(),
                        )
                    } else {
                        Some("user data can't be converted to note array".to_string())
                    },
                })
            }
        }
        LuaValue::Table(table) => {
            // array like { "C4", "C5" }
            if let Some(sequence) = sequence_from_table(&table.clone()) {
                let mut note_events = vec![];
                for (arg_index, arg) in sequence.iter().enumerate() {
                    // flatten sequence events into a single array
                    note_events.append(&mut note_events_from_value(arg, Some(arg_index))?);
                }
                Ok(note_events)
            // { key = xxx } map
            } else {
                Ok(vec![note_event_from_value(arg, arg_index)?])
            }
        }
        LuaValue::String(str) => {
            let str = str.to_string_lossy().to_string();
            // a string with ' is a chord
            if str.contains('\'') {
                Ok(chord_events_from_string(&str)?)
            } else {
                Ok(vec![note_event_from_string(&str)?])
            }
        }
        _ => Ok(vec![note_event_from_value(arg, arg_index)?]),
    }
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn chord_events_from_string(chord_string: &str) -> LuaResult<Vec<Option<NoteEvent>>> {
    let mut white_space_splits = chord_string.split(' ').filter(|v| !v.is_empty());
    let chord_part = white_space_splits.next().unwrap_or("");
    let chord =
        Chord::try_from(chord_part).map_err(|err| LuaError::RuntimeError(err.to_string()))?;
    let mut instrument = None;
    let mut volume = 1.0;
    let mut panning = 0.0;
    let mut delay = 0.0;
    let mut glide = None;
    for split in white_space_splits {
        if let Some(instrument_str) = split.strip_prefix('#') {
            instrument = instrument_value_from_string(instrument_str)?;
        } else if let Some(volume_str) = split.strip_prefix('v') {
            volume = volume_value_from_string(volume_str)?;
        } else if let Some(panning_str) = split.strip_prefix('p') {
            panning = panning_value_from_string(panning_str)?;
        } else if let Some(delay_str) = split.strip_prefix('d') {
            delay = delay_value_from_string(delay_str)?;
        } else if let Some(glide_str) = split.strip_prefix('g') {
            glide = Some(glide_value_from_string(glide_str)?);
        } else {
            return Err(LuaError::RuntimeError(
                format!("invalid note property: '{split}'. ")
                    + "expecting only number values with  "
                    + "'#' (instrument), 'v' (volume), 'p' (panning), 'd' (delay) or 'g' (glide) "
                    + "prefixes here.",
            ));
        }
    }
    Ok(chord
        .intervals()
        .iter()
        .map(|i| {
            Some(NoteEvent {
                note: Note::from(chord.note() as u8 + i),
                instrument,
                glide,
                volume,
                panning,
                delay,
            })
        })
        .collect::<Vec<_>>())
}

pub(crate) fn chord_events_from_mode(
    note: &LuaValue,
    mode: &str,
) -> LuaResult<Vec<Option<NoteEvent>>> {
    let note_event = note_event_from_value(note, Some(1))?;
    if let Some(note_event) = note_event {
        let chord = Chord::try_from((note_event.note, mode))
            .map_err(|err| LuaError::RuntimeError(err.to_string()))?;
        Ok(chord
            .intervals()
            .iter()
            .map(|i| {
                Some(NoteEvent {
                    note: chord.note().transposed(*i as i32),
                    ..note_event.clone()
                })
            })
            .collect::<Vec<_>>())
    } else {
        Err(LuaError::FromLuaConversionError {
            from: note.type_name(),
            to: "note".to_string(),
            message: Some("invalid note in chord: note value is undefined".to_string()),
        })
    }
}

pub(crate) fn chord_events_from_intervals(
    note: &LuaValue,
    intervals: &[i32],
) -> LuaResult<Vec<Option<NoteEvent>>> {
    let note_event = note_event_from_value(note, Some(1))?;
    if let Some(note_event) = note_event {
        let chord = Chord::try_from((note_event.note, intervals))
            .map_err(|err| LuaError::RuntimeError(err.to_string()))?;
        Ok(chord
            .intervals()
            .iter()
            .map(|i| {
                Some(NoteEvent {
                    note: chord.note().transposed(*i as i32),
                    ..note_event.clone()
                })
            })
            .collect::<Vec<_>>())
    } else {
        Err(LuaError::FromLuaConversionError {
            from: note.type_name(),
            to: "note".to_string(),
            message: Some("invalid note in chord: note value is undefined".to_string()),
        })
    }
}

// -------------------------------------------------------------------------------------------------

pub fn pulse_from_value(value: &LuaValue) -> LuaResult<Pulse> {
    match value {
        LuaValue::Nil => Ok(Pulse::Pulse(0.0)),
        LuaValue::Boolean(bool) => Ok(Pulse::from(*bool)),
        LuaValue::Integer(integer) => Ok(Pulse::from(*integer as u32)),
        LuaValue::Number(number) => Ok(Pulse::from(*number as f32)),
        LuaValue::String(str) => {
            let str = str.to_string_lossy();
            if let Ok(number) = str.parse::<f32>() {
                Ok(Pulse::from(number))
            } else if let Ok(integer) = str.parse::<u32>() {
                Ok(Pulse::from(integer))
            } else if let Ok(bool) = str.parse::<bool>() {
                Ok(Pulse::from(bool))
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "string",
                    to: "pulse".to_string(),
                    message: Some("Invalid pulse string value".to_string()),
                })
            }
        }
        LuaValue::Table(table) => {
            let sub_div = table
                .clone()
                .sequence_values()
                .map(|result| pulse_from_value(&result?))
                .collect::<LuaResult<Vec<Pulse>>>()?;
            Ok(Pulse::from(sub_div))
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "pulse".to_string(),
            message: Some("Invalid pulse value".to_string()),
        }),
    }
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn rhythm_repeat_count_from_value(value: &LuaValue) -> LuaResult<Option<usize>> {
    if let Some(boolean) = value.as_boolean() {
        if boolean {
            Ok(None)
        } else {
            Ok(Some(0))
        }
    } else if let Some(number) = value.as_usize() {
        Ok(Some(number))
    } else {
        Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "repeats".to_string(),
            message: Some("must be a boolean or integer value > 0".to_string()),
        })
    }
}

// -------------------------------------------------------------------------------------------------

pub fn gate_trigger_from_value(value: &LuaValue) -> LuaResult<bool> {
    match value {
        LuaValue::Nil => Ok(false),
        LuaValue::Boolean(bool) => Ok(*bool),
        LuaValue::Integer(integer) => Ok(*integer != 0),
        LuaValue::Number(number) => Ok(*number != 0.0),
        LuaValue::String(str) => {
            let str = str.to_string_lossy();
            if let Ok(number) = str.parse::<f32>() {
                Ok(number != 0.0)
            } else if let Ok(integer) = str.parse::<u32>() {
                Ok(integer != 0)
            } else if let Ok(bool) = str.parse::<bool>() {
                Ok(bool)
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "string",
                    to: "gate value".to_string(),
                    message: Some("Invalid boolean gate string value".to_string()),
                })
            }
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "gate value".to_string(),
            message: Some("Invalid boolean gate value".to_string()),
        }),
    }
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn parameters_from_value(_lua: &Lua, value: &LuaTable) -> LuaResult<ParameterSet> {
    let mut parameters = ParameterSet::new();
    for user_data in value.clone().sequence_values::<LuaAnyUserData>() {
        if let Ok(parameter_user_data) = user_data?.take::<ParameterUserData>() {
            let parameter_id = parameter_user_data.parameter.id().to_string();
            if parameter_id.is_empty() {
                return Err(LuaError::FromLuaConversionError {
                    from: "user_data",
                    to: "parameter".to_string(),
                    message: Some("Parameter id's can not be empty.".to_string()),
                });
            }
            let parameter = Rc::new(RefCell::new(parameter_user_data.parameter));
            if parameters.iter().any(|p| p.borrow().id() == parameter_id) {
                return Err(LuaError::FromLuaConversionError {
                    from: "user_data",
                    to: "parameter".to_string(),
                    message: Some(format!(
                        "Parameter id's must be unique: a parameter with id '{}' already exists.",
                        parameter_id
                    )),
                });
            }
            parameters.push(parameter)
        } else {
            return Err(LuaError::FromLuaConversionError {
                from: "user_data",
                to: "parameter".to_string(),
                message: Some("Invalid parameter".to_string()),
            });
        }
    }
    Ok(parameters)
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn rhythm_from_value(
    lua: &Lua,
    timeout_hook: &LuaTimeoutHook,
    value: &LuaValue,
    time_base: &BeatTimeBase,
) -> LuaResult<Box<dyn Rhythm>> {
    match value {
        LuaValue::Function(func) => {
            let callback = LuaCallback::new(lua, func.clone())?;
            let rhythm = ScriptedRhythm::new(timeout_hook, callback, time_base)?;
            Ok(Box::new(rhythm))
        }
        LuaValue::Table(table) => {
            let pulses = table
                .clone()
                .sequence_values::<LuaValue>()
                .map(|result| pulse_from_value(&result?))
                .collect::<LuaResult<Vec<Pulse>>>()?;
            Ok(Box::new(pulses.to_rhythm()))
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "rhythm".to_string(),
            message: Some("pulse must either be an array or a function".to_string()),
        }),
    }
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn gate_from_value(
    lua: &Lua,
    timeout_hook: &LuaTimeoutHook,
    value: &LuaValue,
    time_base: &BeatTimeBase,
) -> LuaResult<Box<dyn Gate>> {
    match value {
        LuaValue::Function(func) => {
            let callback = LuaCallback::new(lua, func.clone())?;
            let gate = ScriptedGate::new(timeout_hook, callback, time_base)?;
            Ok(Box::new(gate))
        }
        _ => Err(LuaError::FromLuaConversionError {
            from: value.type_name(),
            to: "gate".to_string(),
            message: Some("gate must either be nil or a function".to_string()),
        }),
    }
}

// -------------------------------------------------------------------------------------------------

pub(crate) fn emitter_from_value(
    lua: &Lua,
    timeout_hook: &LuaTimeoutHook,
    value: &LuaValue,
    time_base: &BeatTimeBase,
) -> LuaResult<Box<dyn Emitter>> {
    match value {
        LuaValue::UserData(userdata) => {
            if userdata.is::<NoteUserData>() {
                let note = userdata.borrow::<NoteUserData>()?.clone();
                Ok(Box::new(note.notes.to_emitter()))
            } else if userdata.is::<SequenceUserData>() {
                let sequence = userdata.borrow::<SequenceUserData>()?.clone();
                Ok(Box::new(sequence.notes.to_sequence_emitter()))
            } else if userdata.is::<CycleUserData>() {
                // NB: take instead of cloning: cycle userdata has no other usage than being defined
                let userdata = userdata.take::<CycleUserData>()?;
                let cycle = userdata.cycle;
                if let Some(mapping_function) = userdata.mapping_function {
                    let mapping_callback = LuaCallback::new(lua, mapping_function)?;
                    let emitter = ScriptedCycleEmitter::with_mapping_callback(
                        cycle,
                        timeout_hook,
                        mapping_callback,
                        time_base,
                    )?;
                    Ok(Box::new(emitter))
                } else {
                    let mappings = userdata.mappings;
                    let emitter = ScriptedCycleEmitter::with_mappings(cycle, mappings);
                    Ok(Box::new(emitter))
                }
            } else {
                Err(LuaError::FromLuaConversionError {
                    from: "userdata",
                    to: "notes".to_string(),
                    message: Some("expecting a note, sequence or cycle here".to_string()),
                })
            }
        }
        LuaValue::Function(function) => {
            let callback = LuaCallback::new(lua, function.clone())?;
            let emitter = ScriptedEmitter::new(timeout_hook, callback, time_base)?;
            Ok(Box::new(emitter))
        }
        LuaValue::Table(ref table) => {
            // convert an array alike table to a event sequence
            if let Some(sequence) = sequence_from_table(table) {
                let mut note_event_sequence = vec![];
                for (arg_index, arg) in sequence.iter().enumerate() {
                    note_event_sequence.push(note_events_from_value(arg, Some(arg_index))?);
                }
                let iter = note_event_sequence.to_sequence_emitter();
                Ok(Box::new(iter))
            }
            // convert table to a single note event
            else {
                let emitter = note_event_from_value(value, None)?.to_emitter();
                Ok(Box::new(emitter))
            }
        }
        _ => {
            // try converting a note number or note/chord string to an emitter.
            let emitter = note_events_from_value(value, None)?.to_emitter();
            Ok(Box::new(emitter))
        }
    }
}
