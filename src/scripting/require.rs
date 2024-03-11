use mlua::{Lua, Result, Table};

use super::log_module::log_module;
use super::silo_module::silo_module;
use super::utils_module::utils_module;

pub fn register_require(lua: &Lua) -> Result<()> {
    let globals = lua.globals();
    let old_require: mlua::Function = globals.get("require")?;
    globals.set("old_require", old_require)?;
    globals.set("require", lua.create_function(lua_require)?)?;

    Ok(())
}

fn lua_require(lua: &Lua, module: String) -> Result<Table<'_>> {
    match module.as_str() {
        "silo" => silo_module(lua),
        "log" => log_module(lua),
        "utils" => utils_module(lua),
        _ => {
            let old_require: mlua::Function = lua.globals().get("old_require")?;
            old_require.call(module)
        }
    }
}
