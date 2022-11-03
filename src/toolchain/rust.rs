//! Xtensa Rust Toolchain source and installation tools

#[cfg(unix)]
use super::espidf::get_dist_path;
use crate::{
    emoji,
    host_triple::HostTriple,
    toolchain::{download_file, get_home_dir},
};
use anyhow::{bail, Result};
use embuild::cmd;
use log::{debug, info, warn};
use regex::Regex;
use reqwest::header;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::{env, fs::remove_dir_all, path::PathBuf, process::Stdio};

/// Xtensa Rust Toolchain repository
const DEFAULT_XTENSA_RUST_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download";
/// Xtensa Rust Toolchain API URL
const XTENSA_RUST_API_URL: &str = "https://api.github.com/repos/esp-rs/rust-build/releases/latest";
/// Xtensa Rust Toolchain version regex.
const RE_TOOLCHAIN_VERSION: &str = r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)\.(?P<subpatch>0|[1-9]\d*)?$";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct XtensaRust {
    /// Path to the cargo home directory.
    pub cargo_home: PathBuf,
    /// Xtensa Rust toolchain file.
    pub dist_file: String,
    /// Xtensa Rust toolchain URL.
    pub dist_url: String,
    /// Host triple.
    pub host_triple: String,
    /// Path to the rustup home directory.
    pub rustup_home: PathBuf,
    #[cfg(unix)]
    /// Xtensa Src Rust toolchain file.
    pub src_dist_file: String,
    #[cfg(unix)]
    /// Xtensa Src Rust toolchain URL.
    pub src_dist_url: String,
    /// Xtensa Rust toolchain destination path.
    pub toolchain_destination: PathBuf,
    /// Xtensa Rust Toolchain version.
    pub version: String,
}

impl XtensaRust {
    /// Get the latest version of Xtensa Rust toolchain.
    pub fn get_latest_version() -> Result<String> {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github.v3+json".parse().unwrap());

        let client = reqwest::blocking::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("foo")
            .build()
            .unwrap();
        let res = client
            .get(XTENSA_RUST_API_URL)
            .headers(headers)
            .send()?
            .text()?;
        let json: serde_json::Value = serde_json::from_str(&res)?;
        let mut version = json["tag_name"].to_string();

        version.retain(|c| c != 'v' && c != '"');
        Self::parse_version(&version)?;
        debug!("{} Latest Xtensa Rust version: {}", emoji::DEBUG, version);
        Ok(version)
    }

    /// Installs the Xtensa Rust toolchain.
    pub fn install(&self) -> Result<()> {
        #[cfg(unix)]
        let toolchain_path = self.toolchain_destination.clone();
        #[cfg(windows)]
        let toolchain_path = self.toolchain_destination.clone().join("esp");
        if toolchain_path.exists() {
            bail!(
                "{} Previous installation of Rust Toolchain exist in: '{}'. Please, remove the directory before new installation.",
                emoji::WARN,
                self.toolchain_destination.display()
            );
        }
        info!(
            "{} Installing Xtensa Rust {} toolchain",
            emoji::WRENCH,
            self.version
        );

        #[cfg(unix)]
        if cfg!(unix) {
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
                &self.host_triple,
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
        // Some platfroms like Windows are available in single bundle rust + src, because install
        // script in dist is not available for the plaform. It's sufficient to extract the toolchain
        #[cfg(windows)]
        if cfg!(windows) {
            download_file(
                self.dist_url.clone(),
                "rust.zip",
                &self.toolchain_destination.display().to_string(),
                true,
            )?;
        }

        Ok(())
    }

    /// Create a new instance.
    pub fn new(toolchain_version: &str, host_triple: &HostTriple) -> Self {
        let artifact_extension = get_artifact_extension(host_triple);
        let version = toolchain_version.to_string();
        let dist = format!("rust-{}-{}", version, host_triple);
        let dist_file = format!("{}.{}", dist, artifact_extension);
        let dist_url = format!(
            "{}/v{}/{}",
            DEFAULT_XTENSA_RUST_REPOSITORY, version, dist_file
        );
        #[cfg(unix)]
        let src_dist = format!("rust-src-{}", version);
        #[cfg(unix)]
        let src_dist_file = format!("{}.{}", src_dist, artifact_extension);
        #[cfg(unix)]
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
            cargo_home,
            dist_file,
            dist_url,
            host_triple: host_triple.to_string(),
            rustup_home,
            #[cfg(unix)]
            src_dist_file,
            #[cfg(unix)]
            src_dist_url,
            toolchain_destination,
            version,
        }
    }

    /// Parses the version of the Xtensa toolchain.
    pub fn parse_version(arg: &str) -> Result<String> {
        debug!("{} Parsing Xtensa Rust version: {}", emoji::DEBUG, arg);
        let re = Regex::new(RE_TOOLCHAIN_VERSION).unwrap();
        if !re.is_match(arg) {
            bail!(
                "{} Invalid toolchain version, must be in the form of '<major>.<minor>.<patch>.<subpatch>'",
                emoji::ERROR
            );
        }
        Ok(arg.to_string())
    }

    /// Removes the Xtensa Rust toolchain.
    pub fn uninstall(&self) -> Result<()> {
        info!("{} Uninstalling Xtensa Rust toolchain", emoji::WRENCH);
        remove_dir_all(&self.toolchain_destination)?;
        Ok(())
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct Crate {
    /// Crate name.
    pub name: String,
}

impl Crate {
    /// Installs a crate.
    pub fn install(&self) -> Result<()> {
        #[cfg(unix)]
        let crate_path = format!("{}/bin/{}", get_cargo_home().display(), self.name);
        #[cfg(windows)]
        let crate_path = format!("{}/bin/{}.exe", get_cargo_home().display(), self.name);
        if PathBuf::from(crate_path).exists() {
            warn!("{} {} is already installed", emoji::WARN, self.name);
            Ok(())
        } else {
            cmd!("cargo", "install", &self.name).run()?;
            Ok(())
        }
    }

    /// Create a crate instance.
    pub fn new(name: &str) -> Self {
        Crate {
            name: name.to_string(),
        }
    }
}

/// Gets the artifact extension based on the host architecture.
fn get_artifact_extension(host_triple: &HostTriple) -> &str {
    match host_triple {
        HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => "zip",
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

/// Checks if rustup and the proper nightly version are installed. If rustup is not installed,
/// it bails. If nigthly version is not installed, proceed to install it.
pub fn check_rust_installation(nightly_version: &str) -> Result<()> {
    info!("{} Checking existing Rust installation", emoji::WRENCH);

    if let Ok(child_output) = cmd!("rustup", "toolchain", "list")
        .into_inner()
        .stdout(Stdio::piped())
        .output()
    {
        let result = String::from_utf8_lossy(&child_output.stdout);
        if !result.contains("nightly") {
            warn!("{} Rust nightly toolchain not found", emoji::WARN);
            install_rust_nightly(nightly_version)?;
        }
    } else {
        bail!("{} rustup was not found. Please, install rustup: https://www.rust-lang.org/tools/install", emoji::ERROR);
    }

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

#[cfg(test)]
mod tests {
    use crate::toolchain::rust::XtensaRust;

    #[test]
    fn test_xtensa_rust_parse_version() {
        assert_eq!(XtensaRust::parse_version("1.45.0.0").unwrap(), "1.45.0.0");
        assert_eq!(XtensaRust::parse_version("1.45.0.1").unwrap(), "1.45.0.1");
        assert_eq!(XtensaRust::parse_version("1.1.1.1").unwrap(), "1.1.1.1");
        assert_eq!(XtensaRust::parse_version("a.1.1.1").is_err(), true);
        assert_eq!(XtensaRust::parse_version("1.1.1.1.1").is_err(), true);
        assert_eq!(XtensaRust::parse_version("1..1.1").is_err(), true);
        assert_eq!(XtensaRust::parse_version("1._.*.1").is_err(), true);
    }
}
