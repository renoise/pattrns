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

impl LuaUserData for BeatTimePattern {
    // BeatTimePattern is only passed through ATM
}

impl BeatTimePattern {
    // create a BeatTimePattern from the given Lua table value
    pub(crate) fn from_table(
        lua: &Lua,
        timeout_hook: &LuaTimeoutHook,
        time_base: &BeatTimeBase,
        table: &LuaTable,
    ) -> LuaResult<BeatTimePattern> {
        // resolution
        let mut resolution = 1.0;
        if table.contains_key("resolution")? {
            resolution = table.get::<f32>("resolution")?;
            if resolution <= 0.0 {
                return Err(bad_argument_error(
                    "pattern",
                    "resolution",
                    1,
                    "resolution must be > 0",
                ));
            }
        }
        // step
        let mut step = BeatTimeStep::Beats(resolution);
        if table.contains_key("unit")? {
            let unit = table.get::<String>("unit")?;
            match unit.as_str() {
                "bars" => step = BeatTimeStep::Bar(resolution),
                "1/1" => step = BeatTimeStep::Whole(resolution),
                "1/2" => step = BeatTimeStep::Half(resolution),
                "beats" | "1/4" => step = BeatTimeStep::Beats(resolution),
                "1/8" => step = BeatTimeStep::Eighth(resolution),
                "1/16" => step = BeatTimeStep::Sixteenth(resolution),
                "1/32" => step = BeatTimeStep::ThirtySecond(resolution),
                "1/64" => step = BeatTimeStep::SixtyFourth(resolution),
                _ => return Err(bad_argument_error("rhythm", "unit", 1, 
                "expected one of 'ms|seconds' or 'bars|beats' or '1/1|1/2|1/4|1/8|1/16|1/32|1/64"))
            }
        }
        // create a new BeatTimePattern with the given time base and step
        let mut pattern = BeatTimePattern::new(*time_base, step);
        // offset
        if table.contains_key("offset")? {
            let offset = table.get::<f32>("offset")?;
            if offset >= 0.0 {
                let mut new_step = pattern.step();
                new_step.set_steps(offset * resolution);
                pattern = pattern.with_offset(new_step);
            } else {
                return Err(bad_argument_error(
                    "pattern",
                    "offset",
                    1,
                    "offset must be >= 0",
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
            let value = table.get::<LuaValue>("event")?;
            let emitter = emitter_from_value(lua, timeout_hook, &value, time_base)?;
            pattern = pattern.trigger_dyn(emitter);
        }
        Ok(pattern)
    }
}
