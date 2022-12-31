//! GCC Toolchain source and installation tools

use super::Installable;
use crate::{
    emoji,
    error::Error,
    host_triple::HostTriple,
    targets::Target,
    toolchain::{download_file, espidf::get_tool_path},
};
use async_trait::async_trait;
use embuild::espidf::EspIdfVersion;
use log::{debug, warn};
use miette::Result;
use std::{
    fs::remove_dir_all,
    path::{Path, PathBuf},
};

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "esp-2021r2-patch5";
const DEFAULT_GCC_VERSION: &str = "8_4_0";
const ESP32_GCC: &str = "xtensa-esp32-elf";
const ESP32S2_GCC: &str = "xtensa-esp32s2-elf";
const ESP32S3_GCC: &str = "xtensa-esp32s3-elf";
pub const RISCV_GCC: &str = "riscv32-esp-elf";

#[derive(Debug, Clone)]
pub struct Gcc {
    /// Host triple.
    pub host_triple: HostTriple,
    /// Repository release version to use.
    pub release: String,
    /// The repository containing GCC sources.
    pub repository_url: String,
    /// GCC Toolchain target.
    pub toolchain_name: String,
    /// GCC Version.
    pub version: String,
}

impl Gcc {
    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        let toolchain_path = format!(
            "{}/{}-{}/{}/bin",
            &self.toolchain_name, self.release, self.version, &self.toolchain_name
        );
        get_tool_path(&toolchain_path)
    }

    /// Create a new instance with default values and proper toolchain name.
    pub fn new(target: &Target, host_triple: &HostTriple) -> Self {
        Self {
            host_triple: host_triple.clone(),
            release: DEFAULT_GCC_RELEASE.to_string(),
            repository_url: DEFAULT_GCC_REPOSITORY.to_string(),
            toolchain_name: get_toolchain_name(target),
            version: DEFAULT_GCC_VERSION.to_string(),
        }
    }

    /// Create a new instance of RISC-V GCC with default values and proper toolchain name.
    pub fn new_riscv(host_triple: &HostTriple) -> Self {
        Self {
            host_triple: host_triple.clone(),
            release: DEFAULT_GCC_RELEASE.to_string(),
            repository_url: DEFAULT_GCC_REPOSITORY.to_string(),
            toolchain_name: String::from("riscv32-esp-elf"),
            version: DEFAULT_GCC_VERSION.to_string(),
        }
    }

    /// Uninstall the GCC toolchain for the desired target.
    pub fn uninstall(target: &Target) -> Result<(), Error> {
        let gcc_path = get_tool_path(&get_toolchain_name(target));
        remove_dir_all(&gcc_path).map_err(|_| Error::FailedToRemoveDirectory(gcc_path))?;
        Ok(())
    }

    /// Uninstall the RISC-V GCC toolchain.
    pub fn uninstall_riscv() -> Result<(), Error> {
        let riscv_gcc_path = get_tool_path(RISCV_GCC);
        remove_dir_all(&riscv_gcc_path)
            .map_err(|_| Error::FailedToRemoveDirectory(riscv_gcc_path))?;
        Ok(())
    }
}

#[async_trait]
impl Installable for Gcc {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let target_dir = format!("{}/{}-{}", self.toolchain_name, self.release, self.version);
        let gcc_path = get_tool_path(&target_dir);
        let extension = get_artifact_extension(&self.host_triple);
        debug!("{} GCC path: {}", emoji::DEBUG, gcc_path);
        if Path::new(&PathBuf::from(&gcc_path)).exists() {
            warn!(
                "{} Previous installation of GCC exists in: '{}'. Reusing this installation.",
                emoji::WARN,
                &gcc_path
            );
        } else {
            let gcc_file = format!(
                "{}-gcc{}-{}-{}.{}",
                self.toolchain_name,
                self.version,
                self.release,
                get_arch(&self.host_triple).unwrap(),
                extension
            );
            let gcc_dist_url = format!("{}/{}/{}", self.repository_url, self.release, gcc_file);
            download_file(
                gcc_dist_url,
                &format!("{}.{}", &self.toolchain_name, extension),
                &gcc_path,
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
pub fn get_toolchain_name(target: &Target) -> String {
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
