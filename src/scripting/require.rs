use mlua::{Lua, Result, Table};

use super::log_module::log_module;
use super::silo_module::silo_module;

pub fn register_require(lua: &Lua) -> Result<()> {
    let globals = lua.globals();
    let old_require: mlua::Function = globals.get("require")?;
    globals.set("old_require", old_require)?;
    globals.set("require", lua.create_function(lua_require)?)?;

    Ok(())
}

fn lua_require<'a>(lua: &'a Lua, module: String) -> Result<Table<'a>> {
    match module.as_str() {
        "silo" => silo_module(&lua),
        "log" => log_module(&lua),
        _ => {
            let old_require: mlua::Function = lua.globals().get("old_require")?;
            old_require.call(module)
        }
    }
}
