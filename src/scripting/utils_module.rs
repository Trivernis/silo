use std::fs;

use mlua::{Lua, LuaSerdeExt, Result, Table};

pub fn utils_module(lua: &Lua) -> Result<Table> {
    let exports = lua.create_table()?;

    exports.set("merge", lua.create_function(lua_merge)?)?;
    exports.set("load_toml", lua.create_function(lua_read_toml)?)?;

    Ok(exports)
}

fn lua_merge<'a>(lua: &'a Lua, (a, b): (mlua::Value, mlua::Value)) -> Result<mlua::Value<'a>> {
    let val_a: serde_json::Value = lua.from_value(a)?;
    let val_b: serde_json::Value = lua.from_value(b)?;
    let merged = merge_struct::merge(&val_a, &val_b).map_err(mlua::Error::external)?;

    lua.to_value(&merged)
}

fn lua_read_toml<'a>(lua: &'a Lua, path: String) -> Result<mlua::Value<'a>> {
    let contents = fs::read_to_string(path)?;
    let toml_value: toml::Value = toml::from_str(&contents).map_err(mlua::Error::external)?;

    lua.to_value(&toml_value)
}
