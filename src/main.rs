use clap::{CommandFactory, Parser};
#[cfg(windows)]
use espup::env::set_environment_variable;
use espup::{
    cli::{CompletionsOpts, InstallOpts, UninstallOpts},
    emoji,
    error::Error,
    logging::initialize_logger,
    toolchain::{
        gcc::uninstall_gcc_toolchains, install as toolchain_install, llvm::Llvm,
        rust::get_rustup_home,
    },
    update::check_for_update,
};
use log::info;
use miette::Result;
use std::{env, fs::remove_dir_all};

#[derive(Parser)]
#[command(about, version)]
struct Cli {
    #[command(subcommand)]
    subcommand: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Generate completions for the given shell.
    Completions(CompletionsOpts),
    /// Installs Espressif Rust ecosystem.
    // We use a Box here to make clippy happy (see https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant)
    Install(Box<InstallOpts>),
    /// Uninstalls Espressif Rust ecosystem.
    Uninstall(UninstallOpts),
    /// Updates Xtensa Rust toolchain.
    Update(Box<InstallOpts>),
}

/// Updates Xtensa Rust toolchain.
async fn completions(args: CompletionsOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    info!(
        "{} Generating completions for {} shell",
        emoji::DISC,
        args.shell
    );

    clap_complete::generate(
        args.shell,
        &mut Cli::command(),
        "espup",
        &mut std::io::stdout(),
    );

    info!("{} Completions successfully generated!", emoji::CHECK);

    Ok(())
}

/// Installs the Rust for ESP chips environment
async fn install(args: InstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    info!("{} Installing the Espressif Rust ecosystem", emoji::DISC);
    toolchain_install(args).await?;
    info!("{} Installation successfully completed!", emoji::CHECK);
    Ok(())
}

/// Uninstalls the Rust for ESP chips environment
async fn uninstall(args: UninstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    info!("{} Uninstalling the Espressif Rust ecosystem", emoji::DISC);

    let install_path = get_rustup_home().join("toolchains").join(args.name);

    Llvm::uninstall(&install_path)?;

    uninstall_gcc_toolchains(&install_path)?;

    info!(
        "{} Deleting the Xtensa Rust toolchain located in '{}'",
        emoji::DISC,
        &install_path.display()
    );
    remove_dir_all(&install_path)
        .map_err(|_| Error::RemoveDirectory(install_path.display().to_string()))?;

    #[cfg(windows)]
    set_environment_variable("PATH", &env::var("PATH").unwrap())?;

    info!("{} Uninstallation successfully completed!", emoji::CHECK);
    Ok(())
}

/// Updates Xtensa Rust toolchain.
async fn update(args: InstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    info!("{} Updating Espressif Rust ecosystem", emoji::DISC);
    toolchain_install(args).await?;
    info!("{} Update successfully completed!", emoji::CHECK);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().subcommand {
        SubCommand::Completions(args) => completions(args).await,
        SubCommand::Install(args) => install(*args).await,
        SubCommand::Update(args) => update(*args).await,
        SubCommand::Uninstall(args) => uninstall(args).await,
    }
}
