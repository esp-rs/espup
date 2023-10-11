//! GCC Toolchain source and installation tools.

use crate::{
    emoji,
    error::Error,
    host_triple::HostTriple,
    targets::Target,
    toolchain::{download_file, Installable},
};
use async_trait::async_trait;
use log::{debug, info, warn};
use miette::Result;
use std::{
    fs::remove_dir_all,
    path::{Path, PathBuf},
};

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "12.2.0_20230208";
pub const ESP32_GCC: &str = "xtensa-esp32-elf";
pub const ESP32S2_GCC: &str = "xtensa-esp32s2-elf";
pub const ESP32S3_GCC: &str = "xtensa-esp32s3-elf";
pub const RISCV_GCC: &str = "riscv32-esp-elf";

#[derive(Debug, Clone)]
pub struct Gcc {
    /// Host triple.
    pub host_triple: HostTriple,
    /// GCC Toolchain name.
    pub name: String,
    /// GCC Toolchain path.
    pub path: PathBuf,
}

impl Gcc {
    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        format!("{}/{}/bin", &self.path.to_str().unwrap(), &self.name)
    }

    /// Create a new instance with default values and proper toolchain name.
    pub fn new(target: &Target, host_triple: &HostTriple, toolchain_path: &Path) -> Self {
        let name = get_gcc_name(target);
        let path = toolchain_path
            .join(&name)
            .join(format!("esp-{DEFAULT_GCC_RELEASE}"));

        Self {
            host_triple: host_triple.clone(),
            name,
            path,
        }
    }

    /// Create a new instance of RISC-V GCC with default values and proper toolchain name.
    pub fn new_riscv(host_triple: &HostTriple, toolchain_path: &Path) -> Self {
        let name = RISCV_GCC.to_string();
        let path = toolchain_path
            .join(&name)
            .join(format!("esp-{DEFAULT_GCC_RELEASE}"));

        Self {
            host_triple: host_triple.clone(),
            name,
            path,
        }
    }
}

#[async_trait]
impl Installable for Gcc {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let extension = get_artifact_extension(&self.host_triple);
        debug!("{} GCC path: {}", emoji::DEBUG, self.path.display());
        if self.path.exists() {
            warn!(
                "{} Previous installation of GCC exists in: '{}'. Reusing this installation",
                emoji::WARN,
                &self.path.display()
            );
        } else {
            let gcc_file = format!(
                "{}-{}-{}.{}",
                self.name,
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
                &format!("{}.{}", &self.name, extension),
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
            std::env::set_var(
                "PATH",
                self.get_bin_path().replace('/', "\\") + ";" + &std::env::var("PATH").unwrap(),
            );
        }
        #[cfg(unix)]
        exports.push(format!("export PATH=\"{}:$PATH\"", &self.get_bin_path()));

        Ok(exports)
    }

    fn name(&self) -> String {
        format!("GCC ({})", self.name)
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

/// Gets the toolchain name based on the Target
pub fn get_gcc_name(target: &Target) -> String {
    let toolchain = match target {
        Target::ESP32 => ESP32_GCC,
        Target::ESP32S2 => ESP32S2_GCC,
        Target::ESP32S3 => ESP32S3_GCC,
        Target::ESP32C2 | Target::ESP32C3 | Target::ESP32C6 | Target::ESP32H2 => RISCV_GCC,
    };
    toolchain.to_string()
}

/// Checks if the toolchain is pressent, if present uninstalls it.
pub fn uninstall_gcc_toolchains(toolchain_path: &Path) -> Result<(), Error> {
    info!("{} Uninstalling GCC", emoji::WRENCH);

    let gcc_toolchains = vec![ESP32_GCC, ESP32S2_GCC, ESP32S3_GCC, RISCV_GCC];

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
                std::env::set_var(
                    "PATH",
                    std::env::var("PATH")
                        .unwrap()
                        .replace(&format!("{gcc_path};"), ""),
                );
            }
            remove_dir_all(&gcc_path)
                .map_err(|_| Error::RemoveDirectory(gcc_path.display().to_string()))?;
        }
    }

    Ok(())
}
