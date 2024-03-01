use std::{collections::HashMap, path::Path};

use figment::{
    providers::{Env, Format, Serialized, Toml},
    Figment,
};
use miette::{Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use which::which;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SiloConfig {
    /// Diff tool used to display file differences
    pub diff_tool: String,
    /// Context values for handlebars that are available globally under the `ctx` variable
    pub template_context: HashMap<String, toml::Value>,
}

impl Default for SiloConfig {
    fn default() -> Self {
        Self {
            diff_tool: detect_difftool(),
            template_context: HashMap::new(),
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
    Figment::from(Serialized::defaults(SiloConfig::default()))
        .merge(Toml::file(dirs::config_dir().unwrap().join("silo.toml")))
        .merge(Toml::file(repo.join("repo.toml")))
        .merge(Toml::file(repo.join("repo.local.toml")))
        .merge(Env::prefixed("SILO_"))
        .extract()
        .into_diagnostic()
        .context("parsing config file")
}
