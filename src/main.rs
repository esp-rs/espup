use crate::espidf::{get_install_path, get_tool_path, EspIdfRepo};
use crate::gcc_toolchain::install_gcc_targets;
use crate::llvm_toolchain::LlvmToolchain;
use crate::rust_toolchain::{
    check_rust_installation, get_rustup_home, install_riscv_target, RustCrate, RustToolchain,
};
use crate::targets::{parse_targets, Target};
#[cfg(windows)]
use crate::utils::check_arguments;
use crate::utils::{clear_dist_folder, export_environment, logging::initialize_logger};
use anyhow::Result;
use clap::Parser;
use embuild::espidf::{parse_esp_idf_git_ref, EspIdfRemote};
use log::{debug, info};
use std::{collections::HashSet, fs::remove_dir_all, path::PathBuf};

mod emoji;
mod espidf;
mod gcc_toolchain;
mod llvm_toolchain;
mod rust_toolchain;
mod targets;
mod utils;

#[cfg(windows)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.ps1";
#[cfg(not(windows))]
const DEFAULT_EXPORT_FILE: &str = "export-esp.sh";

#[derive(Parser)]
#[clap(name = "espup")]
#[clap(bin_name = "espup")]
#[clap(arg_required_else_help(true))]
#[clap(version)]
#[clap(about)]
struct Cli {
    #[command(subcommand)]
    subcommand: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Installs esp-rs environment
    Install(InstallOpts),
    /// Uninstalls esp-rs environment
    Uninstall(UninstallOpts),
    /// Updates Xtensa Rust toolchain
    Update(UpdateOpts),
}

#[derive(Debug, Parser)]
pub struct InstallOpts {
    /// ESP-IDF version to install. If empty, no esp-idf is installed. Version format:
    ///
    /// - `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository.
    ///
    /// - `tag:<tag>`: Uses the tag `<tag>` of the `esp-idf` repository.
    ///
    /// - `branch:<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.
    ///
    /// - `v<major>.<minor>` or `<major>.<minor>`: Uses the tag `v<major>.<minor>` of the `esp-idf` repository.
    ///
    /// - `<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.
    #[arg(short = 'e', long, required = false)]
    pub espidf_version: Option<String>,
    /// Destination of the generated export file.
    #[arg(short = 'f', long, default_value = DEFAULT_EXPORT_FILE)]
    pub export_file: PathBuf,
    /// Comma or space list of extra crates to install.
    // Make it vector and have splliter =" "
    #[arg(short = 'c', long, default_value = "cargo-espflash")]
    pub extra_crates: String,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Nightly Rust toolchain version.
    #[arg(short = 'n', long, default_value = "nightly")]
    pub nightly_version: String,
    ///  Minifies the installation.
    #[arg(short = 'm', long)]
    pub profile_minimal: bool,
    /// Comma or space separated list of targets [esp32,esp32s2,esp32s3,esp32c3,all].
    #[arg(short = 't', long, default_value = "all")]
    pub targets: String,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long, default_value = "1.64.0.0")]
    pub toolchain_version: String,
}

#[derive(Debug, Parser)]
pub struct UpdateOpts {
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long, default_value = "1.64.0.0")]
    pub toolchain_version: String,
}

#[derive(Debug, Parser)]
pub struct UninstallOpts {
    /// ESP-IDF version to uninstall. If empty, no esp-idf is uninstalled. Version format:
    ///
    /// - `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository.
    ///
    /// - `tag:<tag>`: Uses the tag `<tag>` of the `esp-idf` repository.
    ///
    /// - `branch:<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.
    ///
    /// - `v<major>.<minor>` or `<major>.<minor>`: Uses the tag `v<major>.<minor>` of the `esp-idf` repository.
    ///
    /// - `<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.
    #[arg(short = 'e', long, required = false)]
    pub espidf_version: Option<String>,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Removes clang.
    #[arg(short = 'c', long)]
    pub remove_clang: bool,
}

/// Installs esp-rs environment
fn install(args: InstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);

    info!("{} Installing esp-rs", emoji::DISC);
    let targets: HashSet<Target> = parse_targets(&args.targets).unwrap();
    let mut extra_crates: HashSet<RustCrate> =
        args.extra_crates.split(',').map(RustCrate::new).collect();
    let mut exports: Vec<String> = Vec::new();
    let export_file = args.export_file.clone();
    let rust_toolchain = RustToolchain::new(args.toolchain_version.clone());

    // Complete LLVM is failing for Windows and MacOS, so we are using always minified.
    #[cfg(target_os = "linux")]
    let llvm = LlvmToolchain::new(args.profile_minimal);
    #[cfg(not(target_os = "linux"))]
    let llvm = LlvmToolchain::new(true);

    debug!(
        "{} Arguments:
            - Targets: {:?}
            - ESP-IDF version: {:?}
            - Export file: {:?}
            - Extra crates: {:?}
            - LLVM Toolchain: {:?}
            - Nightly version: {:?}
            - Rust Toolchain: {:?}
            - Profile Minimal: {:?}
            - Toolchain version: {:?}",
        emoji::INFO,
        targets,
        &args.espidf_version,
        export_file,
        extra_crates,
        llvm,
        &args.nightly_version,
        rust_toolchain,
        args.profile_minimal,
        args.toolchain_version,
    );

    #[cfg(windows)]
    check_arguments(&targets, &args.espidf_version)?;

    check_rust_installation(&args.nightly_version)?;

    rust_toolchain.install_xtensa_rust()?;

    exports.extend(llvm.install()?);

    if targets.contains(&Target::ESP32C3) {
        install_riscv_target(&args.nightly_version)?;
    }

    if let Some(espidf_version) = &args.espidf_version {
        let repo = EspIdfRepo::new(espidf_version, args.profile_minimal, targets);
        exports.extend(repo.install()?);
        extra_crates.insert(RustCrate::new("ldproxy"));
    } else {
        exports.extend(install_gcc_targets(targets)?);
    }

    debug!(
        "{} Installing the following crates: {:#?}",
        emoji::DEBUG,
        extra_crates
    );
    for extra_crate in extra_crates {
        extra_crate.install()?;
    }

    if args.profile_minimal {
        clear_dist_folder()?;
    }

    export_environment(&export_file, &exports)?;

    info!("{} Installation suscesfully completed!", emoji::CHECK);
    Ok(())
}

/// Uninstalls esp-rs environment
fn uninstall(args: UninstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    info!("{} Uninstalling esp-rs", emoji::DISC);

    debug!(
        "{} Arguments:
            - Remove Clang: {}
            - ESP-IDF version: {:#?}",
        emoji::INFO,
        &args.remove_clang,
        &args.espidf_version,
    );

    info!("{} Deleting Xtensa Rust toolchain", emoji::WRENCH);
    remove_dir_all(get_rustup_home().join("toolchains").join("esp"))?;

    if args.remove_clang {
        info!("{} Deleting Xtensa Clang", emoji::WRENCH);
        remove_dir_all(PathBuf::from(get_tool_path("")).join("xtensa-esp32-elf-clang"))?;
    }

    clear_dist_folder()?;

    if let Some(espidf_version) = &args.espidf_version {
        info!("{} Deleting ESP-IDF", emoji::WRENCH);
        let repo = EspIdfRemote {
            git_ref: parse_esp_idf_git_ref(espidf_version),
            repo_url: Some(espidf::DEFAULT_GIT_REPOSITORY.to_string()),
        };
        remove_dir_all(get_install_path(repo).parent().unwrap())?;
    }

    info!("{} Uninstallation suscesfully completed!", emoji::CHECK);
    Ok(())
}

/// Updates Xtensa Rust toolchain
fn update(args: UpdateOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    info!("{} Updating Xtensa Rust toolchain", emoji::DISC);

    debug!(
        "{} Arguments:
            - Toolchain version: {}",
        emoji::INFO,
        &args.toolchain_version,
    );

    info!("{} Deleting previous Xtensa Rust toolchain", emoji::WRENCH);
    remove_dir_all(get_rustup_home().join("toolchains").join("esp"))?;

    let rust_toolchain = RustToolchain::new(args.toolchain_version);
    rust_toolchain.install_xtensa_rust()?;

    info!("{} Update suscesfully completed!", emoji::CHECK);
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().subcommand {
        SubCommand::Install(args) => install(args),
        SubCommand::Update(args) => update(args),
        SubCommand::Uninstall(args) => uninstall(args),
    }
}
