//! LLVM Toolchain source and installation tools

use crate::{
    emoji,
    host_triple::HostTriple,
    toolchain::{download_file, espidf::get_tool_path},
};
use anyhow::{bail, Ok, Result};
use log::{info, warn};
use std::path::{Path, PathBuf};

const DEFAULT_LLVM_COMPLETE_REPOSITORY: &str =
    "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_MINIFIED_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download/llvm-project-14.0-minified";
const DEFAULT_LLVM_VERSION: &str = "esp-14.0.0-20220415";

#[derive(Debug, Clone, Default)]
pub struct LlvmToolchain {
    /// LLVM Toolchain file name.
    pub file_name: String,
    /// Host triple.
    pub host_triple: HostTriple,
    /// LLVM Toolchain path.
    pub path: PathBuf,
    /// The repository containing LVVM sources.
    pub repository_url: String,
    /// Repository release version to use.
    pub version: String,
}

impl LlvmToolchain {
    /// Gets the name of the LLVM arch based on the host triple.
    fn get_arch(host_triple: &HostTriple) -> Result<&str> {
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
        let llvm_path = format!("{}/xtensa-esp32-elf-clang/bin", self.path.to_str().unwrap());
        #[cfg(unix)]
        let llvm_path = format!("{}/xtensa-esp32-elf-clang/lib", self.path.to_str().unwrap());
        llvm_path
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
    pub fn new(minified: bool, host_triple: &HostTriple) -> Self {
        let file_name: String;
        let version = DEFAULT_LLVM_VERSION.to_string();
        let repository_url: String;
        if minified {
            file_name = format!(
                "xtensa-esp32-elf-llvm{}-{}-{}.{}",
                get_release_with_underscores(&version),
                &version,
                host_triple,
                Self::get_artifact_extension(host_triple)
            );
            repository_url = format!("{}/{}", DEFAULT_LLVM_MINIFIED_REPOSITORY, file_name,);
        } else {
            file_name = format!(
                "xtensa-esp32-elf-llvm{}-{}-{}.{}",
                get_release_with_underscores(&version),
                &version,
                Self::get_arch(host_triple).unwrap(),
                Self::get_artifact_extension(host_triple)
            );
            repository_url = format!(
                "{}/{}/{}",
                DEFAULT_LLVM_COMPLETE_REPOSITORY, &version, file_name
            );
        }
        let path = format!(
            "{}/{}-{}",
            get_tool_path("xtensa-esp32-elf-clang"),
            version,
            host_triple
        )
        .into();
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
