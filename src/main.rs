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
    init_logging(args.verbose);

    match &args.command {
        args::Command::Init => init(&args)?,
        args::Command::Apply => apply(&args)?,
        args::Command::Context => {
            println!(
                "{}",
                serde_json::to_string_pretty(templating::context()).into_diagnostic()?
            )
        }
        args::Command::Repo => {
            println!("{}", args.repo.to_string_lossy())
        }
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    let mut builder = pretty_env_logger::formatted_builder();
    let builder = if verbose {
        builder.filter_level(log::LevelFilter::Debug)
    } else {
        builder.filter_level(log::LevelFilter::Info)
    };
    builder
        .filter_module("handlebars", log::LevelFilter::Off)
        .init();
}

fn apply(args: &Args) -> Result<()> {
    let repo = SiloRepo::open(&args.repo)?;
    repo.apply()?;
    log::info!("Applied all configurations in {:?}", args.repo);

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
    log::info!("Repo initialized at {:?}", args.repo);

    Ok(())
}
