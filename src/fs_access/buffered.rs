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

use super::FsAccess;

pub struct BufferedFsAccess {
    mappings: Vec<(NamedTempFile, PathBuf)>,
    diff_tool: String,
}

impl BufferedFsAccess {
    pub fn with_difftool(diff_tool: String) -> Self {
        Self {
            mappings: Vec::new(),
            diff_tool,
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

    fn persist(&mut self) -> Result<()> {
        let mappings = mem::take(&mut self.mappings);
        let mut drop_list = Vec::new();

        for (tmp, dst) in mappings {
            if confirm_write(&self.diff_tool, tmp.path(), &dst)? {
                ensure_parent(dst.parent().unwrap())?;
                fs::copy(tmp.path(), &dst)
                    .into_diagnostic()
                    .with_context(|| format!("copying {:?} to {dst:?}", tmp.path()))?;
                log::info!("Updated {dst:?}");
            } else {
                log::info!("Skipping {dst:?}");
            }
            drop_list.push(tmp);
        }
        mem::drop(drop_list);

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
