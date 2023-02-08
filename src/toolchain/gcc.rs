//! GCC Toolchain source and installation tools

use super::Installable;
use crate::{
    emoji, error::Error, host_triple::HostTriple, targets::Target, toolchain::download_file,
};
use async_trait::async_trait;
use embuild::espidf::EspIdfVersion;
use log::{debug, warn};
use miette::Result;
use std::path::{Path, PathBuf};

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "esp-2021r2-patch5";
const DEFAULT_GCC_VERSION: &str = "8_4_0";
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
    /// Repository release version to use.
    pub release: String,
    /// The repository containing GCC sources.
    pub repository_url: String,
    /// GCC Toolchain path.
    pub path: PathBuf,
    /// GCC Version.
    pub version: String,
}

impl Gcc {
    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        // TODO: Test
        format!("{}/bin", &self.path.to_str().unwrap())
    }

    /// Create a new instance with default values and proper toolchain name.
    pub fn new(target: &Target, host_triple: &HostTriple, toolchain_path: &Path) -> Self {
        let name = get_gcc_name(target);
        let version = DEFAULT_GCC_VERSION.to_string();
        let release = DEFAULT_GCC_RELEASE.to_string();
        let path = toolchain_path
            .join(&name)
            .join(format!("{release}-{version}"));

        Self {
            host_triple: host_triple.clone(),
            name,
            release,
            repository_url: DEFAULT_GCC_REPOSITORY.to_string(),
            path,
            version,
        }
    }

    /// Create a new instance of RISC-V GCC with default values and proper toolchain name.
    pub fn new_riscv(host_triple: &HostTriple, toolchain_path: &Path) -> Self {
        let version = DEFAULT_GCC_VERSION.to_string();
        let release = DEFAULT_GCC_RELEASE.to_string();
        let name = RISCV_GCC.to_string();
        let path = toolchain_path
            .join(&name)
            .join(format!("{release}-{version}"));

        Self {
            host_triple: host_triple.clone(),
            name,
            release,
            repository_url: DEFAULT_GCC_REPOSITORY.to_string(),
            path,
            version,
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
                "{} Previous installation of GCC exists in: '{}'. Reusing this installation.",
                emoji::WARN,
                &self.path.display()
            );
        } else {
            let gcc_file = format!(
                "{}-gcc{}-{}-{}.{}",
                self.name,
                self.version,
                self.release,
                get_arch(&self.host_triple).unwrap(),
                extension
            );
            let gcc_dist_url = format!("{}/{}/{}", self.repository_url, self.release, gcc_file);
            download_file(
                gcc_dist_url,
                &format!("{}.{}", &self.name, extension),
                &self.path.display().to_string(),
                true,
            )
            .await?;
        }
        let mut exports: Vec<String> = Vec::new();

        #[cfg(windows)]
        exports.push(format!("$Env:PATH += \";{}\"", &self.get_bin_path()));
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
        HostTriple::Aarch64AppleDarwin | HostTriple::X86_64AppleDarwin => Ok("macos"),
        HostTriple::X86_64UnknownLinuxGnu => Ok("linux-amd64"),
        HostTriple::Aarch64UnknownLinuxGnu => Ok("linux-arm64"),
        HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => Ok("win64"),
    }
}

/// Gets the artifact extension based on the host triple.
fn get_artifact_extension(host_triple: &HostTriple) -> &str {
    match host_triple {
        HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => "zip",
        _ => "tar.gz",
    }
}

/// Gets the toolchain name based on the Target
pub fn get_gcc_name(target: &Target) -> String {
    let toolchain = match target {
        Target::ESP32 => ESP32_GCC,
        Target::ESP32S2 => ESP32S2_GCC,
        Target::ESP32S3 => ESP32S3_GCC,
        Target::ESP32C2 | Target::ESP32C3 => RISCV_GCC,
    };
    toolchain.to_string()
}

/// Gets the toolchain name based on the Target
pub fn get_ulp_toolchain_name(target: Target, version: Option<&EspIdfVersion>) -> Option<String> {
    match target {
        Target::ESP32 => Some("esp32ulp-elf".to_string()),
        Target::ESP32S2 | Target::ESP32S3 => Some(
            if version
                .map(|version| {
                    version.major > 4
                        || version.major == 4 && version.minor > 4
                        || version.major == 4 && version.minor == 4 && version.patch >= 2
                })
                .unwrap_or(true)
            {
                "esp32ulp-elf".to_string()
            } else {
                "esp32s2ulp-elf".to_string()
            },
        ),
        _ => None,
    }
}
