use crate::emoji;

#[derive(Debug, PartialEq, Eq, miette::Diagnostic, thiserror::Error)]
pub enum Error {
    // Host Triple
    #[diagnostic(code(espup::host_triple::unsupported_host_triple))]
    #[error("{} Host triple '{0}' is not supported", emoji::ERROR)]
    UnsupportedHostTriple(String),
    // Target
    #[diagnostic(code(espup::targets::unsupported_target))]
    #[error("{} Target '{0}' is not supported", emoji::ERROR)]
    UnsupportedTarget(String),
    //  Config
    #[diagnostic(code(espup::config::file_not_found))]
    #[error("{} No config file found in '{0}'", emoji::ERROR)]
    FileNotFound(String),
    #[diagnostic(code(espup::config::failed_to_deserialize))]
    #[error("{} Failed to deserialize config", emoji::ERROR)]
    FailedToDeserialize(String),
    #[diagnostic(code(espup::config::failed_to_serialize))]
    #[error("{} Failed to serialize config", emoji::ERROR)]
    FailedToSerialize(String),
    #[diagnostic(code(espup::config::failed_to_create_config_file))]
    #[error("{} Failed to create config directory", emoji::ERROR)]
    FailedToCreateConfigFile(String),
    #[diagnostic(code(espup::config::failed_to_write))]
    #[error("{} Failed to write config to '{0}'", emoji::ERROR)]
    FailedToWrite(String),
}
