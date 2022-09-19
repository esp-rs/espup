use crate::chip::Chip;
use crate::emoji;
use crate::gcc_toolchain::GccToolchain;
use crate::utils::*;
use anyhow::{bail, Result};
use embuild::cmd;
use log::{info, warn};
use std::process::Stdio;

pub fn check_rust_installation(nightly_version: &str) -> Result<()> {
    match std::process::Command::new("rustup")
        .arg("toolchain")
        .arg("list")
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            let result = String::from_utf8_lossy(&child_output.stdout);
            if !result.contains("nightly") {
                warn!("{} Rust nightly toolchain not found", emoji::WARN);
                install_rust_nightly(nightly_version)?;
            }
        }
        Err(e) => {
            if let std::io::ErrorKind::NotFound = e.kind() {
                warn!("{} rustup was not found.", emoji::WARN);
                install_rustup(nightly_version)?;
            } else {
                bail!("{} Error: {}", emoji::ERROR, e);
            }
        }
    }
    Ok(())
}

pub fn install_rustup(nightly_version: &str) -> Result<()> {
    #[cfg(windows)]
    let rustup_init_path = download_file(
        "https://win.rustup.rs/x86_64".to_string(),
        "rustup-init.exe",
        &get_dist_path("rustup"),
        false,
    )?;
    #[cfg(unix)]
    let rustup_init_path = download_file(
        "https://sh.rustup.rs".to_string(),
        "rustup-init.sh",
        &get_dist_path("rustup"),
        false,
    )?;
    info!(
        "{} Installing rustup with {} toolchain",
        emoji::WRENCH,
        nightly_version
    );

    #[cfg(windows)]
    // TO BE TESTED
    cmd!(
        rustup_init_path,
        "--default-toolchain",
        nightly_version,
        "--profile",
        "minimal",
        "-y"
    )
    .run()?;
    #[cfg(not(windows))]
    cmd!(
        "/bin/bash",
        rustup_init_path,
        "--default-toolchain",
        nightly_version,
        "--profile",
        "minimal",
        "-y"
    )
    .run()?;
    Ok(())
}

pub fn install_rust_nightly(version: &str) -> Result<()> {
    info!("{} Installing {} toolchain", emoji::WRENCH, version);
    cmd!(
        "rustup",
        "toolchain",
        "install",
        version,
        "--profile",
        "minimal"
    )
    .run()?;
    Ok(())
}

pub fn install_gcc_targets(targets: Vec<Chip>) -> Result<Vec<String>> {
    let mut exports: Vec<String> = Vec::new();
    for target in targets {
        let gcc = GccToolchain::new(target);
        gcc.install()?;
        exports.push(format!("export PATH={}:$PATH", gcc.get_bin_path()));
    }
    Ok(exports)
}
