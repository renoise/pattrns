use std::{fs::read_to_string, path::Path};

use mlua::prelude::LuaResult;

// -------------------------------------------------------------------------------------------------

mod fennel;

// -------------------------------------------------------------------------------------------------

trait Transpiler {
    /// Read and transpile given file contents into a Lua source code string
    fn transpile_file<P: AsRef<Path>>(file_path: P) -> LuaResult<String> {
        let file_path = file_path.as_ref();
        Self::transpile(&read_to_string(file_path)?, file_path)
    }

    /// Transpile a source code string into a Lua source code string,
    /// using the given optional file path for tracebacks
    fn transpile<'a, P: Into<Option<&'a Path>>>(
        file_contents: &str,
        file_path: P,
    ) -> LuaResult<String>;
}

// -------------------------------------------------------------------------------------------------

/// File extensions which can be transpiled to Lua
pub(crate) fn transpilable_file_extensions() -> Vec<&'static str> {
    vec!["fnl"]
}

/// Check via the file extension if the file can be transpiled to Lua
pub(crate) fn is_transpilable_file<P: AsRef<Path>>(file_path: P) -> bool {
    file_path.as_ref().extension().is_some_and(|extension| {
        transpilable_file_extensions()
            .contains(&extension.to_string_lossy().to_ascii_lowercase().as_str())
    })
}

/// Convert/transpile Lua compatible language files to Lua
pub(crate) fn transpile<P: AsRef<Path>>(file_path: P) -> LuaResult<String> {
    let extension = file_path
        .as_ref()
        .extension()
        .map(|s| s.to_string_lossy())
        .unwrap_or("".into());
    if extension.eq_ignore_ascii_case("fnl") {
        fennel::FennelTranspiler::transpile_file(file_path)
    } else {
        Err(mlua::Error::runtime(format!(
            "Unexpected file extension for transpiler: '{}'. Supported extensions are: '{}'",
            extension,
            transpilable_file_extensions().join(",")
        )))
    }
}
