use std::{cell::RefCell, path::Path};

use mlua::prelude::*;

use super::Transpiler;

// -------------------------------------------------------------------------------------------------

pub(crate) struct FennelTranspiler {}

impl Transpiler for FennelTranspiler {
    fn transpile<'a, P: Into<Option<&'a Path>>>(
        file_contents: &str,
        _file_path: P,
    ) -> LuaResult<String> {
        // get cached compile function
        // NB: this will leak the entire interpreter instance which holds the function!
        thread_local! {
            static FENNEL: RefCell<LuaResult<(Lua, LuaFunction)>> = {
                let try_create = || -> LuaResult<(Lua, LuaFunction)> {
                    let lua = unsafe { Lua::unsafe_new_with(LuaStdLib::ALL, LuaOptions::default()) };
                    let fennel = lua.load(include_str!("./fennel.lua"))
                        .set_name("[inbuilt:fennel.lua]")
                        .call::<LuaTable>(())?;
                    let traceback_function = fennel.get::<LuaFunction>("traceback")?;
                    lua.globals()
                        .get::<LuaTable>("debug")?
                        .set("traceback", traceback_function)?;
                    let compile_function = fennel.get::<LuaFunction>("compileString")?;
                    Ok((lua, compile_function))
                };
                RefCell::new(try_create().map_err(|err|
                    LuaError::runtime(format!("Failed to load lua transpiler: {}", err))))
            };
        }
        let (_, compile_function) = FENNEL.with_borrow(|fennel| fennel.clone())?;
        // compile file
        let compile_options = LuaValue::Nil;
        let lua_code = compile_function
            .call::<LuaString>((file_contents, compile_options))
            .map_err(|err| LuaError::SyntaxError {
                message: match err {
                    LuaError::RuntimeError(str) => str.clone(),
                    _ => err.to_string(),
                },
                incomplete_input: false,
            })?;
        // return compiled code
        Ok(lua_code.to_string_lossy().to_string())
    }
}
