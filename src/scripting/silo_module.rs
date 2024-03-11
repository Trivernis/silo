use mlua::{Lua, LuaSerdeExt, Result, Table};

use crate::{config::SiloConfig, templating::ContextData};

pub fn silo_module(lua: &Lua) -> Result<Table> {
    let silo_ctx = ContextData::default();
    let exports = lua.create_table()?;

    exports.set("dirs", lua.to_value(&silo_ctx.dirs)?)?;
    exports.set("flags", lua.to_value(&silo_ctx.flags)?)?;
    exports.set("system", lua.to_value(&silo_ctx.system)?)?;
    let config = lua.globals().get::<_, mlua::Value>("__silo_config")?;
    exports.set("config", config)?;
    exports.set("default_config", lua.to_value(&SiloConfig::default())?)?;

    Ok(exports)
}
