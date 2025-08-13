//! Xtensa Rust Toolchain source and installation tools.

use crate::{
    error::Error,
    host_triple::HostTriple,
    toolchain::{
        Installable, download_file,
        gcc::{RISCV_GCC, XTENSA_GCC},
        github_query,
        llvm::CLANG_NAME,
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
pub const RE_EXTENDED_SEMANTIC_VERSION: &str = r"^(?P<major>0|[1-9]\d*)\.(?P<minor>0|[1-9]\d*)\.(?P<patch>0|[1-9]\d*)\.(?P<subpatch>0|[1-9]\d*)$";
/// Matches version strings with 1-4 parts.
pub const RE_ANY_SEMANTIC_VERSION: &str =
    r"^(0|[1-9]\d*)(\.(0|[1-9]\d*)(\.(0|[1-9]\d*)(\.(0|[1-9]\d*))?)?)?$";

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
    pub async fn get_latest_version() -> Result<String, Error> {
        debug!("Querying latest Xtensa Rust version from GitHub API");

        // First, handle the spawn_blocking result
        let query_result = tokio::task::spawn_blocking(|| github_query(XTENSA_RUST_LATEST_API_URL))
            .await
            .map_err(|e| {
                Error::GithubConnectivityError(format!("Failed to query GitHub API: {e}"))
            })?;

        // Then handle the github_query result
        let json = query_result?;

        if !json.is_object() || !json["tag_name"].is_string() {
            return Err(Error::SerializeJson);
        }

        let mut version = json["tag_name"].to_string();
        version.retain(|c| c != 'v' && c != '"');

        // Validate the version format - handle both spawning and parsing errors
        let parse_task =
            tokio::task::spawn_blocking(move || Self::find_latest_version_on_github(&version))
                .await
                .map_err(|_| Error::SerializeJson)?;

        let validated_version = parse_task?;

        debug!("Latest Xtensa Rust version: {validated_version}");
        Ok(validated_version)
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

    /// Retrieves the latest version of the Xtensa toolchain.
    ///
    /// Note that this function issues a GitHub API request to retrieve the latest version of the Xtensa toolchain.
    pub fn find_latest_version_on_github(version: &str) -> Result<String, Error> {
        debug!("Parsing Xtensa Rust version: {version}");
        let json = github_query(XTENSA_RUST_API_URL)?;

        let mut candidates: Vec<String> = Vec::new();
        for release in json.as_array().unwrap() {
            candidates.push(release["tag_name"].to_string().replace(['\"', 'v'], ""));
        }

        Self::find_latest_version(version, &candidates)
    }

    /// Find the latest matching version of the Xtensa toolchain.
    ///
    /// This function takes a version string and a list of candidate versions and returns the latest matching version.
    /// If no matching version is found, it returns an error.
    ///
    /// The list of candidate versions is expected to be given in the extended semantic version format.
    fn find_latest_version(version: &str, candidates: &[String]) -> Result<String, Error> {
        lazy_static::lazy_static! {
            static ref RE_EXTENDED: Regex = Regex::new(RE_EXTENDED_SEMANTIC_VERSION).unwrap();
            static ref RE_ANY_SEMVER: Regex = Regex::new(RE_ANY_SEMANTIC_VERSION).unwrap();
        };

        if !RE_ANY_SEMVER.is_match(version) {
            return Err(Error::InvalidVersion(version.to_string()));
        }

        let extract_version_components = |version: &str| -> (u8, u8, u8, u8) {
            RE_EXTENDED
                .captures(version)
                .and_then(|cap| {
                    let major = cap.name("major").unwrap().as_str().parse().ok()?;
                    let minor = cap.name("minor").unwrap().as_str().parse().ok()?;
                    let patch = cap.name("patch").unwrap().as_str().parse().ok()?;
                    let subpatch = cap.name("subpatch").unwrap().as_str().parse().ok()?;
                    Some((major, minor, patch, subpatch))
                })
                .unwrap_or_else(|| panic!("Version {version} is not in the extended semver format"))
        };

        // Make sure that if we are looking for 1.65.0.x, we don't consider 1.65.1.x or 1.66.0.x
        let candidates = candidates.iter().filter(|v| v.starts_with(version));

        // Now find the latest
        let max_version = candidates
            .map(move |candidate| {
                let components = extract_version_components(candidate.as_str());

                (candidate, components)
            })
            .max_by_key(|(_, components)| *components)
            .map(|(version, _)| version.clone());

        match max_version {
            Some(version) => Ok(version),
            None => Err(Error::VersionNotFound(version.to_string())),
        }
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
                .arg(format!("{tmp_dir_path}/rust-src-nightly/install.sh"))
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
    /// Stable Rust toolchain version.
    pub stable_version: String,
}

impl RiscVTarget {
    /// Create a crate instance.
    pub fn new(stable_version: &str) -> Self {
        RiscVTarget {
            stable_version: stable_version.to_string(),
        }
    }

    /// Uninstalls the RISC-V target.
    pub fn uninstall(stable_version: &str) -> Result<(), Error> {
        info!("Uninstalling RISC-V target");

        if !Command::new("rustup")
            .args([
                "target",
                "remove",
                "--toolchain",
                stable_version,
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
            "Installing RISC-V Rust targets ('riscv32imc-unknown-none-elf', 'riscv32imac-unknown-none-elf' and 'riscv32imafc-unknown-none-elf') for '{}' toolchain",
            &self.stable_version
        );

        if !Command::new("rustup")
            .args([
                "toolchain",
                "install",
                &self.stable_version,
                "--profile",
                "minimal",
                "--component",
                "rust-src",
                "--target",
                "riscv32imc-unknown-none-elf",
                "--target",
                "riscv32imac-unknown-none-elf",
                "--target",
                "riscv32imafc-unknown-none-elf",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?
            .success()
        {
            return Err(Error::InstallRiscvTarget(self.stable_version.clone()));
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
        toolchain::rust::{XtensaRust, get_cargo_home, get_rustup_home},
    };
    use directories::BaseDirs;
    use std::env;
    use tempfile::TempDir;

    #[test]
    fn test_xtensa_rust_parse_version() {
        initialize_logger("debug");
        let candidates = [
            String::from("1.64.0.0"),
            String::from("1.65.0.0"),
            String::from("1.65.0.1"),
            String::from("1.65.1.0"),
            String::from("1.82.0.3"),
        ];
        assert_eq!(
            XtensaRust::find_latest_version("1.65.0.0", &candidates).unwrap(),
            "1.65.0.0"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1.65", &candidates).unwrap(),
            "1.65.1.0"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1.65.0.1", &candidates).unwrap(),
            "1.65.0.1"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1.64.0.0", &candidates).unwrap(),
            "1.64.0.0"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1.82.0", &candidates).unwrap(),
            "1.82.0.3"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1.65.0", &candidates).unwrap(),
            "1.65.0.1"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1.64.0", &candidates).unwrap(),
            "1.64.0.0"
        );
        assert_eq!(
            XtensaRust::find_latest_version("1", &candidates).unwrap(),
            "1.82.0.3"
        );
        assert!(XtensaRust::find_latest_version("1.", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("1.0.", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("1.0.0.", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("422.0.0", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("422.0.0.0", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("a.1.1.1", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("1.1.1.1.1", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("1..1.1", &candidates).is_err());
        assert!(XtensaRust::find_latest_version("1._.*.1", &candidates).is_err());
    }

    #[test]
    fn test_get_cargo_home() {
        // No CARGO_HOME set
        unsafe {
            env::remove_var("CARGO_HOME");
        }
        assert_eq!(
            get_cargo_home(),
            BaseDirs::new().unwrap().home_dir().join(".cargo")
        );
        // CARGO_HOME set
        let temp_dir = TempDir::new().unwrap();
        let cargo_home = temp_dir.path().to_path_buf();
        unsafe {
            env::set_var("CARGO_HOME", cargo_home.to_str().unwrap());
        }
        assert_eq!(get_cargo_home(), cargo_home);
    }

    #[test]
    fn test_get_rustup_home() {
        // No RUSTUP_HOME set
        unsafe {
            env::remove_var("RUSTUP_HOME");
        }
        assert_eq!(
            get_rustup_home(),
            BaseDirs::new().unwrap().home_dir().join(".rustup")
        );
        // RUSTUP_HOME set
        let temp_dir = TempDir::new().unwrap();
        let rustup_home = temp_dir.path().to_path_buf();
        unsafe {
            env::set_var("RUSTUP_HOME", rustup_home.to_str().unwrap());
        }
        assert_eq!(get_rustup_home(), rustup_home);
    }
}
