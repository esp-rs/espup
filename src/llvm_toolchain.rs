//! LLVM Toolchain source and installation tools

use crate::emoji;
use crate::espidf::get_tool_path;
use crate::utils::download_file;
use anyhow::{bail, Result};
use log::info;
use std::path::{Path, PathBuf};

const DEFAULT_LLVM_COMPLETE_REPOSITORY: &str =
    "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_MINIFIED_REPOSITORY: &str =
    "https://github.com/esp-rs/rust-build/releases/download/llvm-project-14.0-minified";

#[derive(Debug)]
pub struct LlvmToolchain {
    /// The repository containing LVVM sources.
    pub repository_url: String,
    /// Repository release version to use.
    pub release: String,
    /// LLVM Version.
    pub version: String,
    /// LLVM Toolchain file name.
    pub file_name: String,
    /// LLVM Toolchain path.
    pub path: PathBuf,
}

impl LlvmToolchain {
    /// Gets the name of the LLVM arch based on the host triple.
    fn get_arch(host_triple: &str) -> Result<String, String> {
        match host_triple {
            "aarch64-apple-darwin" | "x86_64-apple-darwin" => Ok("macos".to_string()),
            "x86_64-unknown-linux-gnu" => Ok("linux-amd64".to_string()),
            "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => Ok("win64".to_string()),
            _ => Err(format!(
                "No LLVM arch found for the host triple: {}",
                host_triple
            )),
        }
    }

    /// Gets the artifact extension based on the host architecture.
    fn get_artifact_extension(host_triple: &str) -> &str {
        match host_triple {
            "x86_64-pc-windows-msvc" | "x86_64-pc-windows-gnu" => "zip",
            _ => "tar.xz",
        }
    }

    /// Gets the binary path.
    pub fn get_lib_path(&self) -> String {
        format!("{}/lib", get_tool_path("xtensa-esp32-elf-clang"))
    }

    /// Gets the full release name.
    fn get_release(version: &str) -> Result<String, String> {
        let parsed_version = match version {
            "13" => "esp-13.0.0-20211203",
            "14" => "esp-14.0.0-20220415",
            // "15" => "", // TODO: Fill when released
            _ => {
                return Err(format!("Unknown LLVM Version: {}", version));
            }
        };

        Ok(parsed_version.to_string())
    }

    /// Gets the parsed release name.
    fn get_release_with_underscores(release: &str) -> String {
        let release: Vec<&str> = release.split('-').collect();
        let llvm_dot_release = release[1];
        llvm_dot_release.replace('.', "_")
    }

    pub fn install(&self) -> Result<()> {
        if Path::new(&self.path).exists() {
            bail!(
            "{} Previous installation of LLVM exist in: {}.\n Please, remove the directory before new installation.",
            emoji::WARN,
            self.path.to_str().unwrap()
        );
        } else {
            info!("{} Installing Xtensa elf Clang", emoji::WRENCH);
            download_file(
                self.repository_url.clone(),
                &format!(
                    "idf_tool_xtensa_elf_clang.{}",
                    Self::get_artifact_extension(guess_host_triple::guess_host_triple().unwrap())
                ),
                &get_tool_path(""),
                true,
            )?;
        }
        Ok(())
    }

    /// Create a new instance with default values and proper toolchain version.
    pub fn new(version: &str, minified: bool) -> Self {
        let host_triple = guess_host_triple::guess_host_triple().unwrap();
        let release = Self::get_release(version).unwrap();
        let version = version.to_string();
        let file: String;
        let repository_url: String;
        if minified {
            file = format!(
                "xtensa-esp32-elf-llvm{}-{}-{}.{}",
                Self::get_release_with_underscores(&release),
                &release,
                host_triple,
                Self::get_artifact_extension(host_triple)
            );
            repository_url = format!("{}/{}", DEFAULT_LLVM_MINIFIED_REPOSITORY, file,);
        } else {
            file = format!(
                "xtensa-esp32-elf-llvm{}-{}-{}.{}",
                Self::get_release_with_underscores(&release),
                &release,
                Self::get_arch(host_triple).unwrap(),
                Self::get_artifact_extension(host_triple)
            );
            repository_url = format!("{}/{}/{}", DEFAULT_LLVM_COMPLETE_REPOSITORY, &release, file);
        }
        let path = format!(
            "{}/{}-{}",
            get_tool_path("xtensa-esp32-elf-clang"),
            release,
            host_triple
        )
        .into();
        Self {
            repository_url,
            release,
            version,
            file_name: file,
            path,
        }
    }
}
