//! Environment variables set up and environment file support.

use crate::error::Error;
use directories::BaseDirs;
use miette::Result;
use std::path::{Path, PathBuf};

pub mod shell;
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

/// Instructions to export the environment variables.
pub fn set_environment(toolchain_dir: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    if cfg!(windows) {
        windows::write_env_files(toolchain_dir)?;
        windows::update_env(toolchain_dir)?;
    }
    #[cfg(unix)]
    if cfg!(unix) {
        unix::write_env_files(toolchain_dir)?;
        unix::update_env(toolchain_dir)?;
    }
    Ok(())
}

pub fn get_home_dir() -> PathBuf {
    BaseDirs::new().unwrap().home_dir().to_path_buf()
}

pub fn print_post_install_msg(toolchain_dir: &str, no_modify_env: bool) {
    if no_modify_env {
        println!(
            "\tTo get started you need to configure some environment variable. This has not been done automatically."
        );
    } else {
        println!("\tTo get started you may need to restart your current shell.");
    }
    println!("\tTo configure your current shell, run:");
    #[cfg(unix)]
    println!(
        "\t'. {}/env' or '. {}/env.fish' depending on your shell",
        toolchain_dir, toolchain_dir
    );
    #[cfg(windows)]
    println!(
        "\t'. {}\\env.ps1' or '{}\\env.bat dependeing on your shell'",
        toolchain_dir
    );
}
