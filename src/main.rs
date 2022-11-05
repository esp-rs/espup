use anyhow::{bail, Result};
use clap::Parser;
use directories_next::ProjectDirs;
use embuild::{
    cmd,
    espidf::{parse_esp_idf_git_ref, EspIdfRemote},
};
use espup::{
    config::Config,
    emoji,
    host_triple::get_host_triple,
    logging::initialize_logger,
    targets::{parse_targets, Target},
    toolchain::{
        espidf::{
            get_dist_path, get_install_path, get_tool_path, EspIdfRepo, DEFAULT_GIT_REPOSITORY,
        },
        gcc::{get_toolchain_name, install_gcc_targets},
        llvm::Llvm,
        rust::{check_rust_installation, install_riscv_target, Crate, XtensaRust},
    },
};
use log::{debug, info, warn};
use std::{
    collections::HashSet,
    fs::{remove_dir_all, remove_file, File},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(windows)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.ps1";
#[cfg(not(windows))]
const DEFAULT_EXPORT_FILE: &str = "export-esp.sh";

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
    /// Target triple of the host.
    #[arg(short = 'd', long, required = false)]
    pub default_host: Option<String>,
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
    ///
    /// When using this option, `ldproxy` crate will also be installed.
    #[arg(short = 'e', long, required = false)]
    pub espidf_version: Option<String>,
    /// Destination of the generated export file.
    #[arg(short = 'f', long, default_value = DEFAULT_EXPORT_FILE)]
    pub export_file: PathBuf,
    /// Comma or space list of extra crates to install.
    #[arg(short = 'c', long, default_value = "cargo-espflash")]
    pub extra_crates: String,
    /// LLVM version.
    #[arg(short = 'x', long, default_value = "15", value_parser = ["15"])]
    pub llvm_version: String,
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
    #[arg(short = 'v', long, value_parser = XtensaRust::parse_version)]
    pub toolchain_version: Option<String>,
}

#[derive(Debug, Parser)]
pub struct UpdateOpts {
    /// Target triple of the host.
    #[arg(short = 'd', long, required = false)]
    pub default_host: Option<String>,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long, value_parser = XtensaRust::parse_version)]
    pub toolchain_version: Option<String>,
}

#[derive(Debug, Parser)]
pub struct UninstallOpts {
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
}

/// Installs the Rust for ESP chips environment
fn install(args: InstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);

    info!("{} Installing esp-rs", emoji::DISC);
    let targets: HashSet<Target> = parse_targets(&args.targets).unwrap();
    let host_triple = get_host_triple(args.default_host)?;
    let mut extra_crates: HashSet<Crate> = args.extra_crates.split(',').map(Crate::new).collect();
    let mut exports: Vec<String> = Vec::new();
    let export_file = if args.export_file.is_absolute() {
        args.export_file
    } else {
        let current_dir = std::env::current_dir()?;
        current_dir.join(args.export_file)
    };
    let xtensa_rust = if targets.contains(&Target::ESP32)
        || targets.contains(&Target::ESP32S2)
        || targets.contains(&Target::ESP32S3)
    {
        let xtensa_rust: XtensaRust = if let Some(toolchain_version) = &args.toolchain_version {
            XtensaRust::new(toolchain_version, &host_triple)
        } else {
            let latest_version = XtensaRust::get_latest_version()?;
            XtensaRust::new(&latest_version, &host_triple)
        };
        Some(xtensa_rust)
    } else {
        None
    };
    let llvm = Llvm::new(args.llvm_version, args.profile_minimal, &host_triple);

    debug!(
        "{} Arguments:
            - Host triple: {}
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
        host_triple,
        targets,
        &args.espidf_version,
        &export_file,
        &extra_crates,
        llvm,
        &args.nightly_version,
        xtensa_rust,
        args.profile_minimal,
        args.toolchain_version,
    );

    #[cfg(windows)]
    check_arguments(&targets, &args.espidf_version)?;

    check_rust_installation(&args.nightly_version)?;

    if let Some(ref xtensa_rust) = xtensa_rust {
        xtensa_rust.install()?;
    }

    exports.extend(llvm.install()?);

    if targets.contains(&Target::ESP32C3) {
        install_riscv_target(&args.nightly_version)?;
    }

    if let Some(espidf_version) = &args.espidf_version {
        let repo = EspIdfRepo::new(espidf_version, args.profile_minimal, &targets);
        exports.extend(repo.install()?);
        extra_crates.insert(Crate::new("ldproxy"));
    } else {
        exports.extend(install_gcc_targets(&targets, &host_triple)?);
    }

    debug!(
        "{} Installing the following crates: {:#?}",
        emoji::DEBUG,
        extra_crates
    );
    for extra_crate in &extra_crates {
        extra_crate.install()?;
    }

    if args.profile_minimal {
        clear_dist_folder()?;
    }

    export_environment(&export_file, &exports)?;

    info!("{} Saving configuration file", emoji::WRENCH);
    let config = Config {
        espidf_version: args.espidf_version,
        export_file,
        extra_crates: extra_crates
            .iter()
            .map(|x| x.name.clone())
            .collect::<HashSet<String>>(),
        host_triple,
        llvm_path: llvm.path,
        nightly_version: args.nightly_version,
        targets,
        xtensa_rust,
    };
    if let Err(e) = config.save() {
        bail!("{} Failed to save config {:#}", emoji::ERROR, e);
    }

    info!("{} Installation successfully completed!", emoji::CHECK);
    warn!(
        "{} Please, source the export file, as state above, to properly setup the environment!",
        emoji::WARN
    );
    Ok(())
}

/// Uninstalls the Rust for ESP chips environment
fn uninstall(args: UninstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    info!("{} Uninstalling esp-rs", emoji::DISC);
    let config = Config::load().unwrap();

    debug!(
        "{} Arguments:
            - Config: {:#?}",
        emoji::INFO,
        config
    );

    if let Some(xtensa_rust) = config.xtensa_rust {
        xtensa_rust.uninstall()?;
    }

    info!("{} Deleting Xtensa LLVM", emoji::WRENCH);
    remove_dir_all(config.llvm_path)?;

    if let Some(espidf_version) = config.espidf_version {
        info!("{} Deleting ESP-IDF {}", emoji::WRENCH, espidf_version);
        let repo = EspIdfRemote {
            git_ref: parse_esp_idf_git_ref(&espidf_version),
            repo_url: Some(DEFAULT_GIT_REPOSITORY.to_string()),
        };
        remove_dir_all(get_install_path(repo).parent().unwrap())?;
    } else {
        info!("{} Deleting GCC targets", emoji::WRENCH);
        for target in &config.targets {
            let gcc_path = get_tool_path(&get_toolchain_name(target));
            remove_dir_all(gcc_path)?;
        }
    }

    info!("{} Uninstalling extra crates", emoji::WRENCH);
    for extra_crate in &config.extra_crates {
        cmd!("cargo", "uninstall", extra_crate).run()?;
    }

    clear_dist_folder()?;

    info!("{} Deleting export file", emoji::WRENCH);
    remove_file(Path::new(&config.export_file))?;

    info!("{} Deleting config file", emoji::WRENCH);
    let conf_dirs = ProjectDirs::from("rs", "esp", "espup").unwrap();
    let conf_file = conf_dirs.config_dir().join("espup.toml");
    remove_file(conf_file)?;

    info!("{} Uninstallation successfully completed!", emoji::CHECK);
    Ok(())
}

/// Updates Xtensa Rust toolchain.
fn update(args: UpdateOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    info!("{} Updating ESP Rust environment", emoji::DISC);
    let host_triple = get_host_triple(args.default_host)?;
    let mut config = Config::load().unwrap();
    let xtensa_rust: XtensaRust = if let Some(toolchain_version) = args.toolchain_version {
        XtensaRust::new(&toolchain_version, &host_triple)
    } else {
        let latest_version = XtensaRust::get_latest_version()?;
        XtensaRust::new(&latest_version, &host_triple)
    };

    debug!(
        "{} Arguments:
            - Host triple: {}
            - Toolchain version: {:#?}
            - Config: {:#?}",
        emoji::INFO,
        host_triple,
        xtensa_rust,
        config
    );

    if let Some(config_xtensa_rust) = config.xtensa_rust {
        if config_xtensa_rust.version == xtensa_rust.version {
            info!(
                "{} Toolchain '{}' is already up to date",
                emoji::CHECK,
                xtensa_rust.version
            );
            return Ok(());
        }
        config_xtensa_rust.uninstall()?;
        xtensa_rust.install()?;
        config.xtensa_rust = Some(xtensa_rust);
    }

    if let Err(e) = config.save() {
        bail!("{} Failed to save config {:#}", emoji::ERROR, e);
    }

    info!("{} Update successfully completed!", emoji::CHECK);
    Ok(())
}

fn main() -> Result<()> {
    match Cli::parse().subcommand {
        SubCommand::Install(args) => install(args),
        SubCommand::Update(args) => update(args),
        SubCommand::Uninstall(args) => uninstall(args),
    }
}

/// Deletes dist folder.
fn clear_dist_folder() -> Result<()> {
    let dist_path = PathBuf::from(get_dist_path(""));
    if dist_path.exists() {
        info!("{} Clearing dist folder", emoji::WRENCH);
        remove_dir_all(&dist_path)?;
    }
    Ok(())
}

/// Creates the export file with the necessary environment variables.
pub fn export_environment(export_file: &PathBuf, exports: &[String]) -> Result<()> {
    info!("{} Creating export file", emoji::WRENCH);
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }
    #[cfg(windows)]
    warn!(
        "{} PLEASE set up the environment variables running: '.\\{}'",
        emoji::INFO,
        export_file.display()
    );
    #[cfg(unix)]
    warn!(
        "{} PLEASE set up the environment variables running: '. {}'",
        emoji::INFO,
        export_file.display()
    );
    warn!(
        "{} This step must be done every time you open a new terminal.",
        emoji::WARN
    );
    Ok(())
}

#[cfg(windows)]
/// For Windows, we need to check that we are installing all the targets if we are installing esp-idf.
pub fn check_arguments(targets: &HashSet<Target>, espidf_version: &Option<String>) -> Result<()> {
    if espidf_version.is_some()
        && (!targets.contains(&Target::ESP32)
            || !targets.contains(&Target::ESP32C3)
            || !targets.contains(&Target::ESP32S2)
            || !targets.contains(&Target::ESP32S3))
    {
        bail!(
            "{} When installing esp-idf in Windows, only --targets \"all\" is supported.",
            emoji::ERROR
        );
    }

    Ok(())
}
