use std::{
    fs::{self, File},
    io::Write,
    mem,
    path::{Path, PathBuf},
    process::Command,
};

use chksum::sha2_256::chksum;
use dialoguer::Confirm;

use miette::{Context, IntoDiagnostic, Result};

use tempfile::NamedTempFile;

use crate::repo::hooks::{ApplyAllContext, ApplyEachContext, Hooks};

use super::FsAccess;

pub struct BufferedFsAccess {
    repo: PathBuf,
    mappings: Vec<(NamedTempFile, PathBuf)>,
    diff_tool: String,
    hooks: Hooks,
}

impl BufferedFsAccess {
    pub fn new(repo: PathBuf, diff_tool: String, hooks: Hooks) -> Self {
        Self {
            mappings: Vec::new(),
            repo,
            diff_tool,
            hooks,
        }
    }
}

impl FsAccess for BufferedFsAccess {
    fn write_all(&mut self, dst: &std::path::Path, buf: &[u8]) -> miette::Result<()> {
        let mut tmp = tmpfile()?;
        tmp.write_all(buf).into_diagnostic().with_context(|| {
            format!(
                "writing {} bytes to file contents {:?}",
                buf.len(),
                tmp.path()
            )
        })?;
        self.mappings.push((tmp, dst.to_owned()));

        Ok(())
    }

    fn copy(&mut self, src: &std::path::Path, dst: &std::path::Path) -> miette::Result<()> {
        let tmp = tmpfile()?;
        fs::copy(src, tmp.path())
            .into_diagnostic()
            .with_context(|| format!("copying {src:?} to {:?}", tmp.path()))?;
        self.mappings.push((tmp, dst.to_owned()));

        Ok(())
    }

    fn set_permissions(&mut self, path: &Path, perm: fs::Permissions) -> Result<()> {
        let found_entry = self.mappings.iter().find(|(_, p)| p == path);

        if let Some(entry) = found_entry {
            fs::set_permissions(entry.0.path(), perm.clone())
                .into_diagnostic()
                .with_context(|| format!("Failed to set permissions {perm:?} on {path:?}"))?;
        }

        Ok(())
    }

    fn persist(&mut self) -> Result<()> {
        let mappings = mem::take(&mut self.mappings);
        let mut drop_list = Vec::new();
        let paths: Vec<_> = mappings.iter().map(|(_, p)| p.to_owned()).collect();

        self.hooks.before_apply_all(ApplyAllContext {
            repo: self.repo.clone(),
            paths: paths.clone(),
        })?;

        for (tmp, dst) in mappings {
            if confirm_write(&self.diff_tool, tmp.path(), &dst)? {
                ensure_parent(dst.parent().unwrap())?;

                self.hooks.before_apply_each(ApplyEachContext {
                    repo: self.repo.clone(),
                    src: tmp.path().to_owned(),
                    dst: dst.clone(),
                })?;

                fs::copy(tmp.path(), &dst)
                    .into_diagnostic()
                    .with_context(|| format!("copying {:?} to {dst:?}", tmp.path()))?;

                self.hooks.after_apply_each(ApplyEachContext {
                    repo: self.repo.clone(),
                    src: tmp.path().to_owned(),
                    dst: dst.clone(),
                })?;
                log::info!("Updated {dst:?}");
            } else {
                log::info!("Skipping {dst:?}");
            }
            drop_list.push(tmp);
        }
        mem::drop(drop_list);

        self.hooks.after_apply_all(ApplyAllContext {
            repo: self.repo.clone(),
            paths,
        })?;

        Ok(())
    }
}

fn tmpfile() -> Result<NamedTempFile> {
    NamedTempFile::new()
        .into_diagnostic()
        .context("failed to create tmp file")
}

fn confirm_write(diff_tool: &str, new: &Path, old: &Path) -> Result<bool> {
    if !old.exists() {
        return Ok(true);
    }
    let f1 = File::open(new)
        .into_diagnostic()
        .with_context(|| format!("opening file {new:?}"))?;
    let f2 = File::open(old)
        .into_diagnostic()
        .with_context(|| format!("opening file {old:?}"))?;

    if chksum(&f1).into_diagnostic()?.as_bytes() == chksum(&f2).into_diagnostic()?.as_bytes() {
        return Ok(true);
    }

    Command::new(diff_tool)
        .arg(old)
        .arg(new)
        .spawn()
        .into_diagnostic()
        .context("spawn diff tool")?
        .wait()
        .into_diagnostic()
        .context("wait for diff tool to exit")?;
    println!();

    Confirm::new()
        .with_prompt("Do you want to apply these changes?")
        .interact()
        .into_diagnostic()
}

fn ensure_parent(parent: &Path) -> Result<(), miette::ErrReport> {
    if parent.exists() {
        return Ok(());
    }
    log::info!("Creating {parent:?}");
    fs::create_dir_all(&parent)
        .into_diagnostic()
        .with_context(|| format!("Creating directory {parent:?}"))
}
