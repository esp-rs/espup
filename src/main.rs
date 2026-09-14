use clap::{CommandFactory, Parser};
use espup::{
    cli::{CompletionsOpts, DoctorOpts, EnvOpts, InstallOpts, UninstallOpts},
    doctor::{print_doctor, print_env_exports},
    logging::initialize_logger,
    toolchain::{
        InstallMode,
        gcc::uninstall_gcc_toolchains,
        install as toolchain_install,
        llvm::Llvm,
        remove_dir,
        rust::{XtensaRust, get_rustup_home},
    },
    update::check_for_update,
};
use log::info;
use miette::Result;
use std::{env, io::stdout};

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
    /// Diagnose the local Espressif Rust toolchain / environment.
    Doctor(DoctorOpts),
    /// Print export lines for `eval "$(espup env)"`.
    Env(EnvOpts),
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

    info!("Generating completions for {} shell", args.shell);

    args.shell
        .generate(&mut Cli::command(), "espup", &mut stdout());

    info!("Completions successfully generated!");

    Ok(())
}

/// Installs or updates the Rust for ESP chips environment
async fn install(args: InstallOpts, install_mode: InstallMode) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    toolchain_install(args, install_mode).await?;
    Ok(())
}

/// Uninstalls the Rust for ESP chips environment
async fn uninstall(args: UninstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    info!("Uninstalling the Espressif Rust ecosystem");
    let toolchain_dir = get_rustup_home().join("toolchains").join(args.name);

    if toolchain_dir.exists() {
        Llvm::uninstall(&toolchain_dir).await?;

        uninstall_gcc_toolchains(&toolchain_dir, args.crosstool_toolchain_version).await?;

        XtensaRust::uninstall(&toolchain_dir).await?;

        remove_dir(&toolchain_dir).await?;
    }

    info!("Uninstallation successfully completed!");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().subcommand {
        SubCommand::Completions(args) => completions(args).await,
        SubCommand::Doctor(args) => {
            initialize_logger(&args.log_level);
            if print_doctor(&args.name) {
                Ok(())
            } else {
                Err(miette::miette!("espup doctor found problems"))
            }
        }
        SubCommand::Env(args) => {
            initialize_logger(&args.log_level);
            print_env_exports(args.export_file)
                .map_err(|e| miette::miette!(e))
        }
        SubCommand::Install(args) => install(*args, InstallMode::Install).await,
        SubCommand::Update(args) => install(*args, InstallMode::Update).await,
        SubCommand::Uninstall(args) => uninstall(args).await,
    }
}
