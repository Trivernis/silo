pub mod log_module;
mod require;
pub mod silo_module;
pub mod utils_module;

use miette::Result;
use mlua::{Lua, LuaSerdeExt};
use serde::Serialize;

use crate::utils::Describe;

pub fn create_lua<T: Serialize>(ctx: &T) -> Result<Lua> {
    let lua = Lua::new();
    {
        let globals = lua.globals();
        require::register_require(&lua).describe("registering custom require")?;
        globals
            .set(
                "silo_ctx",
                lua.to_value(ctx).describe("serializing context to lua")?,
            )
            .describe("registering silo context")?;
    }

    Ok(lua)
}
