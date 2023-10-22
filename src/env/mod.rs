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
pub fn set_env(toolchain_dir: &Path, no_modify_env: bool) -> Result<(), Error> {
    #[cfg(windows)]
    windows::write_env_files(toolchain_dir)?;
    #[cfg(unix)]
    unix::write_env_files(toolchain_dir)?;

    if !no_modify_env {
        #[cfg(windows)]
        windows::update_env()?;
        #[cfg(unix)]
        unix::update_env(toolchain_dir)?;
    }

    Ok(())
}

pub fn get_home_dir() -> PathBuf {
    BaseDirs::new().unwrap().home_dir().to_path_buf()
}

pub fn clean_env(install_dir: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    windows::clean_env(install_dir)?;
    #[cfg(unix)]
    unix::clean_env(install_dir)?;

    Ok(())
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
        "\t'. {}\\env.ps1' or '{}\\env.bat' depending on your shell'",
        toolchain_dir, toolchain_dir
    );
}
