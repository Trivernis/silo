use std::{fs, sync::atomic::AtomicBool};

use args::{Args, InitArgs};
use clap::Parser;
use gix::progress::Discard;
use miette::{Context, IntoDiagnostic, Result};
use repo::SiloRepo;

mod args;
mod config;
mod fs_access;
mod repo;
mod scripting;
mod templating;

pub(crate) mod utils;

fn main() -> Result<()> {
    let args: Args = Args::parse();
    init_logging(args.verbose);

    match &args.command {
        args::Command::Init(init_args) => init(&args, init_args)?,
        args::Command::Apply => apply(&args)?,
        args::Command::Context => {
            let repo = SiloRepo::open(&args.repo)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&templating::context(repo.config.template_context))
                    .into_diagnostic()?
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
        .filter_module("handlebars", log::LevelFilter::Error)
        .filter_module("rustls", log::LevelFilter::Error)
        .filter_module("reqwest", log::LevelFilter::Error)
        .init();
}

fn apply(args: &Args) -> Result<()> {
    let mut repo = SiloRepo::open(&args.repo)?;
    repo.apply()?;
    log::info!("Applied all configurations in {:?}", args.repo);

    Ok(())
}

fn init(args: &Args, init_args: &InitArgs) -> Result<()> {
    if let Some(remote) = init_args.remote.as_ref() {
        init_remote(args, init_args, remote)
    } else {
        init_local(args)
    }
}

fn init_local(args: &Args) -> Result<()> {
    if !args.repo.exists() {
        fs::create_dir_all(&args.repo)
            .into_diagnostic()
            .with_context(|| format!("creating folder for repository {:?}", args.repo))?;
    }
    let _gitrepo = gix::init(&args.repo)
        .into_diagnostic()
        .with_context(|| format!("initializing repository at {:?}", args.repo))?;
    fs::write(args.repo.join(".gitignore"), "repo.local.toml\n")
        .into_diagnostic()
        .context("adding default .gitignore")?;

    log::info!("Repo initialized at {:?}", args.repo);

    Ok(())
}

fn init_remote(args: &Args, _init_args: &InitArgs, remote: &str) -> Result<()> {
    log::info!("Cloning {remote} into {:?}", args.repo);
    let interrupt = AtomicBool::new(false);
    gix::prepare_clone(remote, &args.repo)
        .into_diagnostic()
        .context("clone repo")?
        .fetch_then_checkout(Discard, &interrupt)
        .into_diagnostic()
        .context("fetch repo")?
        .0
        .main_worktree(Discard, &interrupt)
        .into_diagnostic()
        .context("checkout main")?;
    log::info!("Repo initialized at {:?}", args.repo);
    Ok(())
}
