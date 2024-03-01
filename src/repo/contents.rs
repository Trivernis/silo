use std::{
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

use crate::templating;

use super::{ApplyContext, ParseContext};
use chksum::sha2_256::chksum;
use dialoguer::Confirm;
use globset::{Glob, GlobSet, GlobSetBuilder};
use lazy_static::lazy_static;
use miette::{Context, IntoDiagnostic, Result};
use serde::{Deserialize, Serialize};
use tempfile::NamedTempFile;

#[derive(Clone, Debug)]
pub struct Contents {
    pub root: DirEntry,
}

impl Contents {
    pub fn parse(pctx: ParseContext, path: PathBuf) -> Result<Self> {
        let root = DirEntry::parse(Rc::new(pctx), path.to_owned())?;
        Ok(Self { root })
    }

    pub fn apply(&self, actx: &ApplyContext, cwd: &Path) -> Result<()> {
        self.root.apply(actx, cwd)
    }
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
    fn parse(mut ctx: Rc<ParseContext>, path: PathBuf) -> Result<Self> {
        if path.is_dir() {
            log::debug!("Parsing directory {path:?}");

            let meta_file = path.join("dir.toml");
            let meta_tmpl = path.join("dir.toml.tmpl");

            let metadata = if meta_file.exists() {
                log::debug!("Found metadata file");
                let metadata = RootDirData::read(&meta_file)?;
                ctx = Rc::new(ParseContext::new(
                    metadata.ignored.clone(),
                    ctx.config.clone(),
                ));

                Some(metadata)
            } else if meta_tmpl.exists() {
                log::debug!("Found metadata template");
                let metadata =
                    RootDirData::read_template(&meta_tmpl, &ctx.config.template_context)?;
                ctx = Rc::new(ParseContext::new(
                    metadata.ignored.clone(),
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
                let test_path = entry_path.strip_prefix(&path).into_diagnostic()?;

                if !IGNORED_PATHS.is_match(&test_path) && !ctx.ignored.is_match(&test_path) {
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

    fn apply(&self, ctx: &ApplyContext, cwd: &Path) -> Result<()> {
        match self {
            DirEntry::File(file) => {
                ensure_cwd(cwd)?;
                file.apply(ctx, cwd)
            }
            DirEntry::Dir(p, children) => {
                let cwd = if p != cwd {
                    let cwd = cwd.join(p.file_name().unwrap());
                    ensure_cwd(&cwd)?;
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

    fn apply(&self, ctx: &ApplyContext, cwd: &Path) -> Result<()> {
        match self {
            FileEntry::Template(path) => {
                log::debug!("Processing template {path:?}");
                let contents = fs::read_to_string(path).into_diagnostic()?;

                let new_path = path.with_extension("");
                let filename = new_path.file_name().unwrap();
                let dest = cwd.join(filename);

                let render_contents = templating::render(&contents, &ctx.config.template_context)?;

                if confirm_changes(&ctx.config.diff_tool, &render_contents, &dest)? {
                    log::info!("Render {path:?} -> {dest:?}");
                    fs::write(&dest, render_contents)
                        .into_diagnostic()
                        .context("writing changes")?;
                } else {
                    log::info!("Skipping {path:?} !-> {dest:?}");
                }
            }
            FileEntry::Plain(path) => {
                let filename = path.file_name().unwrap();
                let dest = cwd.join(filename);

                if confirm_write(&ctx.config.diff_tool, path, &dest)? {
                    log::info!("Copying {path:?} -> {dest:?}");
                    fs::copy(path, &dest)
                        .into_diagnostic()
                        .with_context(|| format!("copy {path:?} to {dest:?}"))?;
                } else {
                    log::info!("Skipping {path:?} !-> {dest:?}");
                }
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

    fn read_template<T: Serialize>(path: &Path, ctx: T) -> Result<Self> {
        let contents = fs::read_to_string(path)
            .into_diagnostic()
            .with_context(|| format!("reading metadata file {path:?}"))?;
        let rendered = templating::render(&contents, ctx)?;
        toml::from_str(&rendered)
            .into_diagnostic()
            .with_context(|| format!("parsing metadata file {path:?}"))
    }
}

fn confirm_changes(diff_tool: &str, changes: &str, original: &Path) -> Result<bool> {
    let mut tmp = NamedTempFile::new()
        .into_diagnostic()
        .context("create tmp file")?;
    tmp.write_all(changes.as_bytes())
        .into_diagnostic()
        .context("write tmp file")?;
    confirm_write(diff_tool, &tmp.into_temp_path(), original)
}

fn confirm_write(diff_tool: &str, a: &Path, b: &Path) -> Result<bool> {
    if !b.exists() {
        return Ok(true);
    }
    let f1 = File::open(a)
        .into_diagnostic()
        .with_context(|| format!("opening file {a:?}"))?;
    let f2 = File::open(b)
        .into_diagnostic()
        .with_context(|| format!("opening file {b:?}"))?;

    if chksum(f1).into_diagnostic()?.as_bytes() == chksum(f2).into_diagnostic()?.as_bytes() {
        return Ok(true);
    }
    Command::new(diff_tool)
        .arg(b)
        .arg(a)
        .spawn()
        .into_diagnostic()
        .context("spawn diff tool")?
        .wait()
        .into_diagnostic()
        .context("wait for diff tool to exit")?;
    Confirm::new()
        .with_prompt("Do you want to apply these changes?")
        .interact()
        .into_diagnostic()
}

fn ensure_cwd(cwd: &Path) -> Result<(), miette::ErrReport> {
    if cwd.exists() {
        return Ok(());
    }
    log::info!("Creating {cwd:?}");
    fs::create_dir_all(&cwd)
        .into_diagnostic()
        .with_context(|| format!("Creating directory {cwd:?}"))
}
