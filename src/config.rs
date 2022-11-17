use crate::{error::Error, host_triple::HostTriple, targets::Target, toolchain::rust::XtensaRust};
use directories_next::ProjectDirs;
use miette::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::{create_dir_all, read, write},
    path::PathBuf,
};

/// Deserialized contents of a configuration file
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct Config {
    /// ESP-IDF version
    pub esp_idf_version: Option<String>,
    /// Destination of the generated export file.
    pub export_file: Option<PathBuf>,
    /// Extra crates to installed.
    pub extra_crates: Option<HashSet<String>>,
    /// Host triple
    pub host_triple: HostTriple,
    /// LLVM toolchain path.
    pub llvm_path: Option<PathBuf>,
    /// Nightly Rust toolchain version.
    pub nightly_version: String,
    /// List of targets instaled.
    pub targets: HashSet<Target>,
    /// Xtensa Rust toolchain.
    pub xtensa_rust: Option<XtensaRust>,
}

impl Config {
    /// Gets the path to the configuration file.
    pub fn get_config_path() -> Result<PathBuf, Error> {
        let dirs = ProjectDirs::from("rs", "esp", "espup").unwrap();
        let file = dirs.config_dir().join("espup.toml");
        Ok(file)
    }

    /// Load the config from config file
    pub fn load() -> Result<Self, Error> {
        let file = Self::get_config_path()?;

        let config = if let Ok(data) = read(&file) {
            toml::from_slice(&data).map_err(|_| Error::FailedToDeserialize)?
        } else {
            return Err(Error::FileNotFound(file.to_string_lossy().into_owned()));
        };
        Ok(config)
    }

    /// Save the config to file
    pub fn save(&self) -> Result<(), Error> {
        let file = Self::get_config_path()?;

        let serialized = toml::to_string(&self.clone()).map_err(|_| Error::FailedToSerialize)?;
        create_dir_all(file.parent().unwrap()).map_err(|_| Error::FailedToCreateConfigFile)?;
        write(&file, serialized).map_err(|_| Error::FailedToWrite(file.display().to_string()))?;
        Ok(())
    }
}
