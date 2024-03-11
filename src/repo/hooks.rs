use mlua::{Lua, LuaSerdeExt, OwnedTable};
use serde::Serialize;
use std::{
    fs, mem,
    path::{Path, PathBuf},
    sync::Arc,
};

use miette::{IntoDiagnostic, Result};

use crate::{config::SiloConfig, scripting::create_lua, utils::Describe};

#[derive(Clone, Debug)]
pub struct Hooks {
    scripts: Vec<Arc<HookScript>>,
}

pub struct HookScript {
    lua: Lua,
    module: mlua::OwnedTable,
}

impl std::fmt::Debug for HookScript {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("HookScript")
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct ApplyAllContext {
    pub repo: PathBuf,
    pub paths: Vec<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
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

    pub fn load(config: &SiloConfig, path: &Path) -> Result<Self> {
        log::debug!("Parsing hooks in {path:?}");
        let readdir = fs::read_dir(path).into_diagnostic()?;
        let mut scripts = Vec::new();

        for entry in readdir {
            let path = entry.into_diagnostic()?.path();

            if path.is_file()
                && path
                    .file_name()
                    .is_some_and(|f| f.to_string_lossy().ends_with(".hook.lua"))
            {
                log::debug!("Found hook {path:?}");
                scripts.push(Arc::new(HookScript::load(config, &path)?))
            }
        }

        Ok(Self { scripts })
    }

    pub fn before_apply_all(&mut self, ctx: ApplyAllContext) -> Result<()> {
        for script in &self.scripts {
            script.before_apply_all(&ctx)?;
        }

        Ok(())
    }

    pub fn after_apply_all(&mut self, ctx: ApplyAllContext) -> Result<()> {
        for script in &self.scripts {
            script.after_apply_all(&ctx)?;
        }

        Ok(())
    }

    pub fn before_apply_each(&mut self, ctx: ApplyEachContext) -> Result<()> {
        for script in &self.scripts {
            script.before_apply_each(&ctx)?;
        }

        Ok(())
    }

    pub fn after_apply_each(&mut self, ctx: ApplyEachContext) -> Result<()> {
        for script in &self.scripts {
            script.after_apply_each(&ctx)?;
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
    pub fn load(config: &SiloConfig, path: &Path) -> Result<Self> {
        let lua = create_lua(&config)?;
        let module: OwnedTable = lua
            .load(path)
            .eval()
            .with_describe(|| format!("loading hook script {path:?}"))?;

        Ok(Self { lua, module })
    }

    pub fn before_apply_all(&self, ctx: &ApplyAllContext) -> Result<()> {
        self.call_function("before_apply_all", ctx)
    }

    pub fn after_apply_all(&self, ctx: &ApplyAllContext) -> Result<()> {
        self.call_function("after_apply_all", ctx)
    }

    pub fn before_apply_each(&self, ctx: &ApplyEachContext) -> Result<()> {
        self.call_function("before_apply_each", ctx)
    }

    pub fn after_apply_each(&self, ctx: &ApplyEachContext) -> Result<()> {
        self.call_function("after_apply_each", ctx)
    }

    fn call_function<S: Serialize>(&self, name: &str, ctx: &S) -> Result<()> {
        if let Ok(hook_fn) = self.module.to_ref().get::<_, mlua::Function<'_>>(name) {
            hook_fn
                .call(self.lua.to_value(&ctx).describe("Serializing context")?)
                .with_describe(|| format!("Calling hook script {name}"))?;
        } else {
            log::debug!("No `before_apply_all` in script");
        }

        Ok(())
    }
}
