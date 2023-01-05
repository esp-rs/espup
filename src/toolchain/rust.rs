//! Xtensa Rust Toolchain source and installation tools

use super::Installable;
use crate::{
    emoji,
    error::Error,
    host_triple::HostTriple,
    toolchain::{download_file, espidf::get_dist_path},
};
use async_trait::async_trait;
use directories::BaseDirs;
use embuild::cmd;
use log::{debug, info, warn};
use miette::{IntoDiagnostic, Result};
use regex::Regex;
use reqwest::header;
use retry::{delay::Fixed, retry_with_index};
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Debug};
use std::{env, fs::remove_dir_all, path::PathBuf, process::Stdio};

/// Xtensa Rust Toolchain repository
const DEFAULT_XTENSA_RUST_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download";
/// Xtensa Rust Toolchain API URL
const XTENSA_RUST_LATEST_API_URL: &str =
    "https://api.github.com/repos/esp-rs/rust-build/releases/latest";
const XTENSA_RUST_API_URL: &str = "https://api.github.com/repos/esp-rs/rust-build/releases";

/// Xtensa Rust Toolchain version regex.
const RE_EXTENDED_SEMANTIC_VERSION: &str = r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)\.(?P<subpatch>0|[1-9]\d*)?$";
const RE_SEMANTIC_VERSION: &str =
    r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)?$";

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
    pub async fn get_latest_version() -> Result<String> {
        let mut headers = header::HeaderMap::new();
        headers.insert("Accept", "application/vnd.github.v3+json".parse().unwrap());
        if let Some(token) = env::var_os("GITHUB_TOKEN") {
            headers.insert("Authorization", token.to_string_lossy().parse().unwrap());
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .user_agent("espup")
            .build()
            .unwrap();
        let res = client
            .get(XTENSA_RUST_LATEST_API_URL)
            .headers(headers)
            .send()
            .await
            .into_diagnostic()?
            .text()
            .await
            .into_diagnostic()?;
        let json: serde_json::Value =
            serde_json::from_str(&res).map_err(|_| Error::FailedToSerializeJson)?;
        let mut version = json["tag_name"].to_string();

        version.retain(|c| c != 'v' && c != '"');
        Self::parse_version(&version)?;
        debug!("{} Latest Xtensa Rust version: {}", emoji::DEBUG, version);
        Ok(version)
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
    pub fn parse_version(arg: &str) -> Result<String, Error> {
        debug!("{} Parsing Xtensa Rust version: {}", emoji::DEBUG, arg);
        let re_extended = Regex::new(RE_EXTENDED_SEMANTIC_VERSION).unwrap();
        let re_semver = Regex::new(RE_SEMANTIC_VERSION).unwrap();

        let mut headers = header::HeaderMap::new();
        headers.insert(header::USER_AGENT, "espup".parse().unwrap());
        headers.insert(
            header::ACCEPT,
            "application/vnd.github+json".parse().unwrap(),
        );
        headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
        if let Some(token) = env::var_os("GITHUB_TOKEN") {
            headers.insert(
                // header::AUTHORIZATION,
                // header::HeaderValue::from_str(&token.to_string_lossy()).unwrap(),
                "Authorization",
                format!("Bearer {}", token.to_string_lossy())
                    .parse()
                    .unwrap(),
            );
        }
        let client = reqwest::blocking::Client::new();
        // let res = client
        //     .get(XTENSA_RUST_API_URL)
        //     .headers(headers.clone())
        //     .send()?
        //     .text()?;
        let json = retry_with_index(
            Fixed::from_millis(100),
            |current_try| -> Result<serde_json::Value, Error> {
                println!("{} Try: {}", emoji::DEBUG, current_try);
                let res = client
                    .get(XTENSA_RUST_API_URL)
                    .headers(headers.clone())
                    .send()?
                    .text()?;
                let json: serde_json::Value =
                    serde_json::from_str(&res).map_err(|_| Error::FailedToSerializeJson)?;
                println!("{} JSON: {}", emoji::DEBUG, json);
                if json.is_null() && current_try > 5 {
                    return Err(Error::FailedGithubQuery);
                }
                Ok(json)
            },
        )
        .unwrap();
        if re_semver.is_match(arg) {
            let mut extended_versions: Vec<String> = Vec::new();
            for release in json.as_array().unwrap() {
                let tag_name = release["tag_name"].to_string().replace(['\"', 'v'], "");
                if tag_name.starts_with(arg) {
                    extended_versions.push(tag_name);
                }
            }
            if extended_versions.is_empty() {
                return Err(Error::InvalidXtensaToolchanVersion(arg.to_string()));
            }
            let mut max_version = extended_versions.pop().unwrap();
            let mut max_subpatch = 0;
            for version in extended_versions {
                let subpatch: i8 = re_extended
                    .captures(&version)
                    .and_then(|cap| {
                        cap.name("subpatch")
                            .map(|subpatch| subpatch.as_str().parse().unwrap())
                    })
                    .unwrap();
                if subpatch > max_subpatch {
                    max_subpatch = subpatch;
                    max_version = version;
                }
            }
            return Ok(max_version);
        } else if re_extended.is_match(arg) {
            for release in json.as_array().unwrap() {
                let tag_name = release["tag_name"].to_string().replace(['\"', 'v'], "");
                if tag_name.starts_with(arg) {
                    return Ok(arg.to_string());
                }
            }
        }
        Err(Error::InvalidXtensaToolchanVersion(arg.to_string()))
    }

    /// Removes the Xtensa Rust toolchain.
    pub fn uninstall(&self) -> Result<()> {
        info!("{} Uninstalling Xtensa Rust toolchain", emoji::WRENCH);
        let toolchain_path = self.toolchain_destination.clone();
        #[cfg(windows)]
        let toolchain_path = toolchain_path.join("esp");
        remove_dir_all(&toolchain_path)
            .into_diagnostic()
            .map_err(|_| Error::FailedToRemoveDirectory(toolchain_path.display().to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl Installable for XtensaRust {
    async fn install(&self) -> Result<Vec<String>, Error> {
        #[cfg(unix)]
        let toolchain_path = self.toolchain_destination.clone();
        #[cfg(windows)]
        let toolchain_path = self.toolchain_destination.clone().join("esp");
        if toolchain_path.exists() {
            return Err(Error::XtensaToolchainAlreadyInstalled(
                toolchain_path.display().to_string(),
            ));
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
            )
            .await?;

            info!("{} Installing rust esp toolchain", emoji::WRENCH);
            let arguments = format!(
                "{}/rust-nightly-{}/install.sh --destdir={} --prefix='' --without=rust-docs-json-preview,rust-docs",
                get_dist_path("rust"),
                &self.host_triple,
                self.toolchain_destination.display()
            );
            cmd!("/bin/bash", "-c", arguments)
                .into_inner()
                .stdout(Stdio::null())
                .spawn()?;

            download_file(
                self.src_dist_url.clone(),
                "rust-src.tar.xz",
                &get_dist_path("rust-src"),
                true,
            )
            .await?;
            info!("{} Installing rust-src for esp toolchain", emoji::WRENCH);
            let arguments = format!(
                "{}/rust-src-nightly/install.sh --destdir={} --prefix='' --without=rust-docs-json-preview,rust-docs",
                get_dist_path("rust-src"),
                self.toolchain_destination.display()
            );
            cmd!("/bin/bash", "-c", arguments)
                .into_inner()
                .stdout(Stdio::null())
                .spawn()?;
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
            )
            .await?;
        }

        Ok(vec![]) // No exports
    }
}

#[derive(Hash, Eq, PartialEq, Debug, Clone, Serialize, Deserialize, Default)]
pub struct Crate {
    /// Crate name.
    pub name: String,
}

impl Crate {
    /// Create a crate instance.
    pub fn new(name: &str) -> Self {
        Crate {
            name: name.to_string(),
        }
    }

    /// Parses the extra crates to be installed.
    pub fn parse_crates(arg: &str) -> Result<HashSet<Crate>> {
        Ok(arg.split(',').map(Crate::new).collect())
    }

    pub fn uninstall(extra_crate: &str) -> Result<(), Error> {
        cmd!("cargo", "uninstall", extra_crate, "--quiet")
            .into_inner()
            .stdout(Stdio::null())
            .spawn()?;
        Ok(())
    }
}

#[async_trait]
impl Installable for Crate {
    async fn install(&self) -> Result<Vec<String>, Error> {
        debug!("{} Installing crate: {}", emoji::DEBUG, self.name);

        #[cfg(unix)]
        let crate_path = format!("{}/bin/{}", get_cargo_home().display(), self.name);
        #[cfg(windows)]
        let crate_path = format!("{}/bin/{}.exe", get_cargo_home().display(), self.name);

        if PathBuf::from(crate_path).exists() {
            warn!("{} {} is already installed", emoji::WARN, self.name);
        } else {
            cmd!("cargo", "install", &self.name, "--quiet")
                .into_inner()
                .stdout(Stdio::null())
                .spawn()?;
        }

        Ok(vec![]) // No exports
    }
}

pub struct RiscVTarget {
    /// Nightly version.
    pub nightly_version: String,
}

impl RiscVTarget {
    /// Create a crate instance.
    pub fn new(nightly_version: &str) -> Self {
        RiscVTarget {
            nightly_version: nightly_version.to_string(),
        }
    }

    /// Uninstalls the RISC-V target.
    pub fn uninstall(nightly_version: &str) -> Result<(), Error> {
        info!("{} Uninstalling RISC-V target", emoji::WRENCH);
        cmd!(
            "rustup",
            "target",
            "remove",
            "--toolchain",
            nightly_version,
            "riscv32imc-unknown-none-elf",
            "riscv32imac-unknown-none-elf"
        )
        .into_inner()
        .stdout(Stdio::null())
        .spawn()?;
        Ok(())
    }
}

#[async_trait]
impl Installable for RiscVTarget {
    async fn install(&self) -> Result<Vec<String>, Error> {
        info!("{} Installing RISC-V target", emoji::WRENCH);
        cmd!(
            "rustup",
            "component",
            "add",
            "rust-src",
            "--toolchain",
            &self.nightly_version
        )
        .into_inner()
        .stderr(Stdio::null())
        .spawn()?;
        cmd!(
            "rustup",
            "target",
            "add",
            "--toolchain",
            &self.nightly_version,
            "riscv32imc-unknown-none-elf",
            "riscv32imac-unknown-none-elf"
        )
        .into_inner()
        .stderr(Stdio::null())
        .spawn()?;

        Ok(vec![]) // No exports
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
    PathBuf::from(env::var("CARGO_HOME").unwrap_or_else(|_e| {
        format!(
            "{}",
            BaseDirs::new().unwrap().home_dir().join(".cargo").display()
        )
    }))
}

/// Gets the default rustup home path.
pub fn get_rustup_home() -> PathBuf {
    PathBuf::from(env::var("RUSTUP_HOME").unwrap_or_else(|_e| {
        format!(
            "{}",
            BaseDirs::new()
                .unwrap()
                .home_dir()
                .join(".rustup")
                .display()
        )
    }))
}

/// Checks if rustup and the proper nightly version are installed. If rustup is not installed,
/// it returns an error. If nigthly version is not installed, proceed to install it.
pub async fn check_rust_installation(
    nightly_version: &str,
    host_triple: &HostTriple,
) -> Result<()> {
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
                install_rustup(nightly_version, host_triple).await?;
            } else {
                return Err(Error::RustupDetectionError(e.to_string())).into_diagnostic();
            }
        }
    }

    Ok(())
}

/// Installs rustup
async fn install_rustup(nightly_version: &str, host_triple: &HostTriple) -> Result<(), Error> {
    #[cfg(windows)]
    let rustup_init_path = download_file(
        "https://win.rustup.rs/x86_64".to_string(),
        "rustup-init.exe",
        &get_dist_path("rustup"),
        false,
    )
    .await?;
    #[cfg(unix)]
    let rustup_init_path = download_file(
        "https://sh.rustup.rs".to_string(),
        "rustup-init.sh",
        &get_dist_path("rustup"),
        false,
    )
    .await?;
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
        "--default-host",
        host_triple.to_string(),
        "--profile",
        "minimal",
        "-y",
        "--quiet"
    )
    .into_inner()
    .stdout(Stdio::null())
    .spawn()?;
    #[cfg(not(windows))]
    cmd!(
        "/bin/bash",
        rustup_init_path,
        "--default-toolchain",
        nightly_version,
        "--default-host",
        host_triple.to_string(),
        "--profile",
        "minimal",
        "-y",
        "--quiet"
    )
    .into_inner()
    .stdout(Stdio::null())
    .spawn()?;

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

/// Installs the desired version of the nightly toolchain.
fn install_rust_nightly(version: &str) -> Result<(), Error> {
    info!("{} Installing {} toolchain", emoji::WRENCH, version);
    cmd!(
        "rustup",
        "toolchain",
        "install",
        version,
        "--profile",
        "minimal",
        "--quiet"
    )
    .into_inner()
    .stdout(Stdio::null())
    .spawn()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::toolchain::rust::{get_cargo_home, get_rustup_home, Crate, XtensaRust};
    use directories::BaseDirs;
    use std::collections::HashSet;

    #[test]
    fn test_xtensa_rust_parse_version() {
        assert_eq!(XtensaRust::parse_version("1.65.0.0").unwrap(), "1.65.0.0");
        assert_eq!(XtensaRust::parse_version("1.65.0.1").unwrap(), "1.65.0.1");
        assert_eq!(XtensaRust::parse_version("1.64.0.0").unwrap(), "1.64.0.0");
        assert_eq!(XtensaRust::parse_version("1.63.0").unwrap(), "1.63.0.2");
        assert_eq!(XtensaRust::parse_version("1.65.0").unwrap(), "1.65.0.1");
        assert_eq!(XtensaRust::parse_version("1.64.0").unwrap(), "1.64.0.0");
        assert!(XtensaRust::parse_version("422.0.0").is_err());
        assert!(XtensaRust::parse_version("422.0.0.0").is_err());
        assert!(XtensaRust::parse_version("a.1.1.1").is_err());
        assert!(XtensaRust::parse_version("1.1.1.1.1").is_err());
        assert!(XtensaRust::parse_version("1..1.1").is_err());
        assert!(XtensaRust::parse_version("1._.*.1").is_err());
    }

    #[test]
    #[allow(unused_variables)]
    fn test_parse_crates() {
        let mut crates: HashSet<Crate> = HashSet::new();
        crates.insert(Crate::new("ldproxy"));
        assert!(matches!(Crate::parse_crates("ldproxy"), Ok(crates)));
        let mut crates: HashSet<Crate> = HashSet::new();
        crates.insert(Crate::new("ldproxy"));
        crates.insert(Crate::new("espflash"));
        assert!(matches!(
            Crate::parse_crates("ldproxy, espflash"),
            Ok(crates)
        ));
        let mut crates: HashSet<Crate> = HashSet::new();
        crates.insert(Crate::new("cargo-generate"));
        crates.insert(Crate::new("sccache"));
        assert!(matches!(
            Crate::parse_crates("cargo-generate  sccache"),
            Ok(crates)
        ));
        let mut crates: HashSet<Crate> = HashSet::new();
        crates.insert(Crate::new("cargo-binstall"));
        crates.insert(Crate::new("espmonitor"));
        assert!(matches!(
            Crate::parse_crates("cargo-binstall,espmonitor"),
            Ok(crates)
        ));
    }

    #[test]
    fn test_get_cargo_home() {
        // No CARGO_HOME set
        std::env::remove_var("CARGO_HOME");
        assert_eq!(
            get_cargo_home(),
            BaseDirs::new().unwrap().home_dir().join(".cargo")
        );
        // CARGO_HOME set
        let temp_dir = tempfile::TempDir::new().unwrap();
        let cargo_home = temp_dir.path().to_path_buf();
        std::env::set_var("CARGO_HOME", cargo_home.to_str().unwrap());
        assert_eq!(get_cargo_home(), cargo_home);
    }

    #[test]
    fn test_get_rustup_home() {
        // No RUSTUP_HOME set
        std::env::remove_var("RUSTUP_HOME");
        assert_eq!(
            get_rustup_home(),
            BaseDirs::new().unwrap().home_dir().join(".rustup")
        );
        // RUSTUP_HOME set
        let temp_dir = tempfile::TempDir::new().unwrap();
        let rustup_home = temp_dir.path().to_path_buf();
        std::env::set_var("RUSTUP_HOME", rustup_home.to_str().unwrap());
        assert_eq!(get_rustup_home(), rustup_home);
    }
}
