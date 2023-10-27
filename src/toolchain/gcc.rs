//! GCC Toolchain source and installation tools.

use crate::{
    error::Error,
    host_triple::HostTriple,
    toolchain::{download_file, Installable},
};
use async_trait::async_trait;
use log::{debug, info, warn};
use miette::Result;
#[cfg(windows)]
use std::env;
use std::path::{Path, PathBuf};
use tokio::fs::remove_dir_all;

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "13.2.0_20230928";
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
}

impl Gcc {
    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        format!("{}/{}/bin", &self.path.to_str().unwrap(), &self.arch)
    }

    /// Create a new instance with default values and proper toolchain name.
    pub fn new(arch: &str, host_triple: &HostTriple, toolchain_path: &Path) -> Self {
        let path = toolchain_path
            .join(arch)
            .join(format!("esp-{DEFAULT_GCC_RELEASE}"));

        Self {
            host_triple: host_triple.clone(),
            arch: arch.to_string(),
            path,
        }
    }
}

#[async_trait]
impl Installable for Gcc {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let extension = get_artifact_extension(&self.host_triple);
        debug!("GCC path: {}", self.path.display());
        if self.path.exists() {
            warn!(
                "Previous installation of GCC exists in: '{}'. Reusing this installation",
                &self.path.display()
            );
        } else {
            let gcc_file = format!(
                "{}-{}-{}.{}",
                self.arch,
                DEFAULT_GCC_RELEASE,
                get_arch(&self.host_triple).unwrap(),
                extension
            );
            let gcc_dist_url = format!(
                "{}/esp-{}/{}",
                DEFAULT_GCC_REPOSITORY, DEFAULT_GCC_RELEASE, gcc_file
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
            exports.push(format!(
                "$Env:PATH = \"{};\" + $Env:PATH",
                &self.get_bin_path()
            ));
            env::set_var(
                "PATH",
                self.get_bin_path().replace('/', "\\") + ";" + &env::var("PATH").unwrap(),
            );
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
pub async fn uninstall_gcc_toolchains(toolchain_path: &Path) -> Result<(), Error> {
    info!("Uninstalling GCC");

    let gcc_toolchains = vec![XTENSA_GCC, RISCV_GCC];

    for toolchain in gcc_toolchains {
        let gcc_path = toolchain_path.join(toolchain);
        if gcc_path.exists() {
            #[cfg(windows)]
            if cfg!(windows) {
                let gcc_path = format!(
                    "{}\\esp-{}\\{}\\bin",
                    gcc_path.display(),
                    DEFAULT_GCC_RELEASE,
                    toolchain
                );
                env::set_var(
                    "PATH",
                    env::var("PATH")
                        .unwrap()
                        .replace(&format!("{gcc_path};"), ""),
                );
            }
            remove_dir_all(&gcc_path)
                .await
                .map_err(|_| Error::RemoveDirectory(gcc_path.display().to_string()))?;
        }
    }

    Ok(())
}
