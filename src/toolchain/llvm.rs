//! LLVM Toolchain source and installation tools

use super::Installable;
use crate::{emoji, error::Error, host_triple::HostTriple, toolchain::download_file};
use async_trait::async_trait;
use log::{info, warn};
use miette::Result;
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::process::{Command, Stdio};
use std::{
    fs::remove_dir_all,
    path::{Path, PathBuf},
};

const DEFAULT_LLVM_REPOSITORY: &str = "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_15_VERSION: &str = "esp-15.0.0-20221201";
pub const CLANG_NAME: &str = "xtensa-esp32-elf-clang";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Llvm {
    // /// If `true`, full LLVM, instead of only libraries, are installed.
    extended: bool,
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

    /// Gets the binary path of clang
    fn get_bin_path(&self) -> String {
        #[cfg(windows)]
        let llvm_path = format!("{}/esp-clang/bin/clang.exe", self.path.to_str().unwrap());
        #[cfg(unix)]
        let llvm_path = format!("{}/esp-clang/bin/clang", self.path.to_str().unwrap());
        llvm_path
    }

    /// Create a new instance with default values and proper toolchain version.
    pub fn new(
        toolchain_path: &Path,
        host_triple: &HostTriple,
        extended: bool,
    ) -> Result<Self, Error> {
        let version = DEFAULT_LLVM_15_VERSION.to_string();
        let mut file_name = format!(
            "llvm-{}-{}.tar.xz",
            version,
            Self::get_arch(host_triple).unwrap()
        );
        if !extended {
            file_name = format!("libs_{file_name}");
        }
        let repository_url = format!("{DEFAULT_LLVM_REPOSITORY}/{version}/{file_name}");
        let path = toolchain_path.join(CLANG_NAME).join(&version);

        Ok(Self {
            extended,
            file_name,
            host_triple: host_triple.clone(),
            path,
            repository_url,
            version,
        })
    }

    /// Uninstall LLVM toolchain.
    pub fn uninstall(toolchain_path: &Path) -> Result<(), Error> {
        info!("{} Uninstalling Xtensa LLVM", emoji::WRENCH);
        let llvm_path = toolchain_path.join(CLANG_NAME);
        if llvm_path.exists() {
            #[cfg(windows)]
            if cfg!(windows) {
                Command::new("setx")
                    .args(["LIBCLANG_PATH", "", "/m"])
                    .stdout(Stdio::null())
                    .output()?;
                Command::new("setx")
                    .args(["CLANG_PATH", "", "/m"])
                    .stdout(Stdio::null())
                    .output()?;
                std::env::set_var(
                    "PATH",
                    std::env::var("PATH").unwrap().replace(
                        &format!(
                            "{}\\{}\\esp-clang\\bin;",
                            llvm_path.display().to_string().replace('/', "\\"),
                            DEFAULT_LLVM_15_VERSION,
                        ),
                        "",
                    ),
                );
            }
            remove_dir_all(toolchain_path.join(CLANG_NAME))?;
        }
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
                false,
            )
            .await?;
        }
        // Set environment variables.
        #[cfg(windows)]
        if cfg!(windows) {
            exports.push(format!(
                "$Env:LIBCLANG_PATH = \"{}/libclang.dll\"",
                self.get_lib_path()
            ));
            exports.push(format!("$Env:PATH += \";{}\"", self.get_lib_path()));
            Command::new("setx")
                .args([
                    "LIBCLANG_PATH",
                    &format!("{}\\libclang.dll", self.get_lib_path().replace('/', "\\")),
                    "/m",
                ])
                .stdout(Stdio::null())
                .output()?;
            std::env::set_var(
                "PATH",
                std::env::var("PATH").unwrap() + ";" + &self.get_lib_path().replace('/', "\\"),
            );
        }
        #[cfg(unix)]
        exports.push(format!("export LIBCLANG_PATH=\"{}\"", self.get_lib_path()));

        if self.extended {
            #[cfg(windows)]
            if cfg!(windows) {
                exports.push(format!("$Env:CLANG_PATH = \"{}\"", self.get_bin_path()));
                Command::new("setx")
                    .args(["CLANG_PATH", &self.get_bin_path().replace('/', "\\"), "/m"])
                    .stdout(Stdio::null())
                    .output()?;
            }
            #[cfg(unix)]
            exports.push(format!("export CLANG_PATH=\"{}\"", self.get_bin_path()));
        }

        Ok(exports)
    }

    fn name(&self) -> String {
        "LLVM".to_string()
    }
}
