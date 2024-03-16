use std::path::PathBuf;

use mlua::{Lua, Result, Table};

struct PathModule;

macro_rules! module {
    ($lua:expr, $($fn_name:expr => $fn: expr),+,) => {
        {
            let table = $lua.create_table()?;
            $(
                table.set($fn_name, $lua.create_function($fn)?)?;
            )+
            table
        }
    };
    ($lua:expr, $($fn_name:expr => $fn: expr),+) => {
        module!($lua, $($fn_name => $fn),+,)
    }
}

/// Utility functions
pub fn path_module(lua: &Lua) -> Result<Table> {
    let exports = module!(lua,
        "join" => PathModule::join,
        "exists" => PathModule::exists,
    );

    Ok(exports)
}

impl PathModule {
    fn join(_lua: &Lua, paths: Vec<String>) -> Result<String> {
        Ok(PathBuf::from_iter(paths).to_string_lossy().to_string())
    }

    fn exists(_lua: &Lua, path: String) -> Result<bool> {
        let path = PathBuf::from(path);

        Ok(path.exists())
    }
}
