use crate::emoji;
use anyhow::Result;
use guess_host_triple::guess_host_triple;
use std::str::FromStr;
use strum::Display;

#[derive(Display, Debug, Clone)]
pub enum HostTriple {
    /// 64-bit Linux
    #[strum(serialize = "x86_64-unknown-linux-gnu")]
    X86_64UnknownLinuxGnu = 0,
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

impl FromStr for HostTriple {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "x86_64-unknown-linux-gnu" => Ok(HostTriple::X86_64UnknownLinuxGnu),
            "aarch64-unknown-linux-gnu" => Ok(HostTriple::Aarch64UnknownLinuxGnu),
            "x86_64-pc-windows-msvc" => Ok(HostTriple::X86_64PcWindowsMsvc),
            "x86_64-pc-windows-gnu" => Ok(HostTriple::X86_64PcWindowsGnu),
            "x86_64-apple-darwin" => Ok(HostTriple::X86_64AppleDarwin),
            "aarch64-apple-darwin" => Ok(HostTriple::Aarch64AppleDarwin),
            _ => Err(format!(
                "{} Host triple '{}' is not supported.",
                emoji::ERROR,
                s
            )),
        }
    }
}

/// Parse the host triple if specified, otherwise guess it.
pub fn get_host_triple(host_triple_arg: Option<String>) -> Result<HostTriple> {
    let host_triple = match host_triple_arg {
        Some(host_triple) => HostTriple::from_str(&host_triple).unwrap(),
        None => HostTriple::from_str(guess_host_triple().unwrap()).unwrap(),
    };
    Ok(host_triple)
}
