use crate::error::Error;
use guess_host_triple::guess_host_triple;
use miette::Result;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use strum::Display;
use strum_macros::EnumString;

#[derive(Display, Debug, Clone, EnumString, Deserialize, Serialize, Default)]
pub enum HostTriple {
    /// 64-bit Linux
    #[strum(serialize = "x86_64-unknown-linux-gnu")]
    #[default]
    X86_64UnknownLinuxGnu,
    /// ARM64 Linux
    #[strum(serialize = "aarch64-unknown-linux-gnu")]
    Aarch64UnknownLinuxGnu,
    /// 64-bit MSVC
    #[strum(serialize = "x86_64-pc-windows-msvc")]
    X86_64PcWindowsMsvc,
    /// 64-bit MinGW
    #[strum(serialize = "x86_64-pc-windows-gnu")]
    X86_64PcWindowsGnu,
    /// 64-bit macOS
    #[strum(serialize = "x86_64-apple-darwin")]
    X86_64AppleDarwin,
    /// ARM64 macOS
    #[strum(serialize = "aarch64-apple-darwin")]
    Aarch64AppleDarwin,
}

/// Parse the host triple if specified, otherwise guess it.
pub fn get_host_triple(host_triple_arg: Option<String>) -> Result<HostTriple, Error> {
    let host_triple = if let Some(host_triple) = &host_triple_arg {
        host_triple
    } else {
        guess_host_triple().unwrap()
    };

    HostTriple::from_str(host_triple).map_err(|_| Error::UnsupportedHostTriple(host_triple.into()))
}

#[cfg(test)]
mod tests {
    use crate::host_triple::{get_host_triple, HostTriple};

    #[test]
    fn test_get_host_triple() {
        assert!(matches!(
            get_host_triple(Some("x86_64-unknown-linux-gnu".to_string())),
            Ok(HostTriple::X86_64UnknownLinuxGnu)
        ));
        assert!(matches!(
            get_host_triple(Some("aarch64-unknown-linux-gnu".to_string())),
            Ok(HostTriple::Aarch64UnknownLinuxGnu)
        ));
        assert!(matches!(
            get_host_triple(Some("x86_64-pc-windows-msvc".to_string())),
            Ok(HostTriple::X86_64PcWindowsMsvc)
        ));
        assert!(matches!(
            get_host_triple(Some("x86_64-pc-windows-gnu".to_string())),
            Ok(HostTriple::X86_64PcWindowsGnu)
        ));
        assert!(matches!(
            get_host_triple(Some("x86_64-apple-darwin".to_string())),
            Ok(HostTriple::X86_64AppleDarwin)
        ));
        assert!(matches!(
            get_host_triple(Some("aarch64-apple-darwin".to_string())),
            Ok(HostTriple::Aarch64AppleDarwin)
        ));

        assert!(get_host_triple(Some("some-fake-triple".to_string())).is_err());

        // Guessed Host Triples
        #[cfg(all(target_os = "linux", target_arch = "aarch64"))]
        assert!(matches!(
            get_host_triple(None),
            Ok(HostTriple::Aarch64UnknownLinuxGnu)
        ));
        #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
        assert!(matches!(
            get_host_triple(None),
            Ok(HostTriple::X86_64UnknownLinuxGnu)
        ));
        #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc"))]
        assert!(matches!(
            get_host_triple(None),
            Ok(HostTriple::X86_64PcWindowsMsvc)
        ));
        #[cfg(all(target_os = "windows", target_arch = "x86_64", target_env = "gnu"))]
        assert!(matches!(
            get_host_triple(None),
            Ok(HostTriple::X86_64PcWindowsGnu)
        ));
        #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
        assert!(matches!(
            get_host_triple(None),
            Ok(HostTriple::X86_64AppleDarwin)
        ));
        #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
        assert!(matches!(
            get_host_triple(None),
            Ok(HostTriple::Aarch64AppleDarwin)
        ));
    }
}
