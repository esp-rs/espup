use crate::chip::*;
use crate::toolchain::*;
use crate::utils::*;
use anyhow::Result;
use clap::Parser;
use clap_verbosity_flag::{InfoLevel, Verbosity};
use embuild::cmd;
use log::{info, warn};
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

mod chip;
mod emoji;
mod toolchain;
mod utils;
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
    #[clap(short = 'b', long, default_value = "esp32,esp32s2,esp32s3")]
    pub build_target: String,
    /// Path to .cargo.
    // TODO: Use home_dir to make it diferent in every OS: #[clap(short = 'c', long, default_value_t: &'a Path = Path::new(format!("{}/.cargo",home_dir())))]
    #[clap(short = 'c', long, default_value = "/home/esp/.cargo")]
    pub cargo_home: PathBuf,
    /// Toolchain instalation folder.
    #[clap(short = 'd', long, default_value = "/home/esp/.rustup/toolchains/esp")]
    pub toolchain_destination: PathBuf,
    /// Comma or space list of extra crates to install.
    // Make it vector and have splliter =" "
    #[clap(short = 'e', long, default_value = "cargo-espflash")]
    pub extra_crates: String,
    /// Destination of the export file generated.
    #[clap(short = 'f', long)]
    pub export_file: Option<PathBuf>,
    /// LLVM version. [13, 14, 15]
    #[clap(short = 'l', long, default_value = "14")]
    pub llvm_version: String,
    ///  [Only applies if using -s|--esp-idf-version]. Deletes some esp-idf folders to save space.
    #[clap(short = 'm', long)]
    pub minified_espidf: Option<bool>,
    /// Nightly Rust toolchain version.
    #[clap(short = 'n', long, default_value = "nightly")]
    pub nightly_version: String,
    // /// Path to .rustup.
    #[clap(short = 'r', long, default_value = "/home/esp/.rustup")]
    pub rustup_home: PathBuf,
    // /// ESP-IDF branch to install. If empty, no esp-idf is installed.
    #[clap(short = 's', long)]
    pub espidf_version: Option<String>,
    /// Xtensa Rust toolchain version.
    #[clap(short = 't', long, default_value = "1.62.1.0")]
    pub toolchain_version: String,
    /// Removes cached distribution files.
    #[clap(short = 'x', long)]
    pub clear_cache: bool,
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

    let arch = guess_host_triple::guess_host_triple().unwrap();
    let targets: Vec<Chip> = parse_targets(&args.build_target).unwrap();
    let llvm_version = parse_llvm_version(&args.llvm_version).unwrap();
    let artifact_file_extension = get_artifact_llvm_extension(arch).to_string();
    let llvm_arch = get_llvm_arch(arch).to_string();
    let llvm_file = format!(
        "xtensa-esp32-elf-llvm{}-{}-{}.{}",
        get_llvm_version_with_underscores(&llvm_version),
        &llvm_version,
        llvm_arch,
        artifact_file_extension
    );
    let rust_dist = format!("rust-{}-{}", args.toolchain_version, arch);
    let rust_src_dist = format!("rust-src-{}", args.toolchain_version);
    let rust_dist_file = format!("{}.{}", rust_dist, artifact_file_extension);
    let rust_src_dist_file = format!("{}.{}", rust_src_dist, artifact_file_extension);
    let rust_dist_url = format!(
        "https://github.com/esp-rs/rust-build/releases/download/v{}/{}",
        args.toolchain_version, rust_dist_file
    );
    let rust_src_dist_url = format!(
        "https://github.com/esp-rs/rust-build/releases/download/v{}/{}",
        args.toolchain_version, rust_src_dist_file
    );
    let llvm_url = format!(
        "https://github.com/espressif/llvm-project/releases/download/{}/{}",
        &llvm_version, llvm_file
    );
    let idf_tool_xtensa_elf_clang = format!(
        "{}/{}-{}",
        get_tool_path("xtensa-esp32-elf-clang"),
        &llvm_version,
        arch
    );
    let mut exports: Vec<String> = Vec::new();
    info!("{} Installing esp-rs", emoji::DISC);
    print_arguments(&args, arch, &targets, &llvm_version);

    check_rust_installation(&args.nightly_version)?;

    if args.toolchain_destination.exists() {
        warn!(
            "{} Previous installation of Rust Toolchain exist in: {}.\n Please, remove the directory before new installation.",
            emoji::WARN,
            args.toolchain_destination.display()
        );
        return Ok(());
    } else {
        // install_rust_xtensa_toolchain
        // Some platfroms like Windows are available in single bundle rust + src, because install
        // script in dist is not available for the plaform. It's sufficient to extract the toolchain
        info!("{} Installing Xtensa Rust toolchain", emoji::WRENCH);
        if get_rust_installer(arch).to_string().is_empty() {
            download_file(
                rust_dist_url,
                "rust.zip",
                &args.toolchain_destination.display().to_string(),
                true,
            )?;
        } else {
            download_file(rust_dist_url, "rust.tar.xz", &get_dist_path("rust"), true)?;
            info!("{} Installing rust esp toolchain", emoji::WRENCH);
            let arguments = format!(
                "{}/rust-nightly-{}/install.sh --destdir={} --prefix='' --without=rust-docs",
                get_dist_path("rust"),
                arch,
                args.toolchain_destination.display()
            );
            cmd!("/bin/bash", "-c", arguments).run()?;

            download_file(
                rust_src_dist_url,
                "rust-src.tar.xz",
                &get_dist_path("rust-src"),
                true,
            )?;
            info!("{} Installing rust-src for esp toolchain", emoji::WRENCH);
            let arguments = format!(
                "{}/rust-src-nightly/install.sh --destdir={} --prefix='' --without=rust-docs",
                get_dist_path("rust-src"),
                args.toolchain_destination.display()
            );
            cmd!("/bin/bash", "-c", arguments).run()?;
        }
    }

    // TODO: move to function
    info!("{} Installing Xtensa elf Clang", emoji::WRENCH);
    if Path::new(idf_tool_xtensa_elf_clang.as_str()).exists() {
        warn!(
            "{} Previous installation of LLVM exist in: {}.\n Please, remove the directory before new installation.",
            emoji::WARN,
            idf_tool_xtensa_elf_clang
        );
    } else {
        download_file(
            llvm_url,
            &format!(
                "idf_tool_xtensa_elf_clang.{}",
                get_artifact_llvm_extension(arch)
            ),
            &get_tool_path(""),
            true,
        )?;
    }
    let libclang_path = format!("{}/lib", get_tool_path("xtensa-esp32-elf-clang"));
    exports.push(format!("export LIBCLANG_PATH=\"{}\"", &libclang_path));

    if targets.contains(&Chip::ESP32C3) {
        install_riscv_target(&args.nightly_version)?;
    }

    if args.espidf_version.is_some() {
        let espidf_version = args.espidf_version.unwrap();
        let mut espidf_targets: String = String::new();
        for target in targets {
            if espidf_targets.is_empty() {
                espidf_targets =
                    espidf_targets + &target.to_string().to_lowercase().replace('-', "");
            } else {
                espidf_targets =
                    espidf_targets + "," + &target.to_string().to_lowercase().replace('-', "");
            }
        }
        install_espidf(&espidf_targets, &espidf_version).unwrap();
        exports.push(format!("export IDF_TOOLS_PATH=\"{}\"", get_tools_path()));
        exports.push(format!(
            "source {}/export.sh",
            get_espidf_path(&espidf_version)
        ));
        // TODO: Install ldproxy
        install_extra_crate("ldproxy")?;
    } else {
        info!("{} Installing gcc for build targets", emoji::WRENCH);
        exports.extend(install_gcc_targets(targets).unwrap().iter().cloned());
    }

    // TODO: Install extra crates

    // TODO: Clear cache

    info!("{} Updating environment variables:", emoji::DIAMOND);
    for e in exports.iter() {
        info!("{}", e);
    }
    if args.export_file.is_some() {
        let mut file = File::create(args.export_file.unwrap())?;
        for e in exports.iter() {
            file.write_all(e.as_bytes())?;
            file.write_all(b"\n")?;
        }
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
