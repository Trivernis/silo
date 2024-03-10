use std::{
    fs::{self},
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{scripting::create_lua, templating, utils::Describe};

use super::{ApplyContext, ParseContext, ReadMode};
use globset::{Glob, GlobSet, GlobSetBuilder};
use lazy_static::lazy_static;
use miette::{Context, IntoDiagnostic, Result};
use mlua::LuaSerdeExt;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Contents {
    pub root: DirEntry,
}

impl Contents {
    pub fn parse(pctx: ParseContext, path: PathBuf) -> Result<Self> {
        let root = DirEntry::parse(Rc::new(pctx), path.to_owned())?;
        Ok(Self { root })
    }

    pub fn apply(&self, actx: &mut ApplyContext, cwd: &Path) -> Result<()> {
        self.root.apply(actx, cwd)
    }
}

lazy_static! {
    static ref IGNORED_PATHS: GlobSet = GlobSetBuilder::new()
        .add(Glob::new("**/.git").unwrap())
        .add(Glob::new("**/dir.{toml,toml.tmpl}").unwrap())
        .add(Glob::new("**/silo.{dir,config}.lua").unwrap())
        .build()
        .unwrap();
}

#[derive(Clone, Debug)]
pub enum DirEntry {
    File(FileEntry),
    Dir(PathBuf, Vec<DirEntry>),
    Root(PathBuf, RootDirData, Vec<DirEntry>),
}

impl DirEntry {
    fn parse(mut ctx: Rc<ParseContext>, path: PathBuf) -> Result<Self> {
        if path.is_dir() {
            log::debug!("Parsing directory {path:?}");

            let meta_file = path.join("dir.toml");
            let meta_tmpl = path.join("dir.toml.tmpl");
            let script_tmpl = path.join("silo.dir.lua");

            let metadata = if script_tmpl.exists() {
                log::debug!("Found script template");
                let metadata = RootDirData::read_lua(&script_tmpl, &ctx.config.template_context)?;
                ctx = Rc::new(ParseContext::new(
                    path.clone(),
                    metadata.read_mode(),
                    ctx.config.clone(),
                ));

                Some(metadata)
            } else if meta_file.exists() {
                log::debug!("Found metadata file");
                log::warn!("Old toml metadata files are deprecated. Please migrate to the `silo.dir.lua` syntax");
                let metadata = RootDirData::read(&meta_file)?;
                ctx = Rc::new(ParseContext::new(
                    path.clone(),
                    metadata.read_mode(),
                    ctx.config.clone(),
                ));

                Some(metadata)
            } else if meta_tmpl.exists() {
                log::debug!("Found metadata template");
                log::warn!("Old template metadata files are deprecated. Please migrate to the `silo.dir.lua` syntax");
                let metadata =
                    RootDirData::read_template(&meta_tmpl, &ctx.config.template_context)?;
                ctx = Rc::new(ParseContext::new(
                    path.clone(),
                    metadata.read_mode(),
                    ctx.config.clone(),
                ));

                Some(metadata)
            } else {
                log::debug!("Directory is child");
                None
            };

            let mut children = Vec::new();

            for read_entry in fs::read_dir(&path).into_diagnostic()? {
                let read_entry = read_entry.into_diagnostic()?;
                let entry_path = read_entry.path();
                let test_path = entry_path.strip_prefix(&ctx.base).into_diagnostic()?;

                if !IGNORED_PATHS.is_match(test_path) && ctx.is_included(test_path) {
                    children.push(DirEntry::parse(ctx.clone(), entry_path)?);
                } else {
                    log::debug!("Entry {entry_path:?} is ignored")
                }
            }

            if let Some(metadata) = metadata {
                Ok(Self::Root(path, metadata, children))
            } else {
                Ok(Self::Dir(path, children))
            }
        } else {
            log::debug!("Parsing file {path:?}");
            Ok(Self::File(FileEntry::parse(path)?))
        }
    }

    fn apply(&self, ctx: &mut ApplyContext, cwd: &Path) -> Result<()> {
        match self {
            DirEntry::File(file) => file.apply(ctx, cwd),
            DirEntry::Dir(p, children) => {
                let cwd = if p != cwd {
                    cwd.join(p.file_name().unwrap())
                } else {
                    p.to_owned()
                };
                for child in children {
                    child.apply(ctx, &cwd)?;
                }
                Ok(())
            }
            DirEntry::Root(_, data, children) => {
                let rendered_path = templating::render(&data.path, &ctx.config.template_context)?;
                let cwd = PathBuf::from(rendered_path);

                for child in children {
                    child.apply(ctx, &cwd)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum FileEntry {
    Template(PathBuf),
    Plain(PathBuf),
}

impl FileEntry {
    fn parse(path: PathBuf) -> Result<Self> {
        if let Some(true) = path.extension().map(|e| e == "tmpl") {
            log::debug!("File is template");
            Ok(Self::Template(path))
        } else {
            log::debug!("File is plain");
            Ok(Self::Plain(path))
        }
    }

    fn apply(&self, ctx: &mut ApplyContext, cwd: &Path) -> Result<()> {
        match self {
            FileEntry::Template(path) => {
                log::debug!("Processing template {path:?}");

                let contents = fs::read_to_string(path).into_diagnostic()?;
                let new_path = path.with_extension("");
                let filename = new_path.file_name().unwrap();

                let dest = cwd.join(filename);
                let render_contents = templating::render(&contents, &ctx.config.template_context)?;

                ctx.fs.write_all(&dest, &render_contents.into_bytes())?;
                ctx.fs
                    .set_permissions(&dest, fs::metadata(path).into_diagnostic()?.permissions())?;
            }
            FileEntry::Plain(path) => {
                let filename = path.file_name().unwrap();
                let dest = cwd.join(filename);
                ctx.fs.copy(path, &dest)?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RootDirData {
    pub path: String,
    #[serde(default)]
    pub mode: Mode,
    #[serde(default, alias = "ignored")]
    pub exclude: GlobSet,
    #[serde(default)]
    pub include: GlobSet,
}

#[derive(Clone, Debug, Deserialize)]
pub enum Mode {
    #[serde(alias = "include")]
    Include,
    #[serde(alias = "exclude")]
    Exclude,
}

impl Default for Mode {
    fn default() -> Self {
        Self::Exclude
    }
}

impl RootDirData {
    fn read(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .into_diagnostic()
            .with_context(|| format!("reading metadata file {path:?}"))?;
        toml::from_str(&contents)
            .into_diagnostic()
            .with_context(|| format!("parsing metadata file {path:?}"))
    }

    fn read_template<T: Serialize>(path: &Path, ctx: T) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .into_diagnostic()
            .with_context(|| format!("reading metadata file {path:?}"))?;
        let rendered = templating::render(&contents, ctx)?;
        toml::from_str(&rendered)
            .into_diagnostic()
            .with_context(|| format!("parsing metadata file {path:?}"))
    }

    fn read_lua<T: Serialize>(path: &Path, ctx: T) -> Result<Self> {
        let lua = create_lua(&ctx)?;
        let cfg: Self = lua
            .from_value(lua.load(path).eval().describe("evaluating script")?)
            .describe("deserialize lua value")?;

        Ok(cfg)
    }

    fn read_mode(&self) -> ReadMode {
        match self.mode {
            Mode::Include => ReadMode::Include(self.include.clone()),
            Mode::Exclude => ReadMode::Exclude(self.exclude.clone()),
        }
    }
}
