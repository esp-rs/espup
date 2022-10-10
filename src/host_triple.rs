use crate::emoji;
use anyhow::{Context, Result};
use guess_host_triple::guess_host_triple;
use std::str::FromStr;
use strum::Display;
use strum_macros::EnumString;

#[derive(Display, Debug, Clone, EnumString)]
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

/// Parse the host triple if specified, otherwise guess it.
pub fn get_host_triple(host_triple_arg: Option<String>) -> Result<HostTriple> {
    if let Some(host_triple_arg) = host_triple_arg {
        HostTriple::from_str(&host_triple_arg).context(format!(
            "{} Host triple '{}' is not supported.",
            emoji::ERROR,
            host_triple_arg,
        ))
    } else {
        HostTriple::from_str(guess_host_triple().unwrap())
            .context(format!("{} Unable to guess host triple.", emoji::ERROR,))
    }
}
