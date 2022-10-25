//! LLVM Toolchain source and installation tools

use crate::{
    emoji,
    host_triple::HostTriple,
    toolchain::{download_file, espidf::get_tool_path},
};
use anyhow::{bail, Ok, Result};
use log::{info, warn};
use std::path::{Path, PathBuf};

// LLVM 14
const DEFAULT_LLVM_14_COMPLETE_REPOSITORY: &str =
    "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_14_MINIFIED_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download/llvm-project-14.0-minified";
const DEFAULT_LLVM_14_VERSION: &str = "esp-14.0.0-20220415";
// LLVM 15
const DEFAULT_LLVM_15_REPOSITORY: &str =
    "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_15_VERSION: &str = "esp-15.0.0-20221014";

#[derive(Debug)]
pub struct LlvmToolchain {
    /// LLVM Toolchain file name.
    pub file_name: String,
    /// Host triple.
    pub host_triple: HostTriple,
    /// LLVM Toolchain path.
    pub path: PathBuf,
    /// The repository containing LVVM sources.
    pub repository_url: String,
    /// LLVM Version ["14", "15"].
    pub version: String,
}

impl LlvmToolchain {
    /// Gets the name of the LLVM arch based on the host triple.
    fn get_arch<'a>(version: &'a str, host_triple: &'a HostTriple) -> Result<&'a str> {
        if version == "14" {
            match host_triple {
                HostTriple::Aarch64AppleDarwin | HostTriple::X86_64AppleDarwin => Ok("macos"),
                HostTriple::X86_64UnknownLinuxGnu => Ok("linux-amd64"),
                HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => Ok("win64"),
                _ => bail!(
                    "{} No LLVM arch found for the host triple: '{}'",
                    emoji::ERROR,
                    host_triple
                ),
            }
        } else {
            // LLVM 15
            match host_triple {
                HostTriple::Aarch64AppleDarwin => Ok("macos-arm64"),
                HostTriple::X86_64AppleDarwin => Ok("macos"),
                HostTriple::X86_64UnknownLinuxGnu => Ok("linux-amd64"),
                HostTriple::Aarch64UnknownLinuxGnu => Ok("linux-arm64"),
                HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => Ok("win64"),
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

    /// Gets the binary path.
    fn get_lib_path(&self) -> String {
        #[cfg(windows)]
        if self.version == "14" {
            format!("{}/xtensa-esp32-elf-clang/bin", self.path.to_str().unwrap())
        } else {
            format!("{}/esp-clang/lib", self.path.to_str().unwrap())
        }
        #[cfg(unix)]
        if self.version == "14" {
            format!("{}/xtensa-esp32-elf-clang/lib", self.path.to_str().unwrap())
        } else {
            format!("{}/esp-clang/lib", self.path.to_str().unwrap())
        }
    }

    /// Installs the LLVM toolchain.
    pub fn install(&self) -> Result<Vec<String>> {
        let mut exports: Vec<String> = Vec::new();

        if Path::new(&self.path).exists() {
            warn!(
                "{} Previous installation of LLVM exist in: '{}'. Reusing this installation.",
                emoji::WARN,
                self.path.to_str().unwrap()
            );
        } else {
            info!("{} Installing Xtensa elf Clang", emoji::WRENCH);
            download_file(
                self.repository_url.clone(),
                &format!(
                    "idf_tool_xtensa_elf_clang.{}",
                    Self::get_artifact_extension(&self.host_triple)
                ),
                self.path.to_str().unwrap(),
                true,
            )?;
        }
        // Set environment variables.
        #[cfg(windows)]
        exports.push(format!(
            "$Env:LIBCLANG_PATH=\"{}/libclang.dll\"",
            self.get_lib_path()
        ));
        #[cfg(windows)]
        exports.push(format!("$Env:PATH+=\";{}\"", self.get_lib_path()));
        #[cfg(unix)]
        exports.push(format!("export LIBCLANG_PATH=\"{}\"", self.get_lib_path()));

        Ok(exports)
    }

    /// Create a new instance with default values and proper toolchain version.
    pub fn new(version: String, minified: bool, host_triple: &HostTriple) -> Self {
        let mut file_name: String;
        let repository_url: String;
        let path: PathBuf;
        if version == "14" {
            if minified {
                file_name = format!(
                    "xtensa-esp32-elf-llvm{}-{}-{}.{}",
                    get_release_with_underscores(DEFAULT_LLVM_14_VERSION),
                    DEFAULT_LLVM_14_VERSION,
                    host_triple,
                    Self::get_artifact_extension(host_triple)
                );
                repository_url = format!("{}/{}", DEFAULT_LLVM_14_MINIFIED_REPOSITORY, file_name,);
            } else {
                file_name = format!(
                    "xtensa-esp32-elf-llvm{}-{}-{}.{}",
                    get_release_with_underscores(DEFAULT_LLVM_14_VERSION),
                    DEFAULT_LLVM_14_VERSION,
                    Self::get_arch(&version, host_triple).unwrap(),
                    Self::get_artifact_extension(host_triple)
                );
                repository_url = format!(
                    "{}/{}/{}",
                    DEFAULT_LLVM_14_COMPLETE_REPOSITORY, DEFAULT_LLVM_14_VERSION, file_name
                );
            }
            path = PathBuf::from(format!(
                "{}/{}-{}",
                get_tool_path("xtensa-esp32-elf-clang"),
                DEFAULT_LLVM_14_VERSION,
                host_triple
            ));
        } else {
            // version == "15"
            file_name = format!(
                "llvm-{}-{}.tar.xz",
                DEFAULT_LLVM_15_VERSION,
                Self::get_arch(&version, host_triple).unwrap()
            );
            if minified {
                file_name = format!("libs_{}", file_name);
            }
            repository_url = format!(
                "{}/{}/{}",
                DEFAULT_LLVM_15_REPOSITORY, DEFAULT_LLVM_15_VERSION, file_name,
            );
            path = PathBuf::from(format!(
                "{}/{}-{}",
                get_tool_path("xtensa-esp32-elf-clang"),
                DEFAULT_LLVM_15_VERSION,
                host_triple
            ));
        }

        Self {
            file_name,
            host_triple: host_triple.clone(),
            path,
            repository_url,
            version,
        }
    }
}

/// Gets the parsed version name.
fn get_release_with_underscores(version: &str) -> String {
    let version: Vec<&str> = version.split('-').collect();
    let llvm_dot_release = version[1];
    llvm_dot_release.replace('.', "_")
}

#[cfg(test)]
mod tests {
    use crate::toolchain::llvm_toolchain::get_release_with_underscores;

    #[test]
    fn test_get_release_with_underscores() {
        assert_eq!(
            get_release_with_underscores("esp-14.0.0-20220415"),
            "14_0_0".to_string()
        );
    }
}
