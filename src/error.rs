use crate::emoji;

#[derive(Debug, miette::Diagnostic, thiserror::Error)]
pub enum Error {
    #[diagnostic(code(espup::toolchain::create_directory))]
    #[error("{} Creating directory '{0}' failed", emoji::ERROR)]
    CreateDirectory(String),

    #[diagnostic(code(espup::toolchain::rust::query_github))]
    #[error("{} Failed to query GitHub API.", emoji::ERROR)]
    GithubQuery,

    #[diagnostic(code(espup::toolchain::rust::install_xtensa_rust))]
    #[error("{} Failed to Install Xtensa Rust toolchain.", emoji::ERROR)]
    InstallXtensaRust,

    #[diagnostic(code(espup::toolchain::rust::install_riscv_target))]
    #[error(
        "{} Failed to Install RISC-V targets for '{0}' toolchain.",
        emoji::ERROR
    )]
    InstallRiscvTarget(String),

    #[diagnostic(code(espup::ivalid_destination))]
    #[error(
        "{} Invalid export file destination: '{0}'. Please, use an absolute or releative path (including the file and its extension).",
        emoji::ERROR
    )]
    InvalidDestination(String),

    #[diagnostic(code(espup::toolchain::rust::invalid_version))]
    #[error(
        "{} Invalid toolchain version '{0}'. Verify that the format is correct: '<major>.<minor>.<patch>.<subpatch>' or '<major>.<minor>.<patch>', and that the release exists in https://github.com/esp-rs/rust-build/releases",
        emoji::ERROR
    )]
    InvalidVersion(String),

    #[error(transparent)]
    IoError(#[from] std::io::Error),

    #[diagnostic(code(espup::toolchain::rust::missing_rust))]
    #[error(
        "{} Rust is not installed. Please, install Rust via rustup: https://rustup.rs/",
        emoji::ERROR
    )]
    MissingRust,

    #[diagnostic(code(espup::remove_directory))]
    #[error("{} Failed to remove '{0}' directory.", emoji::ERROR)]
    RemoveDirectory(String),

    #[error(transparent)]
    RewquestError(#[from] reqwest::Error),

    #[diagnostic(code(espup::toolchain::rust::rustup_detection_error))]
    #[error("{} Error detecting rustup: {0}", emoji::ERROR)]
    RustupDetection(String),

    #[diagnostic(code(espup::toolchain::rust::serialize_json))]
    #[error("{} Failed to serialize json from string.", emoji::ERROR)]
    SerializeJson,

    #[diagnostic(code(espup::toolchain::rust::uninstall_riscv_target))]
    #[error("{} Failed to uninstall RISC-V target.", emoji::ERROR)]
    UninstallRiscvTarget,

    #[diagnostic(code(espup::toolchain::unsupported_file_extension))]
    #[error("{} Unsuported file extension: '{0}'", emoji::ERROR)]
    UnsuportedFileExtension(String),

    #[diagnostic(code(espup::host_triple::unsupported_host_triple))]
    #[error("{} Host triple '{0}' is not supported", emoji::ERROR)]
    UnsupportedHostTriple(String),

    #[diagnostic(code(espup::targets::unsupported_target))]
    #[error("{} Target '{0}' is not supported", emoji::ERROR)]
    UnsupportedTarget(String),
}
