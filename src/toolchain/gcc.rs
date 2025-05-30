//! GCC Toolchain source and installation tools.

#[cfg(windows)]
use crate::env::{get_windows_path_var, set_env_variable};
use crate::{
    error::Error,
    host_triple::HostTriple,
    toolchain::{Installable, download_file},
};
use async_trait::async_trait;
use log::{debug, info, warn};
use miette::Result;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::{env, fs::File};
use tokio::fs::remove_dir_all;

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "14.2.0_20241119";
pub const RISCV_GCC: &str = "riscv32-esp-elf";
pub const XTENSA_GCC: &str = "xtensa-esp-elf";

#[derive(Debug, Clone)]
pub struct Gcc {
    /// Host triple.
    pub host_triple: HostTriple,
    /// GCC Toolchain architecture.
    pub arch: String,
    /// GCC Toolchain path.
    pub path: PathBuf,
    /// GCC release version.
    pub release_version: String,
}

impl Gcc {
    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        let bin_path = format!("{}/{}/bin", &self.path.to_str().unwrap(), &self.arch);
        match std::cfg!(windows) {
            true => bin_path.replace('/', "\\"),
            false => bin_path,
        }
    }

    /// Create a new instance with default values and proper toolchain name.
    pub fn new(arch: &str, host_triple: &HostTriple, toolchain_path: &Path, release_version: Option<String>) -> Self {
        let release_version = release_version.unwrap_or_else(|| DEFAULT_GCC_RELEASE.to_string());
        
        #[cfg(unix)]
        let path = toolchain_path
            .join(arch)
            .join(format!("esp-{}", release_version));
        #[cfg(windows)]
        let path: PathBuf = toolchain_path.into();

        Self {
            host_triple: host_triple.clone(),
            arch: arch.to_string(),
            path,
            release_version,
        }
    }
}

#[async_trait]
impl Installable for Gcc {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let extension = get_artifact_extension(&self.host_triple);
        info!("Installing GCC ({})", self.arch);
        debug!("GCC path: {}", self.path.display());

        #[cfg(unix)]
        let is_installed = self.path.exists();
        #[cfg(windows)]
        let is_installed = self
            .path
            .join(&self.arch)
            .join(&self.release_version)
            .exists();

        if is_installed {
            warn!(
                "Previous installation of GCC exists in: '{}'. Reusing this installation",
                &self.path.display()
            );
        } else {
            let gcc_file = format!(
                "{}-{}-{}.{}",
                self.arch,
                self.release_version,
                get_arch(&self.host_triple).unwrap(),
                extension
            );
            let gcc_dist_url = format!(
                "{}/esp-{}/{}",
                DEFAULT_GCC_REPOSITORY, self.release_version, gcc_file
            );
            download_file(
                gcc_dist_url,
                &format!("{}.{}", &self.arch, extension),
                &self.path.display().to_string(),
                true,
                false,
            )
            .await?;
        }
        let mut exports: Vec<String> = Vec::new();

        #[cfg(windows)]
        if cfg!(windows) {
            File::create(self.path.join(&self.arch).join(&self.release_version))?;

            exports.push(format!(
                "$Env:PATH = \"{};\" + $Env:PATH",
                &self.get_bin_path()
            ));
            if self.arch == RISCV_GCC {
                unsafe {
                    env::set_var("RISCV_GCC", self.get_bin_path());
                }
            } else {
                unsafe {
                    env::set_var("XTENSA_GCC", self.get_bin_path());
                }
            }
        }
        #[cfg(unix)]
        exports.push(format!("export PATH=\"{}:$PATH\"", &self.get_bin_path()));

        Ok(exports)
    }

    fn name(&self) -> String {
        format!("GCC ({})", self.arch)
    }
}

/// Gets the name of the GCC arch based on the host triple.
fn get_arch(host_triple: &HostTriple) -> Result<&str> {
    match host_triple {
        HostTriple::X86_64AppleDarwin => Ok("x86_64-apple-darwin"),
        HostTriple::Aarch64AppleDarwin => Ok("aarch64-apple-darwin"),
        HostTriple::X86_64UnknownLinuxGnu => Ok("x86_64-linux-gnu"),
        HostTriple::Aarch64UnknownLinuxGnu => Ok("aarch64-linux-gnu"),
        HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => {
            Ok("x86_64-w64-mingw32")
        }
    }
}

/// Gets the artifact extension based on the host triple.
fn get_artifact_extension(host_triple: &HostTriple) -> &str {
    match host_triple {
        HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => "zip",
        _ => "tar.xz",
    }
}

/// Checks if the toolchain is pressent, if present uninstalls it.
pub async fn uninstall_gcc_toolchains(toolchain_path: &Path, release_version: Option<String>) -> Result<(), Error> {
    info!("Uninstalling GCC");
    let release_version = release_version.unwrap_or_else(|| DEFAULT_GCC_RELEASE.to_string());

    let gcc_toolchains = vec![XTENSA_GCC, RISCV_GCC];

    for toolchain in gcc_toolchains {
        let gcc_path = toolchain_path.join(toolchain);
        if gcc_path.exists() {
            #[cfg(windows)]
            if cfg!(windows) {
                let mut updated_path = get_windows_path_var()?;
                let gcc_version_path = format!(
                    "{}\\esp-{}\\{}\\bin",
                    gcc_path.display(),
                    release_version,
                    toolchain
                );
                updated_path = updated_path.replace(&format!("{gcc_version_path};"), "");
                let bin_path = format!("{}\\bin", gcc_path.display());
                updated_path = updated_path.replace(&format!("{bin_path};"), "");

                set_env_variable("PATH", &updated_path)?;
            }
            remove_dir_all(&gcc_path)
                .await
                .map_err(|_| Error::RemoveDirectory(gcc_path.display().to_string()))?;
        }
    }

    Ok(())
}
