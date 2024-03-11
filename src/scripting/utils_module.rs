use std::{
    fs,
    process::{Command, Stdio},
};

use mlua::{Function, Lua, LuaSerdeExt, Result, Table};
use serde::Serialize;

/// Utility functions
pub fn utils_module(lua: &Lua) -> Result<Table> {
    let exports = lua.create_table()?;

    exports.set("merge", lua.create_function(lua_merge)?)?;
    exports.set("from_json", lua.create_function(lua_from_json)?)?;
    exports.set("load_json", lua.create_function(lua_load_json)?)?;
    exports.set("from_toml", lua.create_function(lua_from_toml)?)?;
    exports.set("load_toml", lua.create_function(lua_load_toml)?)?;
    exports.set("ext", lua.create_function(lua_ext)?)?;
    exports.set("ext_piped", lua.create_function(lua_ext_piped)?)?;

    if let Ok(nu_path) = which::which("nu") {
        exports.set(
            "nu",
            lua.create_function(move |lua, expr| {
                let output = Command::new(&nu_path)
                    .arg("-c")
                    .arg::<String>(expr)
                    .stdout(Stdio::piped())
                    .spawn()?
                    .wait_with_output()?;

                let output_string =
                    String::from_utf8(output.stdout).map_err(mlua::Error::external)?;

                lua.to_value(&output_string)
            })?,
        )?;
    }

    Ok(exports)
}

/// Merges two values
fn lua_merge<'a>(lua: &'a Lua, (a, b): (mlua::Value, mlua::Value)) -> Result<mlua::Value<'a>> {
    let val_a: serde_json::Value = lua.from_value(a)?;
    let val_b: serde_json::Value = lua.from_value(b)?;
    let merged = merge_struct::merge(&val_a, &val_b).map_err(mlua::Error::external)?;

    lua.to_value(&merged)
}

/// Parse a json string into a lua value
fn lua_from_json<'a>(lua: &'a Lua, json_string: String) -> Result<mlua::Value<'a>> {
    let toml_value: serde_json::Value =
        serde_json::from_str(&json_string).map_err(mlua::Error::external)?;

    lua.to_value(&toml_value)
}

/// Reads a json file and parses it as a lua value
fn lua_load_json<'a>(lua: &'a Lua, path: String) -> Result<mlua::Value<'a>> {
    let contents = fs::read_to_string(path)?;
    lua_from_json(lua, contents)
}

/// Parse a toml string into a lua value
fn lua_from_toml<'a>(lua: &'a Lua, toml_string: String) -> Result<mlua::Value<'a>> {
    let toml_value: toml::Value = toml::from_str(&toml_string).map_err(mlua::Error::external)?;

    lua.to_value(&toml_value)
}

/// Reads a toml file and parses it as a lua value
fn lua_load_toml<'a>(lua: &'a Lua, path: String) -> Result<mlua::Value<'a>> {
    let contents = fs::read_to_string(path)?;
    lua_from_toml(lua, contents)
}

/// Creates a new executable that can be called with a variable number of args
fn lua_ext<'a>(lua: &'a Lua, program: String) -> Result<Function<'a>> {
    lua.create_function(move |_lua, args| {
        let exit_status = Command::new(&program)
            .args::<Vec<String>, _>(args)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?
            .wait()?;
        if exit_status.success() {
            Ok(())
        } else {
            Err(mlua::Error::external(format!(
                "External command `{program}` failed"
            )))
        }
    })
}

#[derive(Serialize)]
struct CommandOutput {
    code: i32,
    stdout: String,
    stderr: String,
}

/// Creates a new executable that can be called with a variable number of args
fn lua_ext_piped<'a>(lua: &'a Lua, program: String) -> Result<Function<'a>> {
    lua.create_function(move |lua, args| {
        let cmd = Command::new(&program)
            .args::<Vec<String>, _>(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        let output = cmd.wait_with_output()?;
        let output = CommandOutput {
            code: output.status.code().unwrap_or(0),
            stdout: String::from_utf8(output.stdout).map_err(mlua::Error::external)?,
            stderr: String::from_utf8(output.stderr).map_err(mlua::Error::external)?,
        };

        lua.to_value(&output)
    })
}
