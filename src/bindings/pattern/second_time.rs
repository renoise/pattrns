use mlua::prelude::*;

use super::super::{
    unwrap::{
        bad_argument_error, emitter_from_value, gate_from_value, parameters_from_value,
        rhythm_from_value, rhythm_repeat_count_from_value,
    },
    LuaTimeoutHook,
};

use crate::prelude::*;

// -------------------------------------------------------------------------------------------------

impl LuaUserData for SecondTimePattern {
    // SecondTimePattern is only passed through ATM
}

impl SecondTimePattern {
    // create a SecondtimePattern from the given Lua table value
    pub(crate) fn from_table(
        lua: &Lua,
        timeout_hook: &LuaTimeoutHook,
        time_base: &BeatTimeBase,
        table: &LuaTable,
    ) -> LuaResult<SecondTimePattern> {
        // resolution
        let mut resolution = 1.0;
        if table.contains_key("resolution")? {
            resolution = table.get::<f64>("resolution")?;
            if resolution <= 0.0 {
                return Err(bad_argument_error(
                    "rhythm",
                    "resolution",
                    1,
                    "resolution must be > 0",
                ));
            }
        }
        // unit
        if table.contains_key("unit")? {
            let unit = table.get::<String>("unit")?;
            match unit.as_str() {
                "seconds" => (),
                "ms" => resolution /= 1000.0,
                _ => return Err(bad_argument_error("pattern", "unit", 1, 
                "expected one of 'ms|seconds' or 'bars|beats' or '1/1|1/2|1/4|1/8|1/16|1/32|1/64"))
            }
        }
        // create a new SecondTimePattern with the given time base and step
        let mut pattern = SecondTimePattern::new(*time_base, resolution);
        // offset
        if table.contains_key("offset")? {
            let offset = table.get::<f32>("offset")? as SecondTimeStep;
            if offset >= 0.0 {
                pattern = pattern.with_offset(offset * resolution);
            } else {
                return Err(bad_argument_error(
                    "pattern",
                    "offset",
                    1,
                    "offset must be a number >= 0",
                ));
            }
        }
        // parameter
        if table.contains_key("parameter")? {
            let value = table.get::<LuaTable>("parameter")?;
            let parameters = parameters_from_value(lua, &value)?;
            pattern = pattern.with_parameters(parameters);
        }
        // pulse
        if table.contains_key("pulse")? {
            let value = table.get::<LuaValue>("pulse")?;
            let rhythm = rhythm_from_value(lua, timeout_hook, &value, time_base)?;
            pattern = pattern.with_rhythm_dyn(rhythm);
        }
        // gate
        if table.contains_key("gate")? {
            let value = table.get::<LuaValue>("gate")?;
            let gate = gate_from_value(lua, timeout_hook, &value, time_base)?;
            pattern = pattern.with_gate_dyn(gate);
        }
        // repeat
        if table.contains_key("repeats")? {
            let value = table.get::<LuaValue>("repeats")?;
            let repeat = rhythm_repeat_count_from_value(&value)?;
            pattern = pattern.with_repeat(repeat);
        }
        // event
        if table.contains_key("event")? {
            let value: LuaValue = table.get::<LuaValue>("event")?;
            let emitter = emitter_from_value(lua, timeout_hook, &value, time_base)?;
            pattern = pattern.trigger_dyn(emitter);
        }
        Ok(pattern)
    }
}
