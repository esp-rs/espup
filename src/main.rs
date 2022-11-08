use anyhow::Result;
use clap::Parser;
use espup::{install, uninstall, update, SubCommand};

#[derive(Parser)]
#[command(
    name = "espup",
    bin_name = "espup",
    version,
    about,
    arg_required_else_help(true)
)]
struct Cli {
    #[command(subcommand)]
    subcommand: SubCommand,
}

fn main() -> Result<()> {
    match Cli::parse().subcommand {
        SubCommand::Install(args) => install(*args),
        SubCommand::Update(args) => update(args),
        SubCommand::Uninstall(args) => uninstall(args),
    }
}
