use crate::toolchain::*;
use crate::utils::*;
use clap::Parser;
use espflash::Chip;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
mod emoji;
mod toolchain;
mod utils;
use anyhow::{bail, Result};
use clap_verbosity_flag::{InfoLevel, Verbosity};
use log::{debug, info, warn};

// General TODOs:
// - Add extra-crates installation support
// - Add minified-esp-idf installation support
// - Add clear_cache funtionality
// - Rustup_home and cargo_home are not used
// - Avoid using shell commands
// - Maybe split toolchain into toolchain(espidf, gcc, llvm...) and rust(rust checks, instalaltion and crates)
// - Add subcommand test that downloads a projects and builds it
// - Esp-idf version should be contained in an enum with the possible values (see chips in espflash for reference)
// - Do a Tauri App so we can install it with gui. If no subcommand is passed, run gui installator
// - Add tests
// - Clean unused code
// - Add progress bar
// - For uninstall cmd: Run uninstall.sh scripts, delete rust and rust-src folders, delete llvm and gcc files

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
    let artifact_file_extension = get_artifact_file_extension(arch).to_string();
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
            // TODO: Check idf_env and adjust
            // match prepare_package_strip_prefix(&rust_dist_url,
            //                              &rust_dist_file,
            //                              get_tool_path("rust".to_string()),
            //                              "esp") {
            //                                 Ok(_) => { info!("Package ready"); },
            //                                 Err(_e) => { bail!("Unable to prepare package"); }
            //                             }
        } else {
            match prepare_package_strip_prefix(
                &rust_dist_url,
                get_tool_path("rust"),
                &format!("rust-nightly-{}", arch),
            ) {
                Ok(_) => {
                    debug!("{} Package rust ready", emoji::CHECK);
                }
                Err(_e) => {
                    bail!("{} Unable to prepare rust", emoji::ERROR);
                }
            }
            info!("{} Installing rust", emoji::WRENCH);
            let mut arguments: Vec<String> = [].to_vec();
            arguments.push("-c".to_string());
            arguments.push(format!(
                "{}/install.sh --destdir={} --prefix='' --without=rust-docs",
                get_tool_path("rust"),
                args.toolchain_destination.display()
            ));

            match run_command("/bin/bash", arguments.clone(), "".to_string()) {
                Ok(_) => {
                    debug!("{} rust/install.sh command succeeded", emoji::CHECK);
                }
                Err(_e) => {
                    bail!("{} rust/install.sh command failed", emoji::ERROR);
                }
            }

            match prepare_package_strip_prefix(
                &rust_src_dist_url,
                get_tool_path("rust-src"),
                "rust-src-nightly",
            ) {
                Ok(_) => {
                    debug!("{} Package rust-src ready", emoji::CHECK);
                }
                Err(_e) => {
                    bail!("{} Unable to prepare rust-src", emoji::ERROR);
                }
            }

            info!("{} Installing rust-src", emoji::WRENCH);
            let mut arguments: Vec<String> = [].to_vec();
            arguments.push("-c".to_string());
            arguments.push(format!(
                "{}/install.sh --destdir={} --prefix='' --without=rust-docs",
                get_tool_path("rust-src"),
                args.toolchain_destination.display()
            ));
            match run_command("/bin/bash", arguments, "".to_string()) {
                Ok(_) => {
                    debug!("{} rust-src/install.sh Command succeeded", emoji::CHECK);
                }
                Err(_e) => {
                    bail!("{} rust-src/install.sh Command failed", emoji::ERROR);
                }
            }
        }
    }

    // install_llvm_clang
    // TODO: move to function
    if Path::new(idf_tool_xtensa_elf_clang.as_str()).exists() {
        warn!(
            "{} Previous installation of LLVM exist in: {}.\n Please, remove the directory before new installation.",
            emoji::WARN,
            idf_tool_xtensa_elf_clang
        );
    } else {
        match prepare_package_strip_prefix(
            &llvm_url,
            get_tool_path(&format!(
                "xtensa-esp32-elf-clang-{}-{}",
                llvm_version, llvm_arch
            )),
            "",
        ) {
            Ok(_) => {
                debug!("{} Package xtensa-esp32-elf-clang ready", emoji::CHECK);
            }
            Err(_e) => {
                bail!("{} Unable to prepare xtensa-esp32-elf-clang", emoji::ERROR);
            }
        }
    }
    let libclang_path = format!("{}/lib", get_tool_path("xtensa-esp32-elf-clang"));
    exports.push(format!("export LIBCLANG_PATH=\"{}\"", &libclang_path));

    if targets.contains(&Chip::Esp32c3) {
        info!("{} Installing riscv target", emoji::WRENCH);
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
        info!("{} Installing gcc for targets", emoji::WRENCH);
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
