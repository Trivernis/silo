pub mod log_module;
mod require;
pub mod silo_module;
pub mod utils_module;

use miette::Result;
use mlua::{Lua, LuaSerdeExt};

use crate::{config::SiloConfig, utils::Describe};

pub fn create_lua(config: &SiloConfig) -> Result<Lua> {
    let lua = Lua::new();
    {
        let globals = lua.globals();
        require::register_require(&lua).describe("registering custom require")?;
        globals
            .set(
                "__silo_config",
                lua.to_value(config)
                    .describe("serializing context to lua")?,
            )
            .describe("registering silo context")?;
    }

    Ok(lua)
}
