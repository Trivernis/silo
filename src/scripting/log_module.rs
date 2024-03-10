use log::Level;
use mlua::{Error, Lua, Result, Table};

pub fn log_module(lua: &Lua) -> Result<Table> {
    let exports = lua.create_table()?;

    for level in [
        Level::Trace,
        Level::Debug,
        Level::Info,
        Level::Warn,
        Level::Error,
    ] {
        exports.set(
            level.as_str().to_lowercase(),
            lua.create_function(move |_, v| lua_log(v, level))?,
        )?;
    }

    Ok(exports)
}

fn lua_log(value: mlua::Value, level: log::Level) -> Result<()> {
    match level {
        Level::Error | Level::Warn | Level::Info => {
            log::log!(target: "lua", level, "{}", value.to_string()?)
        }
        Level::Debug | Level::Trace => log::log!(
            target: "lua",
            level,
            "{}",
            serde_json::to_string(&value).map_err(Error::external)?
        ),
    };
    Ok(())
}
