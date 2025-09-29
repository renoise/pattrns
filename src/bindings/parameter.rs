use mlua::prelude::*;

use crate::Parameter;

// ---------------------------------------------------------------------------------------------

/// Opaque Lua Userdata impl for a Parameter.
pub(crate) struct ParameterUserData {
    pub(crate) parameter: Parameter,
}

// Use default IntoLua impl for LuaUserData
impl LuaUserData for ParameterUserData {}

// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use crate::bindings::*;

    // use pretty_assertions::assert_eq;

    fn new_test_engine() -> LuaResult<Lua> {
        // create a new engine and register bindings
        let (mut lua, mut timeout_hook) = new_engine()?;
        register_bindings(
            &mut lua,
            &timeout_hook,
            &BeatTimeBase {
                beats_per_min: 120.0,
                beats_per_bar: 4,
                samples_per_sec: 44100,
            },
        )?;
        timeout_hook.reset();
        Ok(lua)
    }

    #[test]
    fn inputs() -> LuaResult<()> {
        let lua = new_test_engine()?;

        // boolean
        assert!(lua
            .load(r#"parameter.boolean(1, false)"#) // invalid id
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.boolean("name", "off")"#) // invalid default
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.boolean("name", false, {})"#) // invalid name
            .eval::<LuaValue>()
            .is_err());

        assert!(lua
            .load(r#"parameter.boolean("name", true)"#)
            .eval::<LuaValue>()
            .is_ok());
        assert!(lua
            .load(r#"parameter.boolean("name", false, "Fancy Name", "Fancy Description")"#)
            .eval::<LuaValue>()
            .is_ok());

        // integer
        assert!(lua
            .load(r#"parameter.integer({}, 1)"#) // invalid id
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.integer("name", false)"#) // not an integer
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.integer("name", 20.5)"#) // not an integer
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.integer("name", 50, {1, 20})"#) // out of range
            .eval::<LuaValue>()
            .is_err());

        assert!(lua
            .load(r#"parameter.integer("name", 50)"#)
            .eval::<LuaValue>()
            .is_ok());
        assert!(lua
            .load(r#"parameter.integer("name", 0, {-20, 20}, "Fancy Name", "Fancy Description")"#)
            .eval::<LuaValue>()
            .is_ok());

        // number
        assert!(lua
            .load(r#"parameter.number(12, 0.0)"#) // invalid id
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.number("name", false)"#) // default not a number
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.number("name", 50, {1, 20})"#) // out of range
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.number("name", 50, 100"#) // invalid range
            .eval::<LuaValue>()
            .is_err());

        assert!(lua
            .load(r#"parameter.number("name", 0.5)"#)
            .eval::<LuaValue>()
            .is_ok());
        assert!(lua
            .load(r#"parameter.number("name", 0, {-20, 20}, "Fancy Name", "Fancy Description")"#)
            .eval::<LuaValue>()
            .is_ok());
        assert!(lua
            .load(
                r#"parameter.number("name", 0.5, {-20.5, 20.5}, "Fancy Name", "Fancy Description")"#
            )
            .eval::<LuaValue>()
            .is_ok());

        // number
        assert!(lua
            .load(r#"parameter.enum(12, "default")"#) // invalid id
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.enum("name", 1, [1])"#) // default not a string
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.enum("name", "value", {"1", "2"})"#) // default not in values
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"parameter.enum("name", "a", {"a", "b", "A"})"#) // values not unique
            .eval::<LuaValue>()
            .is_err());

        assert!(lua
            .load(r#"parameter.enum("name", "value", {"VALUE", "value2"})"#)
            .eval::<LuaValue>()
            .is_ok());
        assert!(lua
            .load(r#"parameter.enum("name", "value", {"value", "wurst"}, "Fancy Name", "Fancy Description")"#)
            .eval::<LuaValue>()
            .is_ok());
        Ok(())
    }
}
