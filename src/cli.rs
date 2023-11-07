//! Command line interface.

use crate::targets::{parse_targets, Target};
use clap::Parser;
use clap_complete::Shell;
use std::{collections::HashSet, path::PathBuf};

#[derive(Debug, Parser)]
pub struct CompletionsOpts {
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Shell to generate completions for.
    pub shell: Shell,
}

#[derive(Debug, Parser)]
pub struct InstallOpts {
    /// Target triple of the host.
    #[arg(short = 'd', long, value_parser = ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu", "x86_64-pc-windows-msvc", "x86_64-pc-windows-gnu" , "x86_64-apple-darwin" , "aarch64-apple-darwin"])]
    pub default_host: Option<String>,
    /// Relative or full path for the export file that will be generated. If no path is provided, the file will be generated under home directory (https://docs.rs/dirs/latest/dirs/fn.home_dir.html).
    #[arg(short = 'f', long)]
    pub export_file: Option<PathBuf>,
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
    /// Nightly Rust toolchain version.
    #[arg(short = 'n', long, default_value = "nightly")]
    pub nightly_version: String,
    /// Skips parsing Xtensa Rust version.
    #[arg(short = 'k', long)]
    pub skip_version_parse: bool,
    /// Only install toolchains required for STD applications.
    ///
    /// With this option, espup will skip GCC installation (it will be handled by esp-idf-sys), hence you won't be able to build no_std applications.
    #[arg(short = 's', long)]
    pub std: bool,
    /// Install esp's own risc-v toolchain build with croostool-ng
    ///
    /// Only install this if you don't want to use the systems risc-v toolchain
    #[arg(short = 'u', long)]
    pub ulp: bool,
    /// Comma or space separated list of targets [esp32,esp32c2,esp32c3,esp32c6,esp32h2,esp32s2,esp32s3,all].
    #[arg(short = 't', long, default_value = "all", value_parser = parse_targets)]
    pub targets: HashSet<Target>,
    /// Xtensa Rust toolchain version.
    #[arg(short = 'v', long)]
    pub toolchain_version: Option<String>,
}

#[derive(Debug, Parser)]
pub struct UninstallOpts {
    /// Verbosity level of the logs.
    #[arg(short = 'l', long, default_value = "info", value_parser = ["debug", "info", "warn", "error"])]
    pub log_level: String,
    /// Xtensa Rust toolchain name.
    #[arg(short = 'a', long, default_value = "esp")]
    pub name: String,
}
