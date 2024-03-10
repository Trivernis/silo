use embed_nu::{CommandGroupConfig, Context};
use rusty_value::*;
use std::{
    fs, mem,
    path::{Path, PathBuf},
};

use miette::{IntoDiagnostic, Result};

#[derive(Clone, Debug)]
pub struct Hooks {
    scripts: Vec<HookScript>,
}

#[derive(Clone)]
pub struct HookScript {
    script: embed_nu::Context,
}

impl std::fmt::Debug for HookScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HookScript")
    }
}

#[derive(Clone, Debug, RustyValue)]
pub struct ApplyAllContext {
    pub repo: PathBuf,
    pub paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, RustyValue)]
pub struct ApplyEachContext {
    pub repo: PathBuf,
    pub src: PathBuf,
    pub dst: PathBuf,
}

impl Hooks {
    pub fn take(&mut self) -> Self {
        Hooks {
            scripts: mem::take(&mut self.scripts),
        }
    }

    pub fn parse(path: &Path) -> Result<Self> {
        log::debug!("Parsing hooks in {path:?}");
        let readdir = fs::read_dir(path).into_diagnostic()?;
        let mut scripts = Vec::new();

        for entry in readdir {
            let path = entry.into_diagnostic()?.path();

            if path.is_file() && path.extension().is_some_and(|e| e == "nu") {
                log::debug!("Found hook {path:?}");
                scripts.push(HookScript::parse(&path)?)
            }
        }

        Ok(Self { scripts })
    }

    pub fn before_apply_all(&mut self, ctx: ApplyAllContext) -> Result<()> {
        for script in &mut self.scripts {
            script.before_apply_all(ctx.clone())?;
        }

        Ok(())
    }

    pub fn after_apply_all(&mut self, ctx: ApplyAllContext) -> Result<()> {
        for script in &mut self.scripts {
            script.after_apply_all(ctx.clone())?;
        }

        Ok(())
    }

    pub fn before_apply_each(&mut self, ctx: ApplyEachContext) -> Result<()> {
        for script in &mut self.scripts {
            script.before_apply_each(ctx.clone())?;
        }

        Ok(())
    }

    pub fn after_apply_each(&mut self, ctx: ApplyEachContext) -> Result<()> {
        for script in &mut self.scripts {
            script.after_apply_each(ctx.clone())?;
        }

        Ok(())
    }

    pub(crate) fn empty() -> Hooks {
        Self {
            scripts: Vec::new(),
        }
    }
}

impl HookScript {
    pub fn parse(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path).into_diagnostic()?;

        let ctx = Context::builder()
            .with_command_groups(CommandGroupConfig::default().all_groups(true))
            .into_diagnostic()?
            .add_script(contents)
            .into_diagnostic()?
            .add_parent_env_vars()
            .build()
            .into_diagnostic()?;
        Ok(Self { script: ctx })
    }

    pub fn before_apply_all(&mut self, ctx: ApplyAllContext) -> Result<()> {
        if self.script.has_fn("before_apply_all") {
            let pipeline = self
                .script
                .call_fn("before_apply_all", [ctx])
                .into_diagnostic()?;
            self.script.print_pipeline(pipeline).into_diagnostic()?;
        } else {
            log::debug!("No `before_apply_all` in script");
        }

        Ok(())
    }

    pub fn after_apply_all(&mut self, ctx: ApplyAllContext) -> Result<()> {
        if self.script.has_fn("after_apply_all") {
            let pipeline = self
                .script
                .call_fn("after_apply_all", [ctx])
                .into_diagnostic()?;
            self.script.print_pipeline(pipeline).into_diagnostic()?;
        } else {
            log::debug!("No `after_apply_all` in script");
        }

        Ok(())
    }

    pub fn before_apply_each(&mut self, ctx: ApplyEachContext) -> Result<()> {
        if self.script.has_fn("before_apply_each") {
            let pipeline = self
                .script
                .call_fn("before_apply_each", [ctx])
                .into_diagnostic()?;
            self.script.print_pipeline(pipeline).into_diagnostic()?;
        } else {
            log::debug!("No `before_apply_each` in script");
        }

        Ok(())
    }

    pub fn after_apply_each(&mut self, ctx: ApplyEachContext) -> Result<()> {
        if self.script.has_fn("after_apply_each") {
            let pipeline = self
                .script
                .call_fn("after_apply_each", [ctx])
                .into_diagnostic()?;
            self.script.print_pipeline(pipeline).into_diagnostic()?;
        } else {
            log::debug!("No `after_apply_each` in script");
        }

        Ok(())
    }
}
