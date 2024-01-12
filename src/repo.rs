use handlebars::Handlebars;
use miette::{bail, Context, IntoDiagnostic, Result};
use serde::Deserialize;
use serde_json::json;
use std::{
    env,
    fs::{self},
    path::{Path, PathBuf},
};

use crate::templating;

#[derive(Clone, Debug)]
pub struct SiloRepo {
    pub root: DirEntry,
}

impl SiloRepo {
    pub fn open(path: &Path) -> Result<Self> {
        if !path.try_exists().into_diagnostic()? {
            bail!("The repository {path:?} does not exist");
        }

        Ok(Self {
            root: DirEntry::parse(path.to_owned())?,
        })
    }

    pub fn apply(&self) -> Result<()> {
        let cwd = env::current_dir()
            .into_diagnostic()
            .context("get current dir")?;
        self.root.apply(&cwd)
    }
}

#[derive(Clone, Debug)]
pub enum DirEntry {
    File(FileEntry),
    Dir(PathBuf, Vec<DirEntry>),
    Root(PathBuf, RootDirData, Vec<DirEntry>),
}

impl DirEntry {
    fn parse(path: PathBuf) -> Result<Self> {
        if path.is_dir() {
            let meta_file = path.join("dir.toml");
            let mut children = Vec::new();

            for read_entry in fs::read_dir(&path).into_diagnostic()? {
                let read_entry = read_entry.into_diagnostic()?;
                children.push(DirEntry::parse(read_entry.path())?);
            }

            if meta_file.exists() {
                let metadata = RootDirData::read(&meta_file)?;
                Ok(Self::Root(path, metadata, children))
            } else {
                Ok(Self::Dir(path, children))
            }
        } else {
            Ok(Self::File(FileEntry::parse(path)?))
        }
    }

    fn apply(&self, cwd: &Path) -> Result<()> {
        match self {
            DirEntry::File(file) => file.apply(cwd),
            DirEntry::Dir(p, children) => {
                let cwd = cwd.join(p.iter().last().unwrap());
                if !cwd.exists() {
                    fs::create_dir_all(&cwd)
                        .into_diagnostic()
                        .with_context(|| format!("Creating directory {cwd:?}"))?;
                }
                for child in children {
                    child.apply(&cwd)?;
                }
                Ok(())
            }
            DirEntry::Root(_, data, children) => {
                let engine = templating::engine();
                let rendered_path = engine
                    .render_template(&data.path, templating::context())
                    .into_diagnostic()
                    .with_context(|| format!("render template {}", data.path))?;

                let cwd = PathBuf::from(rendered_path);

                if !cwd.exists() {
                    fs::create_dir_all(&cwd)
                        .into_diagnostic()
                        .with_context(|| format!("Creating directory {cwd:?}"))?;
                }
                for child in children {
                    child.apply(&cwd)?;
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
    Metadata,
}

impl FileEntry {
    fn parse(path: PathBuf) -> Result<Self> {
        if let Some(true) = path.extension().map(|e| e == "tmpl") {
            Ok(Self::Template(path))
        } else if path.file_name().unwrap() == "dir.toml" {
            Ok(Self::Metadata)
        } else {
            Ok(Self::Plain(path))
        }
    }

    fn apply(&self, cwd: &Path) -> Result<()> {
        match self {
            FileEntry::Template(path) => {
                let contents = fs::read_to_string(path).into_diagnostic()?;

                let rendered = templating::engine()
                    .render_template(&contents, templating::context())
                    .into_diagnostic()
                    .with_context(|| format!("rendering template {path:?}"))?;

                let path = path.with_extension("");
                let filename = path.file_name().unwrap();
                let dest = cwd.join(filename);
                fs::write(&dest, rendered)
                    .into_diagnostic()
                    .with_context(|| format!("write to destination {dest:?}"))?;
            }
            FileEntry::Plain(path) => {
                let filename = path.file_name().unwrap();
                let dest = cwd.join(filename);
                fs::copy(path, &dest)
                    .into_diagnostic()
                    .with_context(|| format!("copy {path:?} to {dest:?}"))?;
            }
            FileEntry::Metadata => {}
        }

        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct RootDirData {
    pub path: String,
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
}
