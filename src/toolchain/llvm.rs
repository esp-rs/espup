//! LLVM Toolchain source and installation tools

use super::Installable;
#[cfg(windows)]
use crate::host_triple::get_host_triple;
use crate::{
    emoji,
    error::Error,
    host_triple::HostTriple,
    toolchain::{download_file, espidf::get_tool_path},
};
use async_trait::async_trait;
#[cfg(windows)]
use embuild::cmd;
use log::{info, warn};
use miette::Result;
#[cfg(windows)]
use std::process::Stdio;
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
    /// If `true`, only libraries are installed.
    minified: bool,
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
    pub fn new(version: String, minified: bool, host_triple: &HostTriple) -> Self {
        let mut file_name = format!(
            "llvm-{}-{}.tar.xz",
            DEFAULT_LLVM_15_VERSION,
            Self::get_arch(host_triple).unwrap()
        );
        if minified {
            file_name = format!("libs_{file_name}");
        }
        let repository_url =
            format!("{DEFAULT_LLVM_REPOSITORY}/{DEFAULT_LLVM_15_VERSION}/{file_name}");
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
            minified,
        }
    }

    /// Uninstall LLVM toolchain.
    pub fn uninstall(llvm_path: &Path) -> Result<(), Error> {
        info!("{} Deleting Xtensa LLVM", emoji::WRENCH);
        remove_dir_all(llvm_path)
            .map_err(|_| Error::FailedToRemoveDirectory(llvm_path.display().to_string()))?;
        #[cfg(windows)]
        if cfg!(windows) {
            let host_triple = get_host_triple(None)?;
            cmd!("setx", "LIBCLANG_PATH", "", "/m")
                .into_inner()
                .stdout(Stdio::null())
                .output()?;
            #[cfg(windows)]
            std::env::set_var(
                "PATH",
                std::env::var("PATH").unwrap().replace(
                    &format!(
                        "{}\\{}-{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        DEFAULT_LLVM_15_VERSION,
                        host_triple
                    ),
                    "",
                ),
            );
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
            )
            .await?;
        }

        #[cfg(windows)]
        if cfg!(windows) {
            exports.push(format!(
                "$Env:LIBCLANG_PATH = \"{}/libclang.dll\"",
                self.get_lib_path()
            ));
            exports.push(format!("$Env:PATH += \";{}\"", self.get_lib_path()));
            cmd!(
                "setx",
                "LIBCLANG_PATH",
                format!("{}\\libclang.dll", self.get_lib_path().replace('/', "\\")),
                "/m"
            )
            .into_inner()
            .stdout(Stdio::null())
            .output()?;
            std::env::set_var(
                "PATH",
                std::env::var("PATH").unwrap() + ";" + &self.get_lib_path().replace('/', "\\"),
            );
        }
        #[cfg(unix)]
        exports.push(format!("export LIBCLANG_PATH=\"{}\"", self.get_lib_path()));

        if !self.minified {
            #[cfg(windows)]
            exports.push(format!("$Env:CLANG_PATH = \"{}\"", self.get_bin_path()));
            #[cfg(unix)]
            exports.push(format!("export CLANG_PATH=\"{}\"", self.get_bin_path()));
        }

        Ok(exports)
    }

    fn name(&self) -> String {
        "LLVM".to_string()
    }
}
