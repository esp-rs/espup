//! Xtensa Rust Toolchain source and installation tools.

use crate::{
    error::Error,
    host_triple::HostTriple,
    toolchain::{
        download_file,
        gcc::{RISCV_GCC, XTENSA_GCC},
        github_query,
        llvm::CLANG_NAME,
        Installable,
    },
};
use async_trait::async_trait;
use directories::BaseDirs;
use log::{debug, info, warn};
use miette::Result;
use regex::Regex;
#[cfg(unix)]
use std::fs::create_dir_all;
use std::{
    env,
    fmt::Debug,
    fs::read_dir,
    io,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};
#[cfg(unix)]
use tempfile::tempdir_in;
use tokio::fs::{remove_dir_all, remove_file};

/// Xtensa Rust Toolchain repository
const DEFAULT_XTENSA_RUST_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download";
/// Xtensa Rust Toolchain API URL
const XTENSA_RUST_LATEST_API_URL: &str =
    "https://api.github.com/repos/esp-rs/rust-build/releases/latest";
const XTENSA_RUST_API_URL: &str =
    "https://api.github.com/repos/esp-rs/rust-build/releases?page=1&per_page=100";

/// Xtensa Rust Toolchain version regex.
pub const RE_EXTENDED_SEMANTIC_VERSION: &str = r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)\.(?P<subpatch>0|[1-9]\d*)?$";
const RE_SEMANTIC_VERSION: &str =
    r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)?$";

#[derive(Debug, Clone, Default)]
pub struct XtensaRust {
    /// Path to the cargo home directory.
    pub cargo_home: PathBuf,
    /// Xtensa Rust toolchain file.
    pub dist_file: String,
    /// Xtensa Rust toolchain URL.
    pub dist_url: String,
    /// Host triple.
    pub host_triple: String,
    /// LLVM Toolchain path.
    pub path: PathBuf,
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
        let json = tokio::task::spawn_blocking(|| github_query(XTENSA_RUST_LATEST_API_URL))
            .await
            .unwrap()?;
        let mut version = json["tag_name"].to_string();

        version.retain(|c| c != 'v' && c != '"');
        let borrowed = version.clone();
        tokio::task::spawn_blocking(move || Self::parse_version(&borrowed))
            .await
            .expect("Join blocking task error")?;
        debug!("Latest Xtensa Rust version: {}", version);
        Ok(version)
    }

    /// Create a new instance.
    pub fn new(toolchain_version: &str, host_triple: &HostTriple, toolchain_path: &Path) -> Self {
        let artifact_extension = get_artifact_extension(host_triple);
        let version = toolchain_version.to_string();
        let dist = format!("rust-{version}-{host_triple}");
        let dist_file = format!("{dist}.{artifact_extension}");
        let dist_url = format!("{DEFAULT_XTENSA_RUST_REPOSITORY}/v{version}/{dist_file}");
        #[cfg(unix)]
        let src_dist = format!("rust-src-{version}");
        #[cfg(unix)]
        let src_dist_file = format!("{src_dist}.{artifact_extension}");
        #[cfg(unix)]
        let src_dist_url = format!("{DEFAULT_XTENSA_RUST_REPOSITORY}/v{version}/{src_dist_file}");
        let cargo_home = get_cargo_home();
        let rustup_home = get_rustup_home();
        let toolchain_destination = toolchain_path.to_path_buf();

        Self {
            cargo_home,
            dist_file,
            dist_url,
            host_triple: host_triple.to_string(),
            path: toolchain_path.to_path_buf(),
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
        debug!("Parsing Xtensa Rust version: {}", arg);
        let re_extended = Regex::new(RE_EXTENDED_SEMANTIC_VERSION).unwrap();
        let re_semver = Regex::new(RE_SEMANTIC_VERSION).unwrap();
        let json = github_query(XTENSA_RUST_API_URL)?;
        if re_semver.is_match(arg) {
            let mut extended_versions: Vec<String> = Vec::new();
            for release in json.as_array().unwrap() {
                let tag_name = release["tag_name"].to_string().replace(['\"', 'v'], "");
                if tag_name.starts_with(arg) {
                    extended_versions.push(tag_name);
                }
            }
            if extended_versions.is_empty() {
                return Err(Error::InvalidVersion(arg.to_string()));
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
        Err(Error::InvalidVersion(arg.to_string()))
    }

    /// Removes the Xtensa Rust toolchain.
    pub async fn uninstall(toolchain_path: &Path) -> Result<(), Error> {
        info!("Uninstalling Xtensa Rust toolchain");
        let dir = read_dir(toolchain_path)?;
        for entry in dir {
            let entry_path = entry.unwrap().path();
            let entry_name = entry_path.display().to_string();
            if !entry_name.contains(RISCV_GCC)
                && !entry_name.contains(XTENSA_GCC)
                && !entry_name.contains(CLANG_NAME)
            {
                if entry_path.is_dir() {
                    remove_dir_all(Path::new(&entry_name))
                        .await
                        .map_err(|_| Error::RemoveDirectory(entry_name))?;
                } else {
                    remove_file(&entry_name).await?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Installable for XtensaRust {
    async fn install(&self) -> Result<Vec<String>, Error> {
        if self.toolchain_destination.exists() {
            let toolchain_name = format!(
                "+{}",
                self.toolchain_destination
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap(),
            );
            let rustc_version = Command::new("rustc")
                .args([&toolchain_name, "--version"])
                .stdout(Stdio::piped())
                .output()?;
            let output = String::from_utf8_lossy(&rustc_version.stdout);
            if rustc_version.status.success() && output.contains(&self.version) {
                warn!(
                "Previous installation of Xtensa Rust {} exists in: '{}'. Reusing this installation",
                &self.version,
                &self.toolchain_destination.display()
            );
                return Ok(vec![]);
            } else {
                if !rustc_version.status.success() {
                    warn!("Failed to detect version of Xtensa Rust, reinstalling it");
                }
                Self::uninstall(&self.toolchain_destination).await?;
            }
        }

        info!("Installing Xtensa Rust {} toolchain", self.version);

        #[cfg(unix)]
        if cfg!(unix) {
            let path = get_rustup_home().join("tmp");
            if !path.exists() {
                info!("Creating directory: '{}'", path.display());
                create_dir_all(&path)
                    .map_err(|_| Error::CreateDirectory(path.display().to_string()))?;
            }
            let tmp_dir = tempdir_in(path)?;
            let tmp_dir_path = &tmp_dir.path().display().to_string();

            download_file(
                self.src_dist_url.clone(),
                "rust-src.tar.xz",
                tmp_dir_path,
                true,
                false,
            )
            .await?;

            download_file(
                self.dist_url.clone(),
                "rust.tar.xz",
                tmp_dir_path,
                true,
                false,
            )
            .await?;

            info!("Installing 'rust' component for Xtensa Rust toolchain");

            if !Command::new("/usr/bin/env")
                .arg("bash")
                .arg(format!(
                    "{}/rust-nightly-{}/install.sh",
                    tmp_dir_path, &self.host_triple,
                ))
                .arg(format!(
                    "--destdir={}",
                    self.toolchain_destination.display()
                ))
                .arg("--prefix=''")
                .arg("--without=rust-docs-json-preview,rust-docs")
                .arg("--disable-ldconfig")
                .stdout(Stdio::null())
                .output()?
                .status
                .success()
            {
                Self::uninstall(&self.toolchain_destination).await?;
                return Err(Error::XtensaRust);
            }

            info!("Installing 'rust-src' component for Xtensa Rust toolchain");
            if !Command::new("/usr/bin/env")
                .arg("bash")
                .arg(format!("{}/rust-src-nightly/install.sh", tmp_dir_path))
                .arg(format!(
                    "--destdir={}",
                    self.toolchain_destination.display()
                ))
                .arg("--prefix=''")
                .arg("--disable-ldconfig")
                .stdout(Stdio::null())
                .output()?
                .status
                .success()
            {
                Self::uninstall(&self.toolchain_destination).await?;
                return Err(Error::XtensaRustSrc);
            }
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
                true,
            )
            .await?;
        }

        Ok(vec![]) // No exports
    }

    fn name(&self) -> String {
        "Xtensa Rust".to_string()
    }
}

#[derive(Debug, Clone)]
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
        info!("Uninstalling RISC-V target");

        if !Command::new("rustup")
            .args([
                "target",
                "remove",
                "--toolchain",
                nightly_version,
                "riscv32imc-unknown-none-elf",
                "riscv32imac-unknown-none-elf",
                "riscv32imafc-unknown-none-elf",
            ])
            .stdout(Stdio::null())
            .status()?
            .success()
        {
            return Err(Error::UninstallRiscvTarget);
        }
        Ok(())
    }
}

#[async_trait]
impl Installable for RiscVTarget {
    async fn install(&self) -> Result<Vec<String>, Error> {
        info!(
            "Installing RISC-V Rust targets ('riscv32imc-unknown-none-elf', 'riscv32imac-unknown-none-elf' and 'riscv32imafc-unknown-none-elf') for '{}' toolchain",            &self.nightly_version
        );

        if !Command::new("rustup")
            .args([
                "toolchain",
                "install",
                &self.nightly_version,
                "--profile",
                "minimal",
                "--component",
                "rust-src",
                "--target",
                "riscv32imc-unknown-none-elf",
                "riscv32imac-unknown-none-elf",
                "riscv32imafc-unknown-none-elf",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success()
        {
            return Err(Error::InstallRiscvTarget(self.nightly_version.clone()));
        }

        Ok(vec![]) // No exports
    }

    fn name(&self) -> String {
        "RISC-V Rust target".to_string()
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

/// Checks if rustup is installed.
pub async fn check_rust_installation() -> Result<(), Error> {
    info!("Checking Rust installation");

    if let Err(e) = Command::new("rustup")
        .arg("--version")
        .stdout(Stdio::piped())
        .output()
    {
        if let io::ErrorKind::NotFound = e.kind() {
            return Err(Error::MissingRust);
        } else {
            return Err(Error::RustupDetection(e.to_string()));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        logging::initialize_logger,
        toolchain::rust::{get_cargo_home, get_rustup_home, XtensaRust},
    };
    use directories::BaseDirs;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_xtensa_rust_parse_version() {
        initialize_logger("debug");
        assert_eq!(XtensaRust::parse_version("1.65.0.0").unwrap(), "1.65.0.0");
        assert_eq!(XtensaRust::parse_version("1.65.0.1").unwrap(), "1.65.0.1");
        assert_eq!(XtensaRust::parse_version("1.64.0.0").unwrap(), "1.64.0.0");
        assert_eq!(XtensaRust::parse_version("1.82.0").unwrap(), "1.82.0.3");
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
    fn test_get_cargo_home() {
        // No CARGO_HOME set
        env::remove_var("CARGO_HOME");
        assert_eq!(
            get_cargo_home(),
            BaseDirs::new().unwrap().home_dir().join(".cargo")
        );
        // CARGO_HOME set
        let temp_dir = TempDir::new().unwrap();
        let cargo_home = temp_dir.path().to_path_buf();
        env::set_var("CARGO_HOME", cargo_home.to_str().unwrap());
        assert_eq!(get_cargo_home(), cargo_home);
    }

    #[test]
    fn test_get_rustup_home() {
        // No RUSTUP_HOME set
        env::remove_var("RUSTUP_HOME");
        assert_eq!(
            get_rustup_home(),
            BaseDirs::new().unwrap().home_dir().join(".rustup")
        );
        // RUSTUP_HOME set
        let temp_dir = TempDir::new().unwrap();
        let rustup_home = temp_dir.path().to_path_buf();
        env::set_var("RUSTUP_HOME", rustup_home.to_str().unwrap());
        assert_eq!(get_rustup_home(), rustup_home);
    }
}
