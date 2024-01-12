use std::fs;

use args::Args;
use clap::Parser;
use miette::{Context, IntoDiagnostic, Result};
use repo::SiloRepo;

mod args;
mod repo;
mod templating;

fn main() -> Result<()> {
    let args: Args = Args::parse();
    match &args.command {
        args::Command::Init => init(&args)?,
        args::Command::Apply => apply(&args)?,
    }

    Ok(())
}

fn apply(args: &Args) -> Result<()> {
    let repo = SiloRepo::open(&args.repo)?;
    repo.apply()?;

    Ok(())
}

fn init(args: &Args) -> Result<()> {
    if !args.repo.exists() {
        fs::create_dir_all(&args.repo)
            .into_diagnostic()
            .with_context(|| format!("creating folder for repository {:?}", args.repo))?;
    }
    let _gitrepo = git2::Repository::init(&args.repo)
        .into_diagnostic()
        .with_context(|| format!("initializing repository at {:?}", args.repo))?;
    Ok(())
}
