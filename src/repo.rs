use globset::{Glob, GlobSet, GlobSetBuilder};
use miette::{bail, Context, IntoDiagnostic, Result};
use serde::Deserialize;
use std::{
    env,
    fs::{self},
    path::{Path, PathBuf},
    rc::Rc,
};

use crate::{
    config::{read_config, SiloConfig},
    templating,
};
use lazy_static::lazy_static;

#[derive(Clone, Debug)]
pub struct SiloRepo {
    pub root: DirEntry,
    pub config: SiloConfig,
}

impl SiloRepo {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.try_exists().into_diagnostic()? {
            bail!("The repository {path:?} does not exist");
        }

        Ok(Self {
            config: read_config(path)?,
            root: DirEntry::parse(Rc::new(ParseContext::default()), path.to_owned())?,
        })
    }

    pub fn apply(&self) -> Result<()> {
        let cwd = env::current_dir()
            .into_diagnostic()
            .context("get current dir")?;
        let ctx = ApplyContext {
            config: self.config.clone(),
        };
        self.root.apply(&ctx, &cwd)
    }
}

pub struct ParseContext {
    ignored: GlobSet,
}

impl ParseContext {
    pub fn new(ignored: GlobSet) -> Self {
        Self { ignored }
    }
}

impl Default for ParseContext {
    fn default() -> Self {
        Self {
            ignored: GlobSet::empty(),
        }
    }
}

pub struct ApplyContext {
    config: SiloConfig,
}

lazy_static! {
    static ref IGNORED_PATHS: GlobSet = GlobSetBuilder::new()
        .add(Glob::new("**/.git").unwrap())
        .add(Glob::new("**/dir.{toml,toml.tmpl}").unwrap())
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
    fn parse(mut context: Rc<ParseContext>, path: PathBuf) -> Result<Self> {
        if path.is_dir() {
            log::debug!("Parsing directory {path:?}");

            let meta_file = path.join("dir.toml");
            let meta_tmpl = path.join("dir.toml.tmpl");

            let metadata = if meta_file.exists() {
                log::debug!("Found metadata file");
                let metadata = RootDirData::read(&meta_file)?;
                context = Rc::new(ParseContext::new(metadata.ignored.clone()));

                Some(metadata)
            } else if meta_tmpl.exists() {
                log::debug!("Found metadata template");
                let metadata = RootDirData::read_template(&meta_tmpl)?;
                context = Rc::new(ParseContext::new(metadata.ignored.clone()));

                Some(metadata)
            } else {
                log::debug!("Directory is child");
                None
            };

            let mut children = Vec::new();

            for read_entry in fs::read_dir(&path).into_diagnostic()? {
                let read_entry = read_entry.into_diagnostic()?;
                let entry_path = read_entry.path();
                let test_path = entry_path.strip_prefix(&path).into_diagnostic()?;

                if !IGNORED_PATHS.is_match(&test_path) && !context.ignored.is_match(&test_path) {
                    children.push(DirEntry::parse(context.clone(), entry_path)?);
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

    fn apply(&self, ctx: &ApplyContext, cwd: &Path) -> Result<()> {
        match self {
            DirEntry::File(file) => file.apply(ctx, cwd),
            DirEntry::Dir(p, children) => {
                let cwd = if p != cwd {
                    let cwd = cwd.join(p.file_name().unwrap());
                    if !cwd.exists() {
                        log::info!("Creating {cwd:?}");
                        fs::create_dir_all(&cwd)
                            .into_diagnostic()
                            .with_context(|| format!("Creating directory {cwd:?}"))?;
                    }
                    cwd
                } else {
                    p.to_owned()
                };
                for child in children {
                    child.apply(ctx, &cwd)?;
                }
                Ok(())
            }
            DirEntry::Root(_, data, children) => {
                let engine = templating::engine();
                let rendered_path = engine
                    .render_template(
                        &data.path,
                        &templating::context(&ctx.config.template_context),
                    )
                    .into_diagnostic()
                    .with_context(|| format!("render template {}", data.path))?;

                let cwd = PathBuf::from(rendered_path);

                if !cwd.exists() {
                    log::info!("Creating {cwd:?}");
                    fs::create_dir_all(&cwd)
                        .into_diagnostic()
                        .with_context(|| format!("Creating directory {cwd:?}"))?;
                }
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

    fn apply(&self, ctx: &ApplyContext, cwd: &Path) -> Result<()> {
        match self {
            FileEntry::Template(path) => {
                log::debug!("Processing template {path:?}");
                let contents = fs::read_to_string(path).into_diagnostic()?;

                let rendered = templating::engine()
                    .render_template(
                        &contents,
                        &templating::context(&ctx.config.template_context),
                    )
                    .into_diagnostic()
                    .with_context(|| format!("rendering template {path:?}"))?;

                let new_path = path.with_extension("");
                let filename = new_path.file_name().unwrap();
                let dest = cwd.join(filename);
                log::info!("Writing {path:?} -> {dest:?}");

                fs::write(&dest, rendered)
                    .into_diagnostic()
                    .with_context(|| format!("write to destination {dest:?}"))?;
            }
            FileEntry::Plain(path) => {
                let filename = path.file_name().unwrap();
                let dest = cwd.join(filename);
                log::info!("Copying {path:?} -> {dest:?}");

                fs::copy(path, &dest)
                    .into_diagnostic()
                    .with_context(|| format!("copy {path:?} to {dest:?}"))?;
            }
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RootDirData {
    pub path: String,
    #[serde(default)]
    pub ignored: GlobSet,
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

    fn read_template(path: &Path) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .into_diagnostic()
            .with_context(|| format!("reading metadata file {path:?}"))?;
        let rendered = templating::engine()
            .render_template(&contents, &templating::context(()))
            .into_diagnostic()
            .with_context(|| format!("processing template {path:?}"))?;
        toml::from_str(&rendered)
            .into_diagnostic()
            .with_context(|| format!("parsing metadata file {path:?}"))
    }
}
