use crate::chip::Chip;
use crate::espidf::EspIdf;
use crate::gcc_toolchain::install_gcc_targets;
use crate::llvm_toolchain::LlvmToolchain;
use crate::rust_toolchain::{
    check_rust_installation, get_rust_crate, install_crate, RustCrate, RustToolchain,
};
use crate::utils::{clear_dist_folder, get_tools_path, parse_targets, print_arguments};
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::info;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

mod chip;
mod emoji;
mod espidf;
mod gcc_toolchain;
mod llvm_toolchain;
mod rust_toolchain;
mod utils;

#[cfg(windows)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.ps1";
#[cfg(not(windows))]
const DEFAULT_EXPORT_FILE: &str = "export-esp.sh";

#[derive(Parser)]
struct Opts {
    #[clap(subcommand)]
    subcommand: SubCommand,
}

#[derive(Parser)]
pub enum SubCommand {
    /// Installs esp-rs environment
    Install(InstallOpts),
    /// Updates esp-rs Rust toolchain
    Update(UpdateOpts),
    /// Uninstalls esp-rs environment
    Uninstall(UninstallOpts),
    /// Reinstalls esp-rs environment
    Reinstall(InstallOpts),
}

#[derive(Debug, Parser)]
pub struct InstallOpts {
    /// Comma or space separated list of targets [esp32,esp32s2,esp32s3,esp32c3,all].
    #[clap(short = 'b', long, default_value = "all")]
    pub build_target: String,
    /// Path to .cargo.
    #[clap(short = 'c', long, required = false)]
    pub cargo_home: Option<PathBuf>,
    /// Toolchain instalation folder.
    #[clap(short = 'd', long, required = false)]
    pub toolchain_destination: Option<PathBuf>,
    /// Comma or space list of extra crates to install.
    // Make it vector and have splliter =" "
    #[clap(short = 'e', long, default_value = "cargo-espflash")]
    pub extra_crates: String,
    /// Destination of the export file generated.
    #[clap(short = 'f', long, required = false)]
    pub export_file: Option<PathBuf>,
    /// LLVM version. [13, 14, 15]
    #[clap(short = 'l', long, default_value = "14")]
    pub llvm_version: String,
    ///  [Only applies if using -s|--esp-idf-version]. Deletes some esp-idf folders to save space.
    #[clap(short = 'm', long, takes_value = false)]
    pub minified_espidf: bool,
    /// Nightly Rust toolchain version.
    #[clap(short = 'n', long, default_value = "nightly")]
    pub nightly_version: String,
    /// Path to .rustup.
    #[clap(short = 'r', long, required = false)]
    pub rustup_home: Option<PathBuf>,
    /// ESP-IDF version to install. If empty, no esp-idf is installed. Format:
    /// - `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository.
    /// - `tag:<tag>`: Uses the tag `<tag>` of the `esp-idf` repository.
    /// - `branch:<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.
    /// - `v<major>.<minor>` or `<major>.<minor>`: Uses the tag `v<major>.<minor>` of the `esp-idf` repository.
    /// - `<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.
    #[clap(short = 's', long, required = false)]
    pub espidf_version: Option<String>,
    /// Xtensa Rust toolchain version.
    #[clap(short = 't', long, default_value = "1.62.1.0")]
    pub toolchain_version: String,
    /// Removes cached distribution files.
    #[clap(short = 'x', long, takes_value = false)]
    pub clear_dist: bool,
    /// Verbosity level of the logs.
    #[clap(flatten)]
    verbose: Verbosity<InfoLevel>,
}

#[derive(Parser, Debug)]
pub struct UpdateOpts {
    /// Xtensa Rust toolchain version.
    #[clap(short = 't', long, default_value = "1.62.1.0")]
    pub toolchain_version: String,
}

#[derive(Parser, Debug)]
pub struct UninstallOpts {
    /// Removes clang.
    #[clap(short = 'r', long)]
    pub remove_clang: bool,
    // TODO: Other options to remove?
}

fn install(args: InstallOpts) -> Result<()> {
    env_logger::Builder::new()
        .filter_level(args.verbose.log_level_filter())
        .init();

    info!("{} Installing esp-rs", emoji::DISC);
    let arch = guess_host_triple::guess_host_triple().unwrap();
    let targets: Vec<Chip> = parse_targets(&args.build_target).unwrap();
    let mut extra_crates: Vec<RustCrate> =
        args.extra_crates.split(',').map(get_rust_crate).collect();
    print_arguments(&args, arch, &targets);

    let rust_toolchain = RustToolchain::new(&args, arch, &targets);

    let mut exports: Vec<String> = Vec::new();

    let llvm = LlvmToolchain::new(&args.llvm_version);

    check_rust_installation(&args.nightly_version)?;

    rust_toolchain.install_xtensa()?;

    llvm.install()?;
    #[cfg(windows)]
    exports.push(format!("$Env:LIBCLANG_PATH=\"{}\"", &llvm.get_lib_path()));
    #[cfg(unix)]
    exports.push(format!("export LIBCLANG_PATH=\"{}\"", &llvm.get_lib_path()));

    if targets.contains(&Chip::ESP32C3) {
        rust_toolchain.install_riscv_target()?;
    }

    if args.espidf_version.is_some() {
        let espidf_version = args.espidf_version.unwrap();
        let espidf = EspIdf::new(&espidf_version, args.minified_espidf, targets);
        espidf.install()?;
        #[cfg(windows)]
        exports.push(format!("$Env:IDF_TOOLS_PATH=\"{}\"", get_tools_path()));
        #[cfg(unix)]
        exports.push(format!("export IDF_TOOLS_PATH=\"{}\"", get_tools_path()));

        // TODO: Fix export path
        // exports.push(format!(". {}/export.sh", install_path.display()));
        extra_crates.push(get_rust_crate("ldproxy"));
    } else {
        info!("{} Installing gcc for build targets", emoji::WRENCH);
        exports.extend(install_gcc_targets(targets).unwrap().iter().cloned());
    }

    for extra_crate in extra_crates {
        install_crate(extra_crate)?;
    }

    if args.clear_dist {
        clear_dist_folder()?;
    }

    // TODO: Move to a fn
    info!("{} Updating environment variables:", emoji::DIAMOND);
    for e in exports.iter() {
        info!("{}", e);
    }
    let export_file = args
        .export_file
        .unwrap_or_else(|| PathBuf::from_str(DEFAULT_EXPORT_FILE).unwrap());
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

fn update(_args: UpdateOpts) -> Result<()> {
    // TODO: Update Rust toolchain
    todo!();
}

fn uninstall(_args: UninstallOpts) -> Result<()> {
    // TODO: Uninstall
    todo!();
}

fn reinstall(_args: InstallOpts) -> Result<()> {
    todo!();
    // uninstall();
    // install(args);
}

#[tokio::main]
async fn main() -> Result<()> {
    match Opts::parse().subcommand {
        SubCommand::Install(args) => install(args),
        SubCommand::Update(args) => update(args),
        SubCommand::Uninstall(args) => uninstall(args),
        SubCommand::Reinstall(args) => reinstall(args),
    }
}
