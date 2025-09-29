use mlua::prelude::*;

use crate::prelude::*;

use super::{
    bad_argument_error,
    unwrap::{note_degree_from_value, note_event_from_value},
};

// ---------------------------------------------------------------------------------------------

impl LuaUserData for Scale {
    fn add_fields<F: LuaUserDataFields<Self>>(fields: &mut F) {
        fields.add_field_method_get("notes", |lua, this| -> LuaResult<LuaTable> {
            lua.create_sequence_from(this.notes().iter().map(|n| LuaInteger::from(*n as u8)))
        });
    }

    fn add_methods<M: LuaUserDataMethods<Self>>(methods: &mut M) {
        methods.add_method(
            "chord",
            |lua, this, args: LuaMultiValue| -> LuaResult<LuaTable> {
                // parse degree
                let mut degree = 1;
                #[allow(clippy::get_first)]
                if let Some(degree_value) = args.get(0) {
                    degree = note_degree_from_value(degree_value, 1)?;
                }
                // parse count
                let mut count = 3;
                if let Some(count_value) = args.get(1) {
                    let count_error = || {
                        Err(bad_argument_error(
                            "chord",
                            "num_notes",
                            1,
                            "number of notes must be an integer in range [1..=5]",
                        ))
                    };
                    if let Some(value) = count_value.as_usize() {
                        count = value;
                        if !(1..=5).contains(&count) {
                            return count_error();
                        }
                    } else {
                        return count_error();
                    }
                }
                let notes = this
                    .chord_from_degree(degree, count)
                    .iter()
                    .map(|n| LuaInteger::from(*n as u8))
                    .collect::<Vec<_>>();
                lua.create_sequence_from(notes)
            },
        );

        methods.add_method(
            "notes_iter",
            |lua, this, count_value: LuaValue| -> LuaResult<LuaFunction> {
                let mut max_notes = usize::MAX;
                if !count_value.is_nil() {
                    if let Some(value) = count_value.as_integer() {
                        if value <= 0 {
                            return Err(bad_argument_error(
                                "notes_iter",
                                "count",
                                1,
                                "expecting a number > 0",
                            ));
                        }
                        max_notes = value as usize;
                    } else {
                        return Err(bad_argument_error(
                            "notes_iter",
                            "count",
                            1,
                            "expecting a number or nil",
                        ));
                    }
                }
                lua.create_function_mut({
                    let mut iter = this.notes_iter().enumerate();
                    move |_lua: &Lua, _args: LuaMultiValue| -> LuaResult<LuaValue> {
                        if let Some((index, note)) = iter.next() {
                            if index < max_notes {
                                Ok(LuaValue::Integer(note as LuaInteger))
                            } else {
                                Ok(LuaNil)
                            }
                        } else {
                            Ok(LuaNil)
                        }
                    }
                })
            },
        );
        methods.add_method(
            "degree",
            |_lua, this, args: LuaMultiValue| -> LuaResult<LuaMultiValue> {
                let mut ret = LuaMultiValue::new();
                for (arg_index, arg) in args.iter().enumerate() {
                    let degree = note_degree_from_value(arg, arg_index)?;
                    if let Some(note) = this.notes_iter().nth(degree - 1) {
                        ret.push_back(LuaValue::Integer(u8::from(note) as LuaInteger));
                    }
                }
                Ok(ret)
            },
        );

        methods.add_method(
            "fit",
            |_lua, this, args: LuaMultiValue| -> LuaResult<LuaMultiValue> {
                let mut ret = LuaMultiValue::new();
                for (arg_index, arg) in args.iter().enumerate() {
                    if let Some(note_event) = note_event_from_value(arg, Some(arg_index))? {
                        let fit_note = this.transpose(note_event.note, 0);
                        ret.push_back(LuaValue::Integer(u8::from(fit_note) as LuaInteger));
                    } else {
                        ret.push_back(LuaValue::Nil);
                    }
                }
                Ok(ret)
            },
        );
    }
}

// --------------------------------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use crate::bindings::*;

    use pretty_assertions::assert_eq;

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
    fn scale() -> LuaResult<()> {
        let lua = new_test_engine()?;

        // scale_names ()
        assert!(lua
            .load(r#"scale_names()[1]"#)
            .eval::<LuaString>()
            .is_ok_and(|v| v.to_str().is_ok_and(|s| s == "chromatic")));

        // Scale (note, mode_name)
        assert!(lua
            .load(r#"scale("c", "wurst")"#)
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"scale("c", "harmonic minor")"#)
            .eval::<LuaValue>()
            .is_ok());
        assert_eq!(
            lua.load(r#"scale("c5", "natural major").notes"#)
                .eval::<Vec<LuaValue>>()
                .unwrap()
                .iter()
                .map(|v| v.as_i32().unwrap())
                .collect::<Vec<i32>>(),
            vec![60, 62, 64, 65, 67, 69, 71]
        );

        // Scale (note, interval)
        assert!(lua
            .load(r#"scale("c", {"wurst"})"#)
            .eval::<LuaValue>()
            .is_err());
        assert!(lua
            .load(r#"scale("c", {0,1,2,4,5,6,7,8,9,10,11})"#)
            .eval::<LuaValue>()
            .is_ok());
        assert_eq!(
            lua.load(r#"scale("c5", {0,3,5,7,10}).notes"#)
                .eval::<Vec<LuaValue>>()
                .unwrap()
                .iter()
                .map(|v| v.as_i32().unwrap())
                .collect::<Vec<i32>>(),
            vec![60, 63, 65, 67, 70]
        );
        Ok(())
    }

    #[test]
    fn scale_chord() -> LuaResult<()> {
        let lua = new_test_engine()?;

        assert!(lua
            .load(
                r#" local cmin = scale("c4", "minor")
                return cmin:chord("x", 3)"#
            )
            .eval::<Vec<LuaValue>>()
            .is_err());
        assert!(lua
            .load(
                r#"local cmin = scale("c4", "minor")
                return cmin:chord("x", 8)"#
            )
            .eval::<Vec<LuaValue>>()
            .is_err());

        assert_eq!(
            lua.load(
                r#"local cmin = scale("c4", "minor")
                return cmin:chord("i", 5)"#
            )
            .eval::<Vec<LuaValue>>()?
            .iter()
            .map(|v| v.as_i32().unwrap())
            .collect::<Vec<i32>>(),
            vec![48, 51, 55, 58, 62]
        );

        Ok(())
    }

    #[test]
    fn scale_degree() -> LuaResult<()> {
        let lua = new_test_engine()?;

        assert!(lua
            .load(
                r#" local cmin = scale("c4", "minor")
                return cmin:degree("x")"#
            )
            .eval::<LuaMultiValue>()
            .is_err());
        assert!(lua
            .load(
                r#"local cmin = scale("c4", "minor")
                return cmin:degree(8)"#
            )
            .eval::<LuaMultiValue>()
            .is_err());

        assert_eq!(
            lua.load(
                r#"local cmin = scale("c4", "minor")
                return cmin:degree("i", "iii", "v")"#
            )
            .eval::<LuaMultiValue>()?
            .iter()
            .map(|v| v.as_i32().unwrap())
            .collect::<Vec<i32>>(),
            vec![48, 51, 55]
        );

        Ok(())
    }

    #[test]
    fn scale_notes_iter() -> LuaResult<()> {
        let lua = new_test_engine()?;

        assert!(lua
            .load(r#"scale("c4", "minor"):notes_iter(0)"#)
            .exec()
            .is_err());
        assert!(lua
            .load(r#"scale("c4", "minor"):notes_iter(1)"#)
            .exec()
            .is_ok());

        assert_eq!(
            lua.load(
                r#"local cmin = scale("c4", "minor")
                local iter = cmin:notes_iter(3)
                return iter(), iter(), iter(), iter()
            "#
            )
            .eval::<LuaMultiValue>()?
            .iter()
            .map(|v| v.as_i32().unwrap_or(0))
            .collect::<Vec<i32>>(),
            vec![48, 50, 51, 0]
        );

        assert_eq!(
            lua.load(
                r#"local cmin = scale("f10", "minor")
                local iter = cmin:notes_iter()
                return iter(), iter(), iter()
            "#
            )
            .eval::<LuaMultiValue>()?
            .iter()
            .map(|v| v.as_i32().unwrap_or(0))
            .collect::<Vec<i32>>(),
            vec![125, 127, 0]
        );

        Ok(())
    }

    #[test]
    fn scale_fit() -> LuaResult<()> {
        let lua = new_test_engine()?;

        assert_eq!(
            lua.load(
                r#"local cmin = scale("c4", "minor")
                return cmin:fit("c4", 50, { key = 54 })"#
            )
            .eval::<LuaMultiValue>()?
            .iter()
            .map(|v| v.as_i32().unwrap())
            .collect::<Vec<i32>>(),
            vec![48, 50, 53]
        );

        Ok(())
    }
}
