mod contents;
pub(crate) mod hooks;

use globset::GlobSet;
use miette::{bail, IntoDiagnostic, Result};

use std::{
    env,
    path::{Path, PathBuf},
};

use crate::{
    config::{read_config, SiloConfig},
    fs_access::{BufferedFsAccess, FsAccess},
};

use self::{contents::Contents, hooks::Hooks};

#[derive(Clone, Debug)]
pub struct SiloRepo {
    pub config: SiloConfig,
    repo: PathBuf,
    contents: Contents,
    hooks: Hooks,
}

impl SiloRepo {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.try_exists().into_diagnostic()? {
            bail!("The repository {path:?} does not exist");
        }
        let config = read_config(path)?;
        let pctx = ParseContext::new(
            path.to_owned(),
            ReadMode::Exclude(GlobSet::empty()),
            config.clone(),
        );
        let content_path = path.join("content");

        if !content_path.exists() {
            bail!("No content stored in this dotfiles repo");
        }
        let hook_path = path.join("hooks");

        let hooks = if hook_path.exists() {
            Hooks::parse(&hook_path)?
        } else {
            Hooks::empty()
        };

        Ok(Self {
            contents: Contents::parse(pctx, content_path)?,
            repo: path.to_owned(),
            config,
            hooks,
        })
    }

    pub fn apply(&mut self) -> Result<()> {
        let cwd = dirs::home_dir().unwrap_or(env::current_dir().into_diagnostic()?);
        let fs_access: Box<dyn FsAccess> = Box::new(BufferedFsAccess::new(
            self.repo.clone(),
            self.config.diff_tool.to_owned(),
            self.hooks.take(),
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
    mode: ReadMode,
    config: SiloConfig,
    base: PathBuf,
}

pub enum ReadMode {
    Include(GlobSet),
    Exclude(GlobSet),
}

impl ParseContext {
    pub fn new(base: PathBuf, mode: ReadMode, config: SiloConfig) -> Self {
        Self { mode, config, base }
    }

    pub fn is_included(&self, path: &Path) -> bool {
        match &self.mode {
            ReadMode::Include(i) => i.is_match(path),
            ReadMode::Exclude(e) => !e.is_match(path),
        }
    }
}

pub struct ApplyContext {
    config: SiloConfig,
    fs: Box<dyn FsAccess>,
}
