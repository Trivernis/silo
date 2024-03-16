use std::{collections::HashMap, fs, path::Path};

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use miette::{Context, IntoDiagnostic, Result};
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};
use which::which;

use crate::{scripting::create_lua, utils::Describe};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SiloConfig {
    /// Diff tool used to display file differences
    pub diff_tool: String,
    /// Additional config options
    #[serde(flatten)]
    pub userdata: HashMap<String, toml::Value>,
}

impl Default for SiloConfig {
    fn default() -> Self {
        Self {
            diff_tool: detect_difftool(),
            userdata: HashMap::new(),
        }
    }
}

fn detect_difftool() -> String {
    ["difft", "delta", "diff"]
        .into_iter()
        .filter(|t| which(t).is_ok())
        .map(String::from)
        .next()
        .unwrap_or_else(|| String::from("diff"))
}

/// Read the configuration file from the user config directory
/// with overrides from the `repo.toml` file
/// and the `repo.local.toml` config file
/// and environment variables prefixed with `SILO_``
pub fn read_config(repo: &Path) -> Result<SiloConfig> {
    let conf_dir = dirs::config_dir().unwrap();
    let default_config = conf_dir.join("silo.config.lua");
    let old_config = conf_dir.join("silo.toml");

    if !default_config.exists() {
        let mut lines = vec![
            "local silo = require 'silo'".to_owned(),
            "local utils = require 'utils'".to_owned(),
            "local config = silo.default_config".to_owned(),
        ];
        if old_config.exists() {
            lines.push("".to_owned());
            lines.push("-- merge with old toml config".to_owned());
            lines.push(format!(
                "config = utils.merge(config, utils.load_toml {old_config:?})"
            ));
            lines.push("config = utils.merge(config, config.template_context)".to_string())
        }
        lines.push("-- Changes can be added to the `config` object".to_owned());
        lines.push("".to_owned());
        lines.push("return config".to_owned());

        fs::write(&default_config, lines.join("\n")).describe("Writing default config")?
    }

    let mut builder = Figment::from(Serialized::defaults(SiloConfig::default()))
        .merge(Toml::file(old_config))
        .merge(Toml::file(repo.join("repo.toml")))
        .merge(Toml::file(repo.join("repo.local.toml")));

    let repo_defaults = repo.join("silo.config.lua");

    if repo_defaults.exists() {
        builder = builder.merge(Serialized::globals(read_lua_config(&repo_defaults)?))
    }

    builder
        .merge(Serialized::globals(read_lua_config(&default_config)?))
        .merge(Env::prefixed("SILO_"))
        .extract()
        .into_diagnostic()
        .context("parsing config file")
}

fn read_lua_config(path: &Path) -> Result<SiloConfig> {
    let lua = create_lua(&SiloConfig::default())?;
    let result = lua
        .load(path)
        .eval()
        .with_describe(|| format!("evaluating config script {path:?}"))?;
    let cfg = lua
        .from_value(result)
        .describe("deserializing lua config value")?;

    Ok(cfg)
}
