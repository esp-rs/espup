use crate::emoji;

#[derive(Debug, PartialEq, Eq, miette::Diagnostic, thiserror::Error)]
pub enum Error {
    #[diagnostic(code(espup::host_triple::unsupported_host_triple))]
    #[error("{} Host triple '{0}' is not supported", emoji::ERROR)]
    UnsupportedHostTriple(String),

    #[diagnostic(code(espup::targets::unsupported_target))]
    #[error("{} Target '{0}' is not supported", emoji::ERROR)]
    UnsupportedTarget(String),
}
