//! Custom error implementations.

use std::path::PathBuf;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum Error {
    #[diagnostic(code(espup::toolchain::create_directory))]
    #[error("Creating directory '{0}' failed")]
    CreateDirectory(String),

    #[diagnostic(code(espup::toolchain::rust::query_github))]
    #[error("Failed to query GitHub API")]
    GithubQuery,

    #[diagnostic(code(espup::toolchain::rust::install_riscv_target))]
    #[error("Failed to Install RISC-V targets for '{0}' toolchain")]
    InstallRiscvTarget(String),

    #[diagnostic(code(espup::ivalid_destination))]
    #[error(
        "Invalid export file destination: '{0}'. Please, use an absolute or releative path (including the file and its extension)")]
    InvalidDestination(String),

    #[diagnostic(code(espup::toolchain::rust::invalid_version))]
    #[error(
        "Invalid toolchain version '{0}'. Verify that the format is correct: '<major>.<minor>.<patch>.<subpatch>' or '<major>.<minor>.<patch>', and that the release exists in https://github.com/esp-rs/rust-build/releases")]
    InvalidVersion(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[diagnostic(code(espup::toolchain::rust::missing_rust))]
    #[error("Rust is not installed. Please, install Rust via rustup: https://rustup.rs/")]
    MissingRust,

    #[diagnostic(code(espup::remove_directory))]
    #[error("Failed to remove '{0}'")]
    RemoveDirectory(String),

    #[error(transparent)]
    RewquestError(#[from] reqwest::Error),

    #[diagnostic(code(espup::toolchain::rust::rustup_detection_error))]
    #[error("Error detecting rustup: {0}")]
    RustupDetection(String),

    #[diagnostic(code(espup::toolchain::rust::serialize_json))]
    #[error("Failed to serialize json from string")]
    SerializeJson,

    #[diagnostic(code(espup::toolchain::rust::uninstall_riscv_target))]
    #[error("Failed to uninstall RISC-V target")]
    UninstallRiscvTarget,

    #[diagnostic(code(espup::toolchain::unsupported_file_extension))]
    #[error("Unsuported file extension: '{0}'")]
    UnsuportedFileExtension(String),

    #[diagnostic(code(espup::host_triple::unsupported_host_triple))]
    #[error("Host triple '{0}' is not supported")]
    UnsupportedHostTriple(String),

    #[diagnostic(code(espup::targets::unsupported_target))]
    #[error("Target '{0}' is not supported")]
    UnsupportedTarget(String),

    #[diagnostic(code(espup::toolchain::rust::rust))]
    #[error("Failed to install 'rust' component of Xtensa Rust")]
    XtensaRust,

    #[diagnostic(code(espup::toolchain::rust::rust_src))]
    #[error("Failed to install 'rust-src' component of Xtensa Rust")]
    XtensaRustSrc,

    #[diagnostic(code(espup::env::unix))]
    #[error("Failed to read {name} file: '{}'", .path.display())]
    ReadingFile { name: &'static str, path: PathBuf },

    #[diagnostic(code(espup::env::shell))]
    #[error("ZDOTDIR not set")]
    Zdotdir,
}
