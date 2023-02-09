use clap::Parser;
use directories::BaseDirs;
use espup::{
    emoji,
    error::Error,
    host_triple::get_host_triple,
    logging::initialize_logger,
    targets::{parse_targets, Target},
    toolchain::{
        gcc::Gcc,
        llvm::Llvm,
        rust::{check_rust_installation, get_rustup_home, RiscVTarget, XtensaRust},
        Installable,
    },
    update::check_for_update,
};
#[cfg(windows)]
use log::error;
use log::{debug, info, warn};
#[cfg(windows)]
use miette::IntoDiagnostic;
use miette::Result;
use std::{
    collections::HashSet,
    fs::{remove_dir_all, File},
    io::Write,
    path::{Path, PathBuf},
};
use tokio::sync::mpsc;
use tokio_retry::{strategy::FixedInterval, Retry};

#[cfg(windows)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.ps1";
#[cfg(not(windows))]
const DEFAULT_EXPORT_FILE: &str = "export-esp.sh";

#[derive(Parser)]
#[command(
    name = "espup",
    bin_name = "espup",
    version,
    propagate_version = true,
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
    // We use a Box here to make clippy happy (see https://rust-lang.github.io/rust-clippy/master/index.html#large_enum_variant)
    Install(Box<InstallOpts>),
    /// Uninstalls esp-rs environment
    Uninstall(UninstallOpts),
    /// Updates Xtensa Rust toolchain
    Update(UpdateOpts),
}

#[derive(Debug, Parser)]
pub struct InstallOpts {
    /// Path to where the espup configuration file will be written to.
    #[arg(short = 'c', long)]
    pub config_path: Option<PathBuf>,
    /// Target triple of the host.
    #[arg(short = 'd', long, value_parser = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu" , "x86_64-apple-darwin" , "aarch64-apple-darwin"])]
    pub default_host: Option<String>,
    /// Relative or full path for the export file that will be generated. If no path is provided, the file will be generated under home directory (https://docs.rs/dirs/latest/dirs/fn.home_dir.html).
    #[arg(short = 'f', long)]
    pub export_file: Option<PathBuf>,
    /// Extends the LLVM installation.
    ///
    /// This will install the whole LLVM instead of only installing the libs.
    #[arg(short = 'e', long)]
    pub extended_llvm: bool,
    /// LLVM version.
    #[arg(short = 'm', long, default_value = "15", value_parser = ["15"])]
    pub llvm_version: String,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Nightly Rust toolchain version.
    #[arg(short = 'n', long, default_value = "nightly")]
    pub nightly_version: String,
    /// Comma or space separated list of targets [esp32,esp32s2,esp32s3,esp32c2,esp32c3,all].
    #[arg(short = 't', long, default_value = "all", value_parser = parse_targets)]
    pub targets: HashSet<Target>,
    /// Xtensa Rust toolchain name.
    #[arg(short = 'a', long, default_value = "esp")]
    pub name: String,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long, value_parser = XtensaRust::parse_version)]
    pub xtensa_version: Option<String>,
}

#[derive(Debug, Parser)]
pub struct UpdateOpts {
    /// Path to where the espup configuration file will be written to.
    #[arg(short = 'c', long)]
    pub config_path: Option<PathBuf>,
    /// Target triple of the host.
    #[arg(short = 'd', long, value_parser = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu" , "x86_64-apple-darwin" , "aarch64-apple-darwin"])]
    pub default_host: Option<String>,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain name.
    #[arg(short = 'a', long, default_value = "esp")]
    pub name: String,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long, value_parser = XtensaRust::parse_version)]
    pub xtensa_version: Option<String>,
}

#[derive(Debug, Parser)]
pub struct UninstallOpts {
    /// Path to where the espup configuration file will be written to.
    #[arg(short = 'c', long)]
    pub config_path: Option<PathBuf>,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain name.
    #[arg(short = 'a', long, default_value = "esp")]
    pub name: String,
}

/// Installs the Rust for ESP chips environment
async fn install(args: InstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    info!("{} Installing esp-rs", emoji::DISC);
    let targets = args.targets;
    #[cfg(windows)]
    if args.name != "esp" {
        return Err(Error::InvalidName).into_diagnostic();
    }
    let install_path = get_rustup_home().join("toolchains").join(args.name);
    let host_triple = get_host_triple(args.default_host)?;
    let mut exports: Vec<String> = Vec::new();
    let xtensa_rust = if targets.contains(&Target::ESP32)
        || targets.contains(&Target::ESP32S2)
        || targets.contains(&Target::ESP32S3)
    {
        let xtensa_rust: XtensaRust = if let Some(xtensa_version) = &args.xtensa_version {
            XtensaRust::new(xtensa_version, &host_triple, &install_path)
        } else {
            let latest_version = XtensaRust::get_latest_version().await?;
            XtensaRust::new(&latest_version, &host_triple, &install_path)
        };
        Some(xtensa_rust)
    } else {
        None
    };
    let export_file = get_export_file(args.export_file)?;
    let llvm = Llvm::new(
        &args.llvm_version,
        args.extended_llvm,
        &host_triple,
        &install_path,
    )?;

    debug!(
        "{} Arguments:
            - Host triple: {}
            - Targets: {:?}
            - Toolchain path: {:?}
            - Export file: {:?}
            - LLVM Toolchain: {:?}
            - Nightly version: {:?}
            - Rust Toolchain: {:?}
            - Toolchain version: {:?}",
        emoji::INFO,
        host_triple,
        targets,
        &install_path,
        &export_file,
        &llvm,
        &args.nightly_version,
        xtensa_rust,
        args.xtensa_version,
    );

    check_rust_installation().await?;

    // Build up a vector of installable applications, all of which implement the
    // `Installable` async trait.
    let mut to_install = Vec::<Box<dyn Installable + Send + Sync>>::new();

    if let Some(ref xtensa_rust) = xtensa_rust {
        to_install.push(Box::new(xtensa_rust.to_owned()));
    }

    to_install.push(Box::new(llvm));

    if targets.iter().any(|t| t.riscv()) {
        let riscv_target = RiscVTarget::new(&args.nightly_version);
        to_install.push(Box::new(riscv_target));
    }

    targets.iter().for_each(|target| {
        if target.xtensa() {
            let gcc = Gcc::new(target, &host_triple, &install_path);
            to_install.push(Box::new(gcc));
        }
    });
    // All RISC-V targets use the same GCC toolchain
    // ESP32S2 and ESP32S3 also install the RISC-V toolchain for their ULP coprocessor
    if targets.iter().any(|t| t != &Target::ESP32) {
        let riscv_gcc = Gcc::new_riscv(&host_triple, &install_path);
        to_install.push(Box::new(riscv_gcc));
    }

    // With a list of applications to install, install them all in parallel.
    let installable_items = to_install.len();
    let (tx, mut rx) = mpsc::channel::<Result<Vec<String>, Error>>(installable_items);
    for app in to_install {
        let tx = tx.clone();
        let retry_strategy = FixedInterval::from_millis(50).take(3);
        tokio::spawn(async move {
            let res = Retry::spawn(retry_strategy, || async {
                let res = app.install().await;
                if res.is_err() {
                    warn!(
                        "{} Installation for '{}' failed, retrying",
                        emoji::WARN,
                        app.name()
                    );
                }
                res
            })
            .await;
            tx.send(res).await.unwrap();
        });
    }

    // Read the results of the install tasks as they complete.
    for _ in 0..installable_items {
        let names = rx.recv().await.unwrap()?;
        exports.extend(names);
    }

    create_export_file(&export_file, &exports)?;

    info!("{} Installation successfully completed!", emoji::CHECK);
    export_environment(&export_file)?;
    Ok(())
}

/// Uninstalls the Rust for ESP chips environment
async fn uninstall(args: UninstallOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    #[cfg(windows)]
    if args.name != "esp" {
        return Err(Error::InvalidName).into_diagnostic();
    }
    let install_path = get_rustup_home().join("toolchains").join(args.name);

    info!(
        "{} Deleting the Xtensa Rust toolchain located in '{}'",
        emoji::DISC,
        &install_path.display()
    );

    // TODO: In Windows we need to check if the directory has GCC toolchains,
    // if it does, we need to clean the corresponding PATH environment variable
    // Same for LIBCLANG_PATH and LLVM

    remove_dir_all(&install_path)
        .map_err(|_| Error::RemoveDirectory(install_path.display().to_string()))?;

    info!("{} Uninstallation successfully completed!", emoji::CHECK);
    Ok(())
}

/// Updates Xtensa Rust toolchain.
async fn update(args: UpdateOpts) -> Result<()> {
    initialize_logger(&args.log_level);
    check_for_update(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    info!("{} Updating ESP Rust environment", emoji::DISC);
    #[cfg(windows)]
    if args.name != "esp" {
        return Err(Error::InvalidName).into_diagnostic();
    }
    let install_path = get_rustup_home().join("toolchains").join(args.name);
    let host_triple = get_host_triple(args.default_host)?;

    let xtensa_rust: XtensaRust = if let Some(xtensa_version) = args.xtensa_version {
        XtensaRust::new(&xtensa_version, &host_triple, &install_path)
    } else {
        let latest_version = XtensaRust::get_latest_version().await?;
        XtensaRust::new(&latest_version, &host_triple, &install_path)
    };

    debug!(
        "{} Arguments:
            - Host triple: {}
            - Install path: {:#?}
            - Toolchain version: {:#?}",
        emoji::INFO,
        host_triple,
        install_path,
        xtensa_rust,
    );

    XtensaRust::uninstall(&install_path)?;

    xtensa_rust.install().await?;

    info!("{} Update successfully completed!", emoji::CHECK);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    match Cli::parse().subcommand {
        SubCommand::Install(args) => install(*args).await,
        SubCommand::Update(args) => update(args).await,
        SubCommand::Uninstall(args) => uninstall(args).await,
    }
}

/// Returns the absolute path to the export file, uses the DEFAULT_EXPORT_FILE if no arg is provided.
fn get_export_file(export_file: Option<PathBuf>) -> Result<PathBuf, Error> {
    if let Some(export_file) = export_file {
        if export_file.is_dir() {
            return Err(Error::InvalidDestination(export_file.display().to_string()));
        }
        if export_file.is_absolute() {
            Ok(export_file)
        } else {
            let current_dir = std::env::current_dir()?;
            Ok(current_dir.join(export_file))
        }
    } else {
        Ok(BaseDirs::new()
            .unwrap()
            .home_dir()
            .join(DEFAULT_EXPORT_FILE))
    }
}

/// Creates the export file with the necessary environment variables.
fn create_export_file(export_file: &PathBuf, exports: &[String]) -> Result<(), Error> {
    info!("{} Creating export file", emoji::WRENCH);
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        #[cfg(windows)]
        let e = e.replace('/', r#"\"#);
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

/// Instructions to export the environment variables.
fn export_environment(export_file: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    warn!(
        "{} PLEASE set up the environment variables running: '{}'",
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

#[cfg(test)]
mod tests {
    use crate::{create_export_file, get_export_file, DEFAULT_EXPORT_FILE};
    use directories::BaseDirs;
    use std::{env::current_dir, path::PathBuf};

    #[test]
    #[allow(unused_variables)]
    fn test_get_export_file() {
        // No arg provided
        let home_dir = BaseDirs::new().unwrap().home_dir().to_path_buf();
        let export_file = home_dir.join(DEFAULT_EXPORT_FILE);
        assert!(matches!(get_export_file(None), Ok(export_file)));
        // Relative path
        let current_dir = current_dir().unwrap();
        let export_file = current_dir.join("export.sh");
        assert!(matches!(
            get_export_file(Some(PathBuf::from("export.sh"))),
            Ok(export_file)
        ));
        // Absolute path
        let export_file = PathBuf::from("/home/user/export.sh");
        assert!(matches!(
            get_export_file(Some(PathBuf::from("/home/user/export.sh"))),
            Ok(export_file)
        ));
        // Path is a directory instead of a file
        assert!(get_export_file(Some(home_dir)).is_err());
    }

    #[test]
    fn test_create_export_file() {
        // Creates the export file and writes the correct content to it
        let temp_dir = tempfile::TempDir::new().unwrap();
        let export_file = temp_dir.path().join("export.sh");
        let exports = vec![
            "export VAR1=value1".to_string(),
            "export VAR2=value2".to_string(),
        ];
        create_export_file(&export_file, &exports).unwrap();
        let contents = std::fs::read_to_string(export_file).unwrap();
        assert_eq!(contents, "export VAR1=value1\nexport VAR2=value2\n");

        // Returns the correct error when it fails to create the export file (it already exists)
        let temp_dir = tempfile::TempDir::new().unwrap();
        let export_file = temp_dir.path().join("export.sh");
        std::fs::create_dir_all(&export_file).unwrap();
        let exports = vec![
            "export VAR1=value1".to_string(),
            "export VAR2=value2".to_string(),
        ];
        assert!(create_export_file(&export_file, &exports).is_err());
    }
}
