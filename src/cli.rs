//! Command line interface.

use crate::completion_shell::CompletionShell;
use crate::targets::{Target, parse_targets};
use clap::Parser;
use std::{collections::HashSet, path::PathBuf};

#[derive(Debug, Parser)]
pub struct CompletionsOpts {
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Shell to generate completions for.
    pub shell: CompletionShell,
}

#[derive(Debug, Parser)]
pub struct InstallOpts {
    /// Target triple of the host.
    #[arg(short = 'd', long, value_parser = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu" , "x86_64-apple-darwin" , "aarch64-apple-darwin"])]
    pub default_host: Option<String>,
    /// Install Espressif RISC-V toolchain built with croostool-ng
    ///
    /// Only install this if you don't want to use the systems RISC-V toolchain
    #[arg(short = 'r', long)]
    pub esp_riscv_gcc: bool,
    /// Relative or full path for the export file that will be generated. If no path is provided, the file will be generated under home directory (https://docs.rs/dirs/latest/dirs/fn.home_dir.html).
    #[arg(short = 'f', long, env = "ESPUP_EXPORT_FILE")]
    pub export_file: Option<PathBuf>,
    /// Disables HTTP timeouts for installation downloads and GitHub queries.
    #[arg(long, env = "ESPUP_DISABLE_TIMEOUTS")]
    pub disable_timeouts: bool,
    /// Extends the LLVM installation.
    ///
    /// This will install the whole LLVM instead of only installing the libs.
    #[arg(short = 'e', long)]
    pub extended_llvm: bool,
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain name.
    #[arg(short = 'a', long, default_value = "esp")]
    pub name: String,
    /// Stable Rust toolchain version.
    ///
    /// Note that only RISC-V targets use stable Rust channel.
    #[arg(short = 'b', long, default_value = "stable")]
    pub stable_version: String,
    /// Skips parsing Xtensa Rust version.
    #[arg(short = 'k', long, requires = "toolchain_version")]
    pub skip_version_parse: bool,
    /// Only install toolchains required for STD applications.
    ///
    /// With this option, espup will skip GCC installation (it will be handled by esp-idf-sys), hence you won't be able to build no_std applications.
    #[arg(short = 's', long)]
    pub std: bool,
    /// Comma or space separated list of targets [esp32,esp32c2,esp32c3,esp32c5,esp32c6,esp32c61,esp32h2,esp32s2,esp32s3,esp32p4,all].
    #[arg(short = 't', long, default_value = "all", value_parser = parse_targets)]
    pub targets: HashSet<Target>,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long)]
    pub toolchain_version: Option<String>,
    /// Crosstool-NG toolchain version, e.g. (14.2.0_20241119)
    #[arg(short = 'c', long)]
    pub crosstool_toolchain_version: Option<String>,
}

#[derive(Debug, Parser)]
pub struct UninstallOpts {
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain name.
    #[arg(short = 'a', long, default_value = "esp")]
    pub name: String,
    /// GCC toolchain version.
    #[arg(short = 'c', long)]
    pub crosstool_toolchain_version: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::InstallOpts;
    use clap::Parser;

    #[test]
    fn install_accepts_disable_timeouts_flag() {
        let opts = InstallOpts::try_parse_from(["espup", "--disable-timeouts"]).unwrap();

        assert!(opts.disable_timeouts);
    }

    #[test]
    fn install_accepts_disable_timeouts_env_var() {
        unsafe {
            std::env::set_var("ESPUP_DISABLE_TIMEOUTS", "true");
        }
        let opts = InstallOpts::try_parse_from(["espup"]).unwrap();
        unsafe {
            std::env::remove_var("ESPUP_DISABLE_TIMEOUTS");
        }

        assert!(opts.disable_timeouts);
    }
}
