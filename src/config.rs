use crate::{
    host_triple::HostTriple,
    targets::Target,
    toolchain::espidf::EspIdfRepo,
    toolchain::llvm_toolchain::LlvmToolchain,
    toolchain::rust_toolchain::{RustCrate, RustToolchain},
};
use directories_next::ProjectDirs;
use miette::{IntoDiagnostic, Result, WrapErr};
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
    pub espidf_version: Option<String>,
    // /// ESP-IDF
    // pub espidf: EspIdfRepo,
    /// Destination of the generated export file.
    pub export_file: PathBuf,
    /// Extra crates to installed.
    // pub extra_crates: HashSet<RustCrate>,
    /// GCC toolchain.
    // pub gcc_toolchain: GccToolchain,
    /// Host triple
    pub host_triple: HostTriple,
    /// LLVM toolchain.
    // pub llvm_toolchain: LlvmToolchain,
    /// Nightly Rust toolchain version.
    pub nightly_version: String,
    ///  Minifies the installation.
    pub profile_minimal: bool,
    // /// List of targets [esp32,esp32s2,esp32s3,esp32c3,all].
    pub targets: HashSet<Target>,
    /// Xtensa Rust toolchain.
    pub xtensa_toolchain: RustToolchain,
}

impl Config {
    /// Load the config from config file
    pub fn load() -> Result<Self> {
        let dirs = ProjectDirs::from("rs", "esp", "espup").unwrap();
        let file = dirs.config_dir().join("espup.toml");

        let mut config = if let Ok(data) = read(&file) {
            toml::from_slice(&data).into_diagnostic()?
        } else {
            Self::default()
        };
        // config.save_path = file;
        Ok(config)
    }

    pub fn save_with(&self) -> Result<()> {
        // pub fn save_with<F: Fn(&mut Self)>(&self, modify_fn: F) -> Result<()> {
        // let mut copy = self.clone();
        // modify_fn(&mut copy);
        let dirs = ProjectDirs::from("rs", "esp", "espup").unwrap();
        let file = dirs.config_dir().join("espup.toml");

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
