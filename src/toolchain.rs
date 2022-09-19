use crate::chip::Chip;
use crate::emoji;
use crate::gcc_toolchain::GccToolchain;
use crate::utils::*;
use anyhow::{bail, Result};
use embuild::cmd;
use log::{debug, info, warn};
use std::path::Path;
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

// pub fn install_espidf(targets: &str, version: &str) -> Result<()> {
//     let espidf_path = get_espidf_path(version);
//     // debug!("{} ESP-IDF Path: {}", emoji::DEBUG, espidf_path);

//     // #[cfg(windows)]
//     // println!("{} Downloading Git package", emoji::DOWNLOAD);
//     // #[cfg(windows)]
//     // download_file(
//     //     // TODO: Store URL in a constant
//     //     "https://dl.espressif.com/dl/idf-git/idf-git-2.30.1-win64.zip".to_string(),
//     //     "idf-git-2.30.1-win64.zip",
//     //     &get_tool_path("idf-git/2.30.1"),
//     //     true,
//     // )
//     // .unwrap();

//     // #[cfg(windows)]
//     // let git_path = get_tool_path("idf-git/2.30.1/cmd/git.exe");
//     // #[cfg(unix)]
//     // let git_path = "/usr/bin/git".to_string();

//     // #[cfg(windows)]
//     // println!("{} Downloading Python package", emoji::DOWNLOAD);
//     // #[cfg(windows)]
//     // download_file(
//     //     // TODO: Store the URL in RustToolchain
//     //     "https://dl.espressif.com/dl/idf-python/idf-python-3.8.7-embed-win64.zip".to_string(),
//     //     "idf-python-3.8.7-embed-win64.zip",
//     //     &get_tool_path("idf-python/3.8.7"),
//     //     true,
//     // )
//     // .unwrap();

//     #[cfg(windows)]
//     let python_path = get_tool_path("idf-python/3.8.7/python.exe");
//     #[cfg(target_os = "linux")]
//     let python_path = "/usr/bin/python".to_string();
//     #[cfg(target_os = "macos")]
//     let python_path = "/usr/local/bin/python".to_string();
//     if !Path::new(&python_path).exists() {
//         bail!("{} Python not found at {}", emoji::ERROR, python_path);
//     }
//     // #[cfg(target_os = "macos")]
//     // let virtual_env_path = get_python_env_path("4.4", "3.10");
//     // #[cfg(not(target_os = "macos"))]
//     // let virtual_env_path = get_python_env_path("4.4", "3.9");

//     // TODO: See if needed
//     // update_property("gitPath".to_string(), git_path.clone());

//     // TODO: See idf-env to verify installation

//     // TODO: Use any git crate?
//     if !Path::new(&espidf_path).exists() {
//         let mut arguments: Vec<String> = [].to_vec();
//         arguments.push("clone".to_string());
//         arguments.push("--jobs".to_string());
//         arguments.push("8".to_string());
//         arguments.push("--branch".to_string());
//         arguments.push(version.to_string());
//         arguments.push("--depth".to_string());
//         arguments.push("1".to_string());
//         arguments.push("--shallow-submodules".to_string());
//         arguments.push("--recursive".to_string());
//         arguments.push("https://github.com/espressif/esp-idf.git".to_string());
//         arguments.push(espidf_path.clone());
//         // info!("{} Dowloading esp-idf {}", emoji::DOWNLOAD, version);
//         // match run_command(git_path, arguments, "".to_string()) {
//         //     Ok(_) => {
//         //         debug!("{} Cloned esp-idf suscessfuly", emoji::CHECK);
//         //     }
//         //     Err(_e) => {
//         //         bail!("{} Cloned esp-idf failed", emoji::ERROR);
//         //     }
//         // }
//     }
//     info!(
//         "{} Installing esp-idf for {} with {}/install.sh",
//         emoji::WRENCH,
//         targets,
//         espidf_path
//     );
//     let install_script_path = format!("{}/install.sh", espidf_path);
//     let mut arguments: Vec<String> = [].to_vec();
//     arguments.push(targets.to_string());
//     match run_command(&install_script_path, arguments, "".to_string()) {
//         Ok(_) => {
//             debug!("{} ESP-IDF installation succeeded", emoji::CHECK);
//         }
//         Err(_e) => {
//             bail!("{} ESP-IDF installation failed", emoji::ERROR);
//         }
//     }

//     info!("{} Installing CMake", emoji::WRENCH);
//     let mut arguments: Vec<String> = [].to_vec();
//     let idf_tools_scritp_path = format!("{}/tools/idf_tools.py", espidf_path);
//     arguments.push(idf_tools_scritp_path);
//     arguments.push("install".to_string());
//     arguments.push("cmake".to_string());
//     match run_command(&python_path, arguments, "".to_string()) {
//         Ok(_) => {
//             debug!("{} CMake installation succeeded", emoji::CHECK);
//         }
//         Err(_e) => {
//             bail!("{} CMake installation failed", emoji::ERROR);
//         }
//     }

//     Ok(())
// }
