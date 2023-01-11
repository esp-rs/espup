//! LLVM Toolchain source and installation tools

use super::Installable;
use crate::{
    emoji,
    error::Error,
    host_triple::HostTriple,
    toolchain::{download_file, espidf::get_tool_path},
};
use async_trait::async_trait;
use log::{info, warn};
use miette::Result;
use std::{
    fs::remove_dir_all,
    path::{Path, PathBuf},
};

const DEFAULT_LLVM_REPOSITORY: &str = "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_15_VERSION: &str = "esp-15.0.0-20221201";

#[derive(Debug, Clone, Default)]
pub struct Llvm {
    /// LLVM Toolchain file name.
    pub file_name: String,
    /// Host triple.
    pub host_triple: HostTriple,
    /// LLVM Toolchain path.
    pub path: PathBuf,
    /// The repository containing LLVM sources.
    pub repository_url: String,
    /// LLVM Version ["15"].
    pub version: String,
}

impl Llvm {
    /// Gets the name of the LLVM arch based on the host triple.
    fn get_arch(host_triple: &HostTriple) -> Result<&str> {
        match host_triple {
            HostTriple::Aarch64AppleDarwin => Ok("macos-arm64"),
            HostTriple::X86_64AppleDarwin => Ok("macos"),
            HostTriple::X86_64UnknownLinuxGnu => Ok("linux-amd64"),
            HostTriple::Aarch64UnknownLinuxGnu => Ok("linux-arm64"),
            HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => Ok("win64"),
        }
    }

    /// Gets the binary path.
    fn get_lib_path(&self) -> String {
        #[cfg(windows)]
        let llvm_path = format!("{}/esp-clang/bin", self.path.to_str().unwrap());
        #[cfg(unix)]
        let llvm_path = format!("{}/esp-clang/lib", self.path.to_str().unwrap());
        llvm_path
    }

    /// Create a new instance with default values and proper toolchain version.
    pub fn new(version: String, minified: bool, host_triple: &HostTriple) -> Self {
        let mut file_name = format!(
            "llvm-{}-{}.tar.xz",
            DEFAULT_LLVM_15_VERSION,
            Self::get_arch(host_triple).unwrap()
        );
        if minified {
            file_name = format!("libs_{}", file_name);
        }
        let repository_url = format!(
            "{}/{}/{}",
            DEFAULT_LLVM_REPOSITORY, DEFAULT_LLVM_15_VERSION, file_name,
        );
        let path = PathBuf::from(format!(
            "{}/{}-{}",
            get_tool_path("xtensa-esp32-elf-clang"),
            DEFAULT_LLVM_15_VERSION,
            host_triple
        ));
        Self {
            file_name,
            host_triple: host_triple.clone(),
            path,
            repository_url,
            version,
        }
    }

    /// Uninstall LLVM toolchain.
    pub fn uninstall(llvm_path: &Path) -> Result<(), Error> {
        info!("{} Deleting Xtensa LLVM", emoji::WRENCH);
        remove_dir_all(llvm_path)
            .map_err(|_| Error::FailedToRemoveDirectory(llvm_path.display().to_string()))?;
        Ok(())
    }
}

#[async_trait]
impl Installable for Llvm {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let mut exports: Vec<String> = Vec::new();

        if Path::new(&self.path).exists() {
            warn!(
                "{} Previous installation of LLVM exists in: '{}'. Reusing this installation.",
                emoji::WARN,
                self.path.to_str().unwrap()
            );
        } else {
            info!("{} Installing Xtensa elf Clang", emoji::WRENCH);
            download_file(
                self.repository_url.clone(),
                "idf_tool_xtensa_elf_clang.tar.xz",
                self.path.to_str().unwrap(),
                true,
            )
            .await?;
        }
        // Set environment variables.
        #[cfg(windows)]
        exports.push(format!(
            "$Env:LIBCLANG_PATH = \"{}/libclang.dll\"",
            self.get_lib_path()
        ));
        #[cfg(windows)]
        exports.push(format!("$Env:PATH += \";{}\"", self.get_lib_path()));
        #[cfg(unix)]
        exports.push(format!("export LIBCLANG_PATH=\"{}\"", self.get_lib_path()));

        Ok(exports)
    }
}
