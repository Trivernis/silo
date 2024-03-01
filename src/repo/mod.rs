mod contents;

use globset::GlobSet;
use miette::{bail, IntoDiagnostic, Result};

use std::{env, path::Path};

use crate::{
    config::{read_config, SiloConfig},
    fs_access::{BufferedFsAccess, FsAccess},
};

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
        let fs_access: Box<dyn FsAccess> = Box::new(BufferedFsAccess::with_difftool(
            self.config.diff_tool.to_owned(),
        ));
        let mut ctx = ApplyContext {
            config: self.config.clone(),
            fs: fs_access,
        };
        self.contents.apply(&mut ctx, &cwd)?;
        ctx.fs.persist()
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
    fs: Box<dyn FsAccess>,
}
