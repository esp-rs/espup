//! GCC Toolchain source and installation tools

use crate::chip::Chip;
use crate::emoji;
use crate::gcc_toolchain::GccToolchain;
use crate::utils::get_tools_path;
use anyhow::{Context, Result};
use embuild::{espidf, git};
use log::debug;
use std::path::PathBuf;
use strum::{Display, EnumIter, EnumString, IntoStaticStr};

const DEFAULT_GIT_REPOSITORY: &str = "https://github.com/espressif/esp-idf";

pub const DEFAULT_CMAKE_GENERATOR: Generator = {
    // No Ninja builds for linux=aarch64 from Espressif yet
    #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
    {
        Generator::UnixMakefiles
    }

    #[cfg(not(all(target_os = "linux", target_arch = "aarch64")))]
    {
        Generator::Ninja
    }
};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, EnumString, Display, EnumIter, IntoStaticStr)]
pub enum Generator {
    Ninja,
    NinjaMultiConfig,
    UnixMakefiles,
    BorlandMakefiles,
    MSYSMakefiles,
    MinGWMakefiles,
    NMakeMakefiles,
    NMakeMakefilesJOM,
    WatcomWMake,
}

pub struct EspIdf {
    /// The repository containing GCC sources.
    pub repository_url: String,
    /// ESP-IDF Version.
    pub version: String,
    /// Minify ESP-IDF?.
    pub minified: bool,
    /// Installation directory.
    pub install_path: PathBuf,
    /// ESP targets.
    pub targets: Vec<Chip>,
}

impl EspIdf {
    pub fn get_install_path(version: &str) -> String {
        let parsed_version: String = version
            .chars()
            .map(|x| match x {
                '/' => '-',
                _ => x,
            })
            .collect();
        format!("{}/frameworks/esp-idf-{}", get_tools_path(), parsed_version)
    }

    pub fn new(version: &str, minified: bool, targets: Vec<Chip>) -> EspIdf {
        let install_path = PathBuf::from(get_tools_path());
        debug!(
            "{} ESP-IDF install path: {}",
            emoji::DEBUG,
            install_path.display()
        );
        Self {
            repository_url: DEFAULT_GIT_REPOSITORY.to_string(),
            version: version.to_string(),
            minified,
            install_path,
            targets,
        }
    }

    pub fn install(self) -> Result<()> {
        let cmake_generator = DEFAULT_CMAKE_GENERATOR;

        // A closure to specify which tools `idf-tools.py` should install.
        let make_tools = move |repo: &git::Repository,
                               version: &Result<espidf::EspIdfVersion>|
              -> Result<Vec<espidf::Tools>> {
            eprintln!(
                "Using esp-idf {} at '{}'",
                espidf::EspIdfVersion::format(version),
                repo.worktree().display()
            );

            let mut tools = vec![];
            let mut subtools = Vec::new();
            for target in self.targets.clone() {
                let gcc_toolchain_name = GccToolchain::get_toolchain_name(target);
                subtools.push(gcc_toolchain_name);

                let ulp_toolchain_name =
                    GccToolchain::get_ulp_toolchain_name(target, version.as_ref().ok());
                if !cfg!(target_os = "linux") || !cfg!(target_arch = "aarch64") {
                    if let Some(ulp_toolchain_name) = ulp_toolchain_name {
                        subtools.push(ulp_toolchain_name);
                    }
                }
            }

            // Use custom cmake for esp-idf<4.4, because we need at least cmake-3.20
            match version.as_ref().map(|v| (v.major, v.minor, v.patch)) {
                Ok((major, minor, _)) if major >= 4 && minor >= 4 => {
                    subtools.push("cmake".to_string())
                }
                _ => {
                    tools.push(espidf::Tools::cmake()?);
                }
            }

            subtools.push("openocd-esp32".to_string());

            if cmake_generator == Generator::Ninja {
                subtools.push("ninja".to_string())
            }

            tools.push(espidf::Tools::new(subtools));

            Ok(tools)
        };
        let install = |esp_idf_origin: espidf::EspIdfOrigin| -> Result<espidf::EspIdf> {
            espidf::Installer::new(esp_idf_origin)
                .install_dir(Some(self.install_path))
                .with_tools(make_tools)
                .install()
                .context("Could not install esp-idf")
        };

        install(espidf::EspIdfOrigin::Managed(espidf::EspIdfRemote {
            git_ref: espidf::parse_esp_idf_git_ref(&self.version),
            repo_url: Some("https://github.com/espressif/esp-idf".to_string()),
        }))?;
        Ok(())
    }
}
