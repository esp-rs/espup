//! Xtensa Rust Toolchain source and installation tools

use crate::{
    emoji,
    toolchain::{download_file, espidf::get_dist_path, get_home_dir},
};
use anyhow::{bail, Result};
use embuild::cmd;
use log::{info, warn};
use std::fmt::Debug;
use std::{env, path::PathBuf, process::Stdio};

const DEFAULT_XTENSA_RUST_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download";

#[derive(Debug)]
pub struct RustToolchain {
    /// Xtensa Rust toolchain file.
    pub dist_file: String,
    /// Xtensa Rust toolchain URL.
    pub dist_url: String,
    /// Xtensa Src Rust toolchain file.
    pub src_dist_file: String,
    /// Xtensa Src Rust toolchain URL.
    pub src_dist_url: String,
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
        info!(
            "{} Installing Xtensa Rust {} toolchain",
            emoji::WRENCH,
            self.version
        );

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
    pub fn new(toolchain_version: String) -> Self {
        let host_triple = guess_host_triple::guess_host_triple().unwrap();
        let artifact_extension = get_artifact_extension(host_triple);
        let version = toolchain_version;
        let dist = format!("rust-{}-{}", version, host_triple);
        let dist_file = format!("{}.{}", dist, artifact_extension);
        let dist_url = format!(
            "{}/v{}/{}",
            DEFAULT_XTENSA_RUST_REPOSITORY, version, dist_file
        );
        let src_dist = format!("rust-src-{}", version);
        let src_dist_file = format!("{}.{}", src_dist, artifact_extension);
        let src_dist_url = format!(
            "{}/v{}/{}",
            DEFAULT_XTENSA_RUST_REPOSITORY, version, src_dist_file
        );
        let cargo_home = get_cargo_home();
        let rustup_home = get_rustup_home();
        #[cfg(unix)]
        let toolchain_destination = rustup_home.join("toolchains").join("esp");
        #[cfg(windows)]
        let toolchain_destination = rustup_home.join("toolchains");
        Self {
            dist_file,
            dist_url,
            src_dist_file,
            src_dist_url,
            cargo_home,
            rustup_home,
            toolchain_destination,
            version,
        }
    }
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct BinstallCrate {
    /// Crate version.
    pub url: String,
    /// Crate source.
    pub bin: String,
    /// Crate destination.
    pub fmt: String,
}

#[derive(Hash, Eq, PartialEq, Debug)]
pub struct RustCrate {
    /// Crate name.
    pub name: String,
    /// Binary.
    pub binstall: Option<BinstallCrate>,
}

impl RustCrate {
    /// Installs a crate.
    pub fn install(&self) -> Result<()> {
        let output = cmd!("cargo", "install", "--list").stdout()?;
        if output.contains(&self.name) {
            warn!("{} {} is already installed", emoji::WARN, self.name);
            Ok(())
        } else {
            info!("{} Installing {} crate", emoji::WRENCH, self.name);
            if let Some(binstall) = &self.binstall {
                // TODO: Fix this as is not picking the arguments properly
                if !output.contains("cargo-binstall") {
                    info!("{} Installing cargo-binstall crate", emoji::WRENCH);
                    cmd!("cargo", "install", "cargo-binstall").run()?;
                }
                println!(
                    "cargo binstall --no-confirm --pkg-url {} --pkg-fmt {} --bin-dir {}",
                    binstall.url, binstall.fmt, binstall.bin
                );
                cmd!(
                    "cargo",
                    "binstall",
                    "--no-confirm",
                    "--pkg-url",
                    &binstall.url,
                    "--pkg-fmt",
                    &binstall.fmt,
                    "--bin-dir",
                    &binstall.bin,
                    &self.name
                )
                .run()?;
            } else {
                cmd!("cargo", "install", &self.name).run()?;
            }
            Ok(())
        }
    }

    /// Create a crate instance.
    pub fn new(name: &str) -> Self {
        RustCrate {
            name: name.to_string(),
            binstall: None,
        }
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
        //     RustCrate {
        //         name: name.to_string() + "@0.15.2",
        //         binstall: Some(BinstallCrate {
        //             url: "{ repo }/releases/download/v{ version }/{ name }-{ version }-{ target }.{ archive-format }".to_string(),
        //             bin: "{ bin }{ binary-ext }".to_string(),
        //             fmt: "tgz".to_string(),
        //         }),
        //     }
        // }
        // "sccache" => RustCrate {
        //     name: name.to_string(),
        //     binstall: Some(BinstallCrate {
        //         url: "".to_string(),
        //         bin: "".to_string(),
        //         fmt: "".to_string(),
        //     }),
        // },
        // "wokwi-server" => RustCrate {
        //     name: name.to_string(),
        //     binstall: Some(BinstallCrate {
        //         url: "".to_string(),
        //         bin: "".to_string(),
        //         fmt: "".to_string(),
        //     }),
        // },
        // "web-flash" => RustCrate {
        //     name: name.to_string(),
        //     binstall: Some(BinstallCrate {
        //         url: "".to_string(),
        //         bin: "".to_string(),
        //         fmt: "".to_string(),
        //     }),
        // },
        // _ => RustCrate {
        //      name: name.to_string(),
        //      binstall: None,
        //   },
        // }
    }
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
pub fn get_rustup_home() -> PathBuf {
    PathBuf::from(env::var("RUSTUP_HOME").unwrap_or_else(|_e| get_home_dir() + "/.rustup"))
}

/// Gets the installer file.
fn get_installer(host_triple: &str) -> &str {
    match host_triple {
        "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "",
        _ => "./install.sh",
    }
}

/// Checks if rustup and the proper nightly version are installed. If they are
/// not, proceed to install them.
pub fn check_rust_installation(nightly_version: &str) -> Result<()> {
    info!("{} Checking existing Rust installation", emoji::WRENCH);

    match cmd!("rustup", "toolchain", "list")
        .into_inner()
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

    #[cfg(windows)]
    let path = format!(
        "{};{}",
        std::env::var("PATH").unwrap(),
        get_cargo_home().join("bin").display()
    );
    #[cfg(unix)]
    let path = format!(
        "{}:{}",
        std::env::var("PATH").unwrap(),
        get_cargo_home().join("bin").display()
    );

    std::env::set_var("PATH", path);
    warn!(
        "{} Please restart your terminal after the installation for the changes to take effect.",
        emoji::WARN
    );

    Ok(())
}

/// Installs the RiscV target.
pub fn install_riscv_target(nightly_version: &str) -> Result<()> {
    info!("{} Installing Riscv target", emoji::WRENCH);
    cmd!(
        "rustup",
        "component",
        "add",
        "rust-src",
        "--toolchain",
        nightly_version
    )
    .run()?;
    cmd!(
        "rustup",
        "target",
        "add",
        "--toolchain",
        nightly_version,
        "riscv32imac-unknown-none-elf"
    )
    .run()?;
    Ok(())
}

/// Installs the desired version of the nightly toolchain.
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
