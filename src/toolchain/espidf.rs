//! GCC Toolchain source and installation tools
use super::Installable;
use crate::{
    emoji,
    error::Error,
    targets::Target,
    toolchain::gcc::{get_toolchain_name, get_ulp_toolchain_name},
};
use async_trait::async_trait;
use directories::BaseDirs;
use embuild::{espidf, espidf::EspIdfRemote, git};
use log::{debug, info};
use miette::Result;
use std::{
    collections::hash_map::DefaultHasher,
    collections::HashSet,
    env,
    fs::remove_dir_all,
    hash::{Hash, Hasher},
    path::PathBuf,
};
use strum::{Display, EnumIter, EnumString, IntoStaticStr};

pub const DEFAULT_GIT_REPOSITORY: &str = "https://github.com/espressif/esp-idf";

const DEFAULT_CMAKE_GENERATOR: Generator = {
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
#[derive(Debug, Clone, Default)]
pub struct EspIdfRepo {
    /// The repository containing GCC sources.
    pub repository_url: String,
    /// ESP-IDF Version.
    pub version: String,
    /// Minify ESP-IDF?.
    pub minified: bool,
    /// Installation directory.
    pub install_path: PathBuf,
    /// ESP targets.
    pub targets: HashSet<Target>,
}

impl EspIdfRepo {
    /// Create a new instance with the proper arguments.
    pub fn new(version: &str, minified: bool, targets: &HashSet<Target>) -> EspIdfRepo {
        let install_path = PathBuf::from(get_tools_path());
        debug!(
            "{} ESP-IDF install path: '{}'",
            emoji::DEBUG,
            install_path.display()
        );

        Self {
            repository_url: DEFAULT_GIT_REPOSITORY.to_string(),
            version: version.to_string(),
            minified,
            install_path,
            targets: targets.clone(),
        }
    }

    /// Uninstall ESP-IDF.
    pub fn uninstall(version: &str) -> Result<(), Error> {
        info!("{} Deleting ESP-IDF {}", emoji::WRENCH, version);
        let repo = EspIdfRemote {
            git_ref: espidf::parse_esp_idf_git_ref(version),
            repo_url: Some(DEFAULT_GIT_REPOSITORY.to_string()),
        };
        remove_dir_all(get_install_path(repo.clone()).parent().unwrap()).map_err(|_| {
            Error::FailedToRemoveDirectory(
                get_install_path(repo)
                    .parent()
                    .unwrap()
                    .display()
                    .to_string(),
            )
        })?;
        Ok(())
    }
}

#[async_trait]
impl Installable for EspIdfRepo {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let cmake_generator = DEFAULT_CMAKE_GENERATOR;
        let mut exports: Vec<String> = Vec::new();
        let targets = self.targets.clone();
        // A closure to specify which tools `idf-tools.py` should install.
        let make_tools = move |repo: &git::Repository,
                               version: &anyhow::Result<espidf::EspIdfVersion>|
              -> anyhow::Result<Vec<espidf::Tools>> {
            let version_str = match version {
                Ok(v) => format!("v{v}"),
                Err(_) => "(unknown version)".to_string(),
            };
            info!(
                "{} Using esp-idf {} at '{}'",
                emoji::INFO,
                version_str,
                repo.worktree().display()
            );

            let mut tools = vec![];
            let mut subtools = Vec::new();
            for target in targets {
                let gcc_toolchain_name = get_toolchain_name(&target);
                subtools.push(gcc_toolchain_name);

                let ulp_toolchain_name = get_ulp_toolchain_name(target, version.as_ref().ok());
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
                    tools
                        .push(espidf::Tools::cmake().map_err(|_| Error::FailedToInstantiateCmake)?);
                }
            }
            #[cfg(windows)]
            subtools.push("openocd-esp32".to_string());
            #[cfg(windows)]
            subtools.push("idf-exe".to_string());
            #[cfg(windows)]
            subtools.push("ccache".to_string());
            #[cfg(windows)]
            subtools.push("dfu-util".to_string());

            if cmake_generator == Generator::Ninja {
                subtools.push("ninja".to_string())
            }

            tools.push(espidf::Tools::new(subtools));

            Ok(tools)
        };

        let install = |esp_idf_origin: espidf::EspIdfOrigin| -> Result<espidf::EspIdf, Error> {
            espidf::Installer::new(esp_idf_origin)
                .install_dir(Some(self.install_path.clone()))
                .with_tools(make_tools)
                .install()
                .map_err(|_| Error::FailedToCreateEspIdfInstallClosure)
        };

        let repo = espidf::EspIdfRemote {
            git_ref: espidf::parse_esp_idf_git_ref(&self.version),
            repo_url: Some("https://github.com/espressif/esp-idf".to_string()),
        };

        let espidf_origin = espidf::EspIdfOrigin::Managed(repo.clone());
        #[cfg(unix)]
        let espidf = install(espidf_origin).map_err(|_| Error::FailedToInstallEspIdf)?;
        #[cfg(windows)]
        install(espidf_origin).map_err(|_| Error::FailedToInstallEspIdf)?;
        let espidf_dir = get_install_path(repo);
        #[cfg(windows)]
        exports.push(format!("$Env:IDF_PATH=\"{}\"", espidf_dir.display()));
        #[cfg(unix)]
        exports.push(format!("export IDF_PATH={}", espidf_dir.display()));
        #[cfg(windows)]
        exports.push(espidf_dir.join("export.ps1").display().to_string());
        #[cfg(unix)]
        exports.push(format!("export PATH={:?}", espidf.exported_path));
        if self.minified {
            info!("{} Minifying ESP-IDF", emoji::INFO);
            remove_dir_all(espidf_dir.join("docs"))?;
            remove_dir_all(espidf_dir.join("examples"))?;
            remove_dir_all(espidf_dir.join("tools").join("esp_app_trace"))?;
            remove_dir_all(espidf_dir.join("tools").join("test_idf_size"))?;
        }

        #[cfg(windows)]
        exports.push(format!("$Env:IDF_TOOLS_PATH=\"{}\"", get_tools_path()));
        #[cfg(unix)]
        exports.push(format!("export IDF_TOOLS_PATH=\"{}\"", get_tools_path()));

        Ok(exports)
    }
}

/// Gets the esp-idf installation path.
pub fn get_install_path(repo: EspIdfRemote) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    repo.repo_url.as_ref().unwrap().hash(&mut hasher);
    let repo_url_hash = format!("{:x}", hasher.finish());
    let repo_dir = match repo.git_ref {
        git::Ref::Branch(n) | git::Ref::Tag(n) | git::Ref::Commit(n) => n,
    };
    // Replace all directory separators with a dash `-`, so that we don't create
    // subfolders for tag or branch names that contain such characters.
    let repo_dir = repo_dir.replace(['/', '\\'], "-");

    let mut install_path = PathBuf::from(get_tools_path());
    install_path = install_path.join(PathBuf::from(format!("esp-idf-{}", repo_url_hash)));
    install_path = install_path.join(PathBuf::from(repo_dir));
    install_path
}

/// Gets path where esp-idf tools where be downloaded and installed. If environment
/// variable IDF_TOOLS_PATH is not set. Uses HOME/.espressif on Linux and macOS,
/// and %USER_PROFILE%\.espressif on Windows.
pub fn get_tools_path() -> String {
    env::var("IDF_TOOLS_PATH").unwrap_or_else(|_e| {
        format!(
            "{}",
            BaseDirs::new()
                .unwrap()
                .home_dir()
                .join(".espressif")
                .display()
        )
    })
}

/// Gets the espressif tools directory path. Tools directory is where the tools
/// are extracted.
pub fn get_tool_path(tool_name: &str) -> String {
    format!("{}/tools/{}", get_tools_path(), tool_name)
}

/// Gets the Espressif dist directory path. Dist directory is where the archives
/// of the tools are downloaded.
pub fn get_dist_path(tool_name: &str) -> String {
    let tools_path = get_tools_path();
    format!("{}/dist/{}", tools_path, tool_name)
}
