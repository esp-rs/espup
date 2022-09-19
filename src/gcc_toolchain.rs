//! GCC Toolchain source and installation tools

use crate::chip::Chip;
use crate::emoji;
use crate::utils::{download_file, get_tool_path};
use anyhow::Result;
use log::debug;

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "esp-2021r2-patch3";
const DEFAULT_GCC_VERSION: &str = "gcc8_4_0-esp-2021r2-patch3";

pub struct GccToolchain {
    /// The repository containing GCC sources.
    pub repository_url: String,
    /// Repository release version to use.
    pub release: String,
    /// GCC Version.
    pub version: String,
    /// GCC Toolchain target.
    pub toolchain_name: String,
}

impl GccToolchain {
    /// Gets the name of the GCC arch based on the host triple.
    fn get_arch(host_triple: &str) -> Result<&str, String> {
        match host_triple {
            "aarch64-apple-darwin" | "x86_64-apple-darwin" => Ok("macos"),
            "aarch64-unknown-linux-gnu" => Ok("linux-arm64"),
            "x86_64-unknown-linux-gnu" => Ok("linux-amd64"),
            "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => Ok("win64"),
            _ => Err(format!(
                "No GCC arch found for the host triple: {}",
                host_triple
            )),
        }
    }

    /// Gets the artifact extension based on the host architecture.
    fn get_artifact_extension(host_triple: &str) -> &str {
        match host_triple {
            "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "zip",
            _ => "tar.gz",
        }
    }

    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        format!("{}/bin", get_tool_path(&self.toolchain_name))
    }

    /// Gets the toolchain name based on the Chip
    fn get_toolchain_name(chip: Chip) -> String {
        match chip {
            Chip::ESP32 => "xtensa-esp32-elf".to_string(),
            Chip::ESP32S2 => "xtensa-esp32s2-elf".to_string(),
            Chip::ESP32S3 => "xtensa-esp32s3-elf".to_string(),
            Chip::ESP32C3 => "riscv32-esp-elf".to_string(),
        }
    }

    /// Installs the gcc toolchain.
    pub fn install(&self) -> Result<()> {
        let gcc_path = get_tool_path(&self.toolchain_name);
        let host_triple = guess_host_triple::guess_host_triple().unwrap();
        let extension = Self::get_artifact_extension(host_triple);
        debug!("{} GCC path: {}", emoji::DEBUG, gcc_path);
        let gcc_file = format!(
            "{}-{}-{}.{}",
            self.toolchain_name,
            self.version,
            Self::get_arch(host_triple).unwrap(),
            extension
        );
        let gcc_dist_url = format!("{}/{}/{}", self.repository_url, self.release, gcc_file);
        download_file(
            gcc_dist_url,
            &format!("{}.{}", &self.toolchain_name, extension),
            &get_tool_path(""),
            true,
        )?;
        Ok(())
    }

    /// Create a new instance with default values and proper toolchain name.
    pub fn new(chip: Chip) -> Self {
        Self {
            repository_url: DEFAULT_GCC_REPOSITORY.to_string(),
            release: DEFAULT_GCC_RELEASE.to_string(),
            version: DEFAULT_GCC_VERSION.to_string(),
            toolchain_name: Self::get_toolchain_name(chip),
        }
    }
}
