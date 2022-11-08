use crate::{host_triple::HostTriple, targets::Target, toolchain::rust::XtensaRust};
use directories_next::ProjectDirs;
use miette::{ErrReport, IntoDiagnostic, Result, WrapErr};
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
    pub export_file: PathBuf,
    /// Extra crates to installed.
    pub extra_crates: Option<HashSet<String>>,
    /// Host triple
    pub host_triple: HostTriple,
    /// LLVM toolchain path.
    pub llvm_path: PathBuf,
    /// Nightly Rust toolchain version.
    pub nightly_version: String,
    /// List of targets instaled.
    pub targets: HashSet<Target>,
    /// Xtensa Rust toolchain.
    pub xtensa_rust: Option<XtensaRust>,
}

impl Config {
    /// Gets the path to the configuration file.
    pub fn get_config_path() -> Result<PathBuf> {
        let dirs = ProjectDirs::from("rs", "esp", "espup").unwrap();
        let file = dirs.config_dir().join("espup.toml");
        Ok(file)
    }

    /// Load the config from config file
    pub fn load() -> Result<Self> {
        let file = Self::get_config_path()?;

        let config = if let Ok(data) = read(file) {
            toml::from_slice(&data).into_diagnostic()?
        } else {
            return Err(ErrReport::msg("No config file found"));
        };
        Ok(config)
    }

    /// Save the config to file
    pub fn save(&self) -> Result<()> {
        let file = Self::get_config_path()?;

        let serialized = toml::to_string(&self.clone())
            .into_diagnostic()
            .wrap_err("Failed to serialize config")?;
        create_dir_all(file.parent().unwrap())
            .into_diagnostic()
            .wrap_err("Failed to create config directory")?;
        write(&file, serialized)
            .into_diagnostic()
            .wrap_err_with(|| format!("Failed to write config to {}", file.display()))
    }
}
