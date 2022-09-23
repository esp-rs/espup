//! Xtensa Rust Toolchain source and installation tools

use super::InstallOpts;
use crate::chip::Chip;
use crate::emoji;
use crate::espidf::get_dist_path;
use crate::utils::{download_file, get_home_dir};
use anyhow::{bail, Result};
use embuild::cmd;
use log::{info, warn};
use std::{env, path::PathBuf, process::Stdio};

const DEFAULT_XTENSA_RUST_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download";

#[derive(Debug)]
pub struct RustToolchain {
    /// Xtensa Rust toolchain file.
    pub dist_file: String,
    /// Xtensa Rust toolchain url.
    pub dist_url: String,
    /// Xtensa Src Rust toolchain file.
    pub src_dist_file: String,
    /// Xtensa Src Rust toolchain url.
    pub src_dist_url: String,
    /// ESP targets.
    pub targets: Vec<Chip>,
    /// Extra crates to install.
    pub extra_crates: String,
    /// Nightly version to install.
    pub nightly_version: String,
    /// Path to the cargo home directory.
    pub cargo_home: PathBuf,
    /// Path to the rustup home directory.
    pub rustup_home: PathBuf,
    /// Xtensa Rust toolchain destination path.
    pub toolchain_destination: PathBuf,
    /// Xtensa Rust Toolchain version.
    pub version: String,
}

impl RustToolchain {
    /// Installs the RiscV target.
    pub fn install_riscv_target(&self) -> Result<()> {
        info!("{} Installing Riscv target", emoji::WRENCH);
        cmd!(
            "rustup",
            "component",
            "add",
            "rust-src",
            "--toolchain",
            self.nightly_version.clone()
        )
        .run()?;
        cmd!(
            "rustup",
            "target",
            "add",
            "--toolchain",
            self.nightly_version.clone(),
            "riscv32imac-unknown-none-elf"
        )
        .run()?;
        Ok(())
    }

    /// Installs the Xtensa Rust toolchain.
    pub fn install_xtensa_rust(&self) -> Result<()> {
        #[cfg(unix)]
        let toolchain_path = self.toolchain_destination.clone();
        #[cfg(windows)]
        let toolchain_path = self.toolchain_destination.clone().join("esp");
        if toolchain_path.exists() {
            bail!(
                "{} Previous installation of Rust Toolchain exist in: {}.\n Please, remove the directory before new installation.",
                emoji::WARN,
                self.toolchain_destination.display()
            );
        }
        info!("{} Installing Xtensa Rust toolchain", emoji::WRENCH);

        let host_triple = guess_host_triple::guess_host_triple().unwrap();

        // Some platfroms like Windows are available in single bundle rust + src, because install
        // script in dist is not available for the plaform. It's sufficient to extract the toolchain
        if get_installer(host_triple).to_string().is_empty() {
            download_file(
                self.dist_url.clone(),
                "rust.zip",
                &self.toolchain_destination.display().to_string(),
                true,
            )?;
        } else {
            download_file(
                self.dist_url.clone(),
                "rust.tar.xz",
                &get_dist_path("rust"),
                true,
            )?;
            info!("{} Installing rust esp toolchain", emoji::WRENCH);
            let arguments = format!(
                "{}/rust-nightly-{}/install.sh --destdir={} --prefix='' --without=rust-docs",
                get_dist_path("rust"),
                host_triple,
                self.toolchain_destination.display()
            );
            cmd!("/bin/bash", "-c", arguments).run()?;

            download_file(
                self.src_dist_url.clone(),
                "rust-src.tar.xz",
                &get_dist_path("rust-src"),
                true,
            )?;
            info!("{} Installing rust-src for esp toolchain", emoji::WRENCH);
            let arguments = format!(
                "{}/rust-src-nightly/install.sh --destdir={} --prefix='' --without=rust-docs",
                get_dist_path("rust-src"),
                self.toolchain_destination.display()
            );
            cmd!("/bin/bash", "-c", arguments).run()?;
        }
        Ok(())
    }

    // TODO: Some fields are not needed in Windows
    /// Create a new instance.
    pub fn new(args: &InstallOpts, arch: &str, targets: &[Chip]) -> Self {
        let artifact_extension = get_artifact_extension(arch);
        let version = args.toolchain_version.clone();
        let dist = format!("rust-{}-{}", args.toolchain_version, arch);
        let dist_file = format!("{}.{}", dist, artifact_extension);
        let dist_url = format!(
            "{}/v{}/{}",
            DEFAULT_XTENSA_RUST_REPOSITORY, version, dist_file
        );
        let src_dist = format!("rust-src-{}", args.toolchain_version);
        let src_dist_file = format!("{}.{}", src_dist, artifact_extension);
        let src_dist_url = format!(
            "{}/v{}/{}",
            DEFAULT_XTENSA_RUST_REPOSITORY, version, src_dist_file
        );
        let cargo_home = get_cargo_home();
        let rustup_home = get_rustup_home();
        #[cfg(unix)]
        let default_toolchain_destination = rustup_home.join("toolchains").join("esp");
        #[cfg(windows)]
        let default_toolchain_destination = rustup_home.join("toolchains");
        let toolchain_destination = args
            .toolchain_destination
            .clone()
            .unwrap_or(default_toolchain_destination);
        Self {
            dist_file,
            dist_url,
            src_dist_file,
            src_dist_url,
            targets: targets.to_vec(),
            extra_crates: args.extra_crates.clone(),
            nightly_version: args.nightly_version.clone(),
            cargo_home,
            rustup_home,
            toolchain_destination,
            version,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BinstallCrate {
    /// Crate version.
    pub url: String,
    /// Crate source.
    pub bin: String,
    /// Crate destination.
    pub fmt: String,
}
#[derive(Debug, Clone)]
pub struct RustCrate {
    /// Crate name.
    pub name: String,
    /// Binary.
    pub binstall: Option<BinstallCrate>,
}

/// Gets the artifact extension based on the host architecture.
fn get_artifact_extension(host_triple: &str) -> &str {
    match host_triple {
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "zip",
        _ => "tar.xz",
    }
}

/// Gets the default cargo home path.
fn get_cargo_home() -> PathBuf {
    PathBuf::from(env::var("CARGO_HOME").unwrap_or_else(|_e| get_home_dir() + "/.cargo"))
}

/// Gets the default rustup home path.
fn get_rustup_home() -> PathBuf {
    PathBuf::from(env::var("RUSTUP_HOME").unwrap_or_else(|_e| get_home_dir() + "/.rustup"))
}

/// Gets the installer file.
fn get_installer(host_triple: &str) -> &str {
    match host_triple {
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "",
        _ => "./install.sh",
    }
}

/// Checks if rustup and the propper nightly version are installed. If they are
/// not, proceed to install them.
pub fn check_rust_installation(nightly_version: &str) -> Result<()> {
    info!("{} Checking existing Rust installation", emoji::WRENCH);
    match std::process::Command::new("rustup")
        .arg("toolchain")
        .arg("list")
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            let result = String::from_utf8_lossy(&child_output.stdout);
            if !result.contains("nightly") {
                warn!("{} Rust nightly toolchain not found", emoji::WARN);
                install_rust_nightly(nightly_version)?;
            }
        }
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                warn!("{} rustup was not found.", emoji::WARN);
                install_rustup(nightly_version)?;
            } else {
                bail!("{} Error: {}", emoji::ERROR, e);
            }
        }
    }
    Ok(())
}

/// Retuns the RustCrate from a given name.
pub fn get_rust_crate(name: &str) -> RustCrate {
    // match name {
    // "ldproxy" => {
    //     RustCrate {
    //         name: name.to_string(),
    //         binstall: Some(BinstallCrate {
    //             url: "{ repo }/releases/download/{ name }-v{ version }/{ name }-{ target }.{ archive-format }".to_string(),
    //             bin: "{ bin }{ binary-ext }".to_string(),
    //             fmt: "zip".to_string(),
    //         }),
    //     }
    // }
    // "espflash" => {
    //     RustCrate {
    //         name: name.to_string(),
    //         binstall: Some(BinstallCrate {
    //             url: "{ repo }/releases/download/{ name }-v{ version }/{ name }-{ target }.{ archive-format }".to_string(),
    //             bin: "{ bin }{ binary-ext }".to_string(),
    //             fmt: "zip".to_string(),
    //         }),
    //     }
    // }

    // "cargo-generate" => {

    // },

    // "wokwi-server" => {

    // },
    // "web-flash" => {

    // },
    // _ => RustCrate {
    //     name: name.to_string(),
    //     binstall: None,
    // },
    // }
    RustCrate {
        name: name.to_string(),
        binstall: None,
    }
}

/// Installs an extra crate.
pub fn install_crate(rust_crate: RustCrate) -> Result<()> {
    info!("{} Installing {} crate", emoji::WRENCH, rust_crate.name);
    cmd!("cargo", "install", rust_crate.name).run()?;
    Ok(())
}

/// Installs rustup
fn install_rustup(nightly_version: &str) -> Result<()> {
    #[cfg(windows)]
    let rustup_init_path = download_file(
        "https://win.rustup.rs/x86_64".to_string(),
        "rustup-init.exe",
        &get_dist_path("rustup"),
        false,
    )?;
    #[cfg(unix)]
    let rustup_init_path = download_file(
        "https://sh.rustup.rs".to_string(),
        "rustup-init.sh",
        &get_dist_path("rustup"),
        false,
    )?;
    info!(
        "{} Installing rustup with {} toolchain",
        emoji::WRENCH,
        nightly_version
    );

    #[cfg(windows)]
    cmd!(
        rustup_init_path,
        "--default-toolchain",
        nightly_version,
        "--profile",
        "minimal",
        "-y"
    )
    .run()?;
    #[cfg(not(windows))]
    cmd!(
        "/bin/bash",
        rustup_init_path,
        "--default-toolchain",
        nightly_version,
        "--profile",
        "minimal",
        "-y"
    )
    .run()?;
    Ok(())
}

/// Installs the dessired version of the nightly toolchain.
fn install_rust_nightly(version: &str) -> Result<()> {
    info!("{} Installing {} toolchain", emoji::WRENCH, version);
    cmd!(
        "rustup",
        "toolchain",
        "install",
        version,
        "--profile",
        "minimal"
    )
    .run()?;
    Ok(())
}
