use args::Args;
use clap::Parser;
use miette::Result;
use repo::SiloRepo;

mod args;
mod repo;
mod templating;

fn main() -> Result<()> {
    let args: Args = Args::parse();
    match &args.command {
        args::Command::Init => todo!(),
        args::Command::Apply => apply(&args)?,
    }

    Ok(())
}

fn apply(args: &Args) -> Result<()> {
    let repo = SiloRepo::open(&args.repo)?;
    dbg!(&repo);
    repo.apply()?;

    Ok(())
}
