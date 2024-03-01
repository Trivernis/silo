mod contents;

use globset::GlobSet;
use miette::{bail, IntoDiagnostic, Result};

use std::{env, path::Path};

use crate::config::{read_config, SiloConfig};

use self::contents::Contents;

#[derive(Clone, Debug)]
pub struct SiloRepo {
    pub config: SiloConfig,
    contents: Contents,
}

impl SiloRepo {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.try_exists().into_diagnostic()? {
            bail!("The repository {path:?} does not exist");
        }
        let config = read_config(path)?;
        let pctx = ParseContext::new(GlobSet::empty(), config.clone());

        Ok(Self {
            contents: Contents::parse(pctx, path.to_owned())?,
            config,
        })
    }

    pub fn apply(&self) -> Result<()> {
        let cwd = dirs::home_dir().unwrap_or(env::current_dir().into_diagnostic()?);
        let ctx = ApplyContext {
            config: self.config.clone(),
        };
        self.contents.apply(&ctx, &cwd)
    }
}

pub struct ParseContext {
    ignored: GlobSet,
    config: SiloConfig,
}

impl ParseContext {
    pub fn new(ignored: GlobSet, config: SiloConfig) -> Self {
        Self { ignored, config }
    }
}

pub struct ApplyContext {
    config: SiloConfig,
}
