//! GCC Toolchain source and installation tools

use crate::chip::Chip;
use crate::emoji;
use crate::utils::{download_file, get_tool_path};
use anyhow::Result;
use embuild::espidf::EspIdfVersion;
use log::debug;

const DEFAULT_GCC_REPOSITORY: &str = "https://github.com/espressif/crosstool-NG/releases/download";
const DEFAULT_GCC_RELEASE: &str = "esp-2021r2-patch3";
const DEFAULT_GCC_VERSION: &str = "gcc8_4_0-esp-2021r2-patch3";

#[derive(Debug)]
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
    /// Gets the binary path.
    pub fn get_bin_path(&self) -> String {
        format!("{}/bin", get_tool_path(&self.toolchain_name))
    }

    /// Installs the gcc toolchain.
    pub fn install(&self) -> Result<()> {
        let gcc_path = get_tool_path(&self.toolchain_name);
        let host_triple = guess_host_triple::guess_host_triple().unwrap();
        let extension = get_artifact_extension(host_triple);
        debug!("{} GCC path: {}", emoji::DEBUG, gcc_path);
        let gcc_file = format!(
            "{}-{}-{}.{}",
            self.toolchain_name,
            self.version,
            get_arch(host_triple).unwrap(),
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
            toolchain_name: get_toolchain_name(chip),
        }
    }
}

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

/// Gets the toolchain name based on the Chip
pub fn get_toolchain_name(chip: Chip) -> String {
    match chip {
        Chip::ESP32 => "xtensa-esp32-elf".to_string(),
        Chip::ESP32S2 => "xtensa-esp32s2-elf".to_string(),
        Chip::ESP32S3 => "xtensa-esp32s3-elf".to_string(),
        Chip::ESP32C3 => "riscv32-esp-elf".to_string(),
    }
}

/// Gets the toolchain name based on the Chip
pub fn get_ulp_toolchain_name(chip: Chip, version: Option<&EspIdfVersion>) -> Option<String> {
    match chip {
        Chip::ESP32 => Some("esp32ulp-elf".to_string()),
        Chip::ESP32S2 | Chip::ESP32S3 => Some(
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

/// Installs GCC toolchain the selected chips.
pub fn install_gcc_targets(targets: Vec<Chip>) -> Result<Vec<String>> {
    let mut exports: Vec<String> = Vec::new();
    for target in targets {
        let gcc = GccToolchain::new(target);
        gcc.install()?;

        #[cfg(windows)]
        exports.push(format!("$Env:PATH += \"{}\"", gcc.get_bin_path()));
        #[cfg(unix)]
        exports.push(format!("export PATH={}:$PATH", gcc.get_bin_path()));
    }
    Ok(exports)
}
