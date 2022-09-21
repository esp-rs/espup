use crate::chip::Chip;
use crate::emoji;
use crate::InstallOpts;
use anyhow::{bail, Result};
use dirs::home_dir;
use flate2::bufread::GzDecoder;
use log::{debug, info};
use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use std::{fs, io};
use tar::Archive;
use xz2::read::XzDecoder;

pub mod logging {
    use env_logger::{Builder, Env, WriteStyle};
    use log::LevelFilter;

    pub fn initialize_logger(filter: LevelFilter) {
        Builder::from_env(Env::default().default_filter_or(filter.as_str()))
            .format_target(false)
            .format_timestamp_secs()
            .write_style(WriteStyle::Always)
            .init();
    }
}

pub fn clear_dist_folder() -> Result<()> {
    info!("{} Clearing dist folder", emoji::WRENCH);
    fs::remove_dir_all(&get_dist_path(""))?;
    Ok(())
}

pub fn parse_targets(build_target: &str) -> Result<Vec<Chip>, String> {
    debug!("{} Parsing targets: {}", emoji::DEBUG, build_target);
    let mut chips: Vec<Chip> = Vec::new();
    if build_target.contains("all") {
        chips.push(Chip::ESP32);
        chips.push(Chip::ESP32S2);
        chips.push(Chip::ESP32S3);
        chips.push(Chip::ESP32C3);
        return Ok(chips);
    }
    let targets: Vec<&str> = if build_target.contains(' ') || build_target.contains(',') {
        build_target.split([',', ' ']).collect()
    } else {
        vec![build_target]
    };
    for target in targets {
        match target {
            "esp32" => chips.push(Chip::ESP32),
            "esp32s2" => chips.push(Chip::ESP32S2),
            "esp32s3" => chips.push(Chip::ESP32S3),
            "esp32c3" => chips.push(Chip::ESP32C3),
            _ => {
                return Err(format!("Unknown target: {}", target));
            }
        };
    }

    Ok(chips)
}

pub fn get_home_dir() -> String {
    home_dir().unwrap().display().to_string()
}

pub fn get_tools_path() -> String {
    env::var("IDF_TOOLS_PATH").unwrap_or_else(|_e| get_home_dir() + "/.espressif")
}

pub fn get_tool_path(tool_name: &str) -> String {
    format!("{}/tools/{}", get_tools_path(), tool_name)
}

pub fn get_dist_path(tool_name: &str) -> String {
    let tools_path = get_tools_path();
    format!("{}/dist/{}", tools_path, tool_name)
}

pub fn download_file(
    url: String,
    file_name: &str,
    output_directory: &str,
    uncompress: bool,
) -> Result<String> {
    let file_path = format!("{}/{}", output_directory, file_name);
    if Path::new(&file_path).exists() {
        info!("{} Using cached file: {}", emoji::INFO, file_path);
        return Ok(file_path);
    } else if !Path::new(&output_directory).exists() {
        info!("{} Creating directory: {}", emoji::WRENCH, output_directory);
        if let Err(_e) = fs::create_dir_all(output_directory) {
            bail!(
                "{} Creating directory {} failed",
                emoji::ERROR,
                output_directory
            );
        }
    }
    info!(
        "{} Downloading file {} from {}",
        emoji::DOWNLOAD,
        file_name,
        url
    );
    let mut resp = reqwest::blocking::get(&url).unwrap();

    if uncompress {
        let extension = Path::new(file_name).extension().unwrap().to_str().unwrap();
        match extension {
            "zip" => {
                let mut tmpfile = tempfile::tempfile().unwrap();
                resp.copy_to(&mut tmpfile)?;
                let mut zipfile = zip::ZipArchive::new(tmpfile).unwrap();
                zipfile.extract(output_directory).unwrap();
            }
            "gz" => {
                info!(
                    "{} Uncompressing tar.gz file to {}",
                    emoji::WRENCH,
                    output_directory
                );
                let content_br = BufReader::new(resp);
                let tarfile = GzDecoder::new(content_br);
                let mut archive = Archive::new(tarfile);
                archive.unpack(output_directory).unwrap();
            }
            "xz" => {
                info!(
                    "{} Uncompressing tar.xz file to {}",
                    emoji::WRENCH,
                    output_directory
                );
                let content_br = BufReader::new(resp);
                let tarfile = XzDecoder::new(content_br);
                let mut archive = Archive::new(tarfile);
                archive.unpack(output_directory).unwrap();
            }
            _ => {
                bail!("{} Unsuported file extension: {}", emoji::ERROR, extension);
            }
        }
    } else {
        info!("{} Creating file: {}", emoji::WRENCH, file_path);
        let mut out = File::create(file_path)?;
        io::copy(&mut resp, &mut out)?;
    }
    Ok(format!("{}/{}", output_directory, file_name))
}

pub fn print_parsed_arguments(args: &InstallOpts, arch: &str, targets: &Vec<Chip>) {
    debug!(
        "{} Arguments:
            - Arch: {}
            - Build targets: {:?}
            - Cargo home: {:?}
            - Clear dist folder: {:?}
            - ESP-IDF version: {:?}
            - Export file: {:?}
            - Extra crates: {:?}
            - LLVM version: {:?}
            - Minified ESP-IDF: {:?}
            - Nightly version: {:?}
            - Rustup home: {:?}
            - Toolchain version: {:?}
            - Toolchain destination: {:?}",
        emoji::INFO,
        arch,
        targets,
        &args.cargo_home,
        args.clear_dist,
        &args.espidf_version,
        &args.export_file,
        args.extra_crates,
        args.llvm_version,
        &args.minified_espidf,
        args.nightly_version,
        &args.rustup_home,
        args.toolchain_version,
        &args.toolchain_destination
    );
}

pub fn export_environment(export_file: &PathBuf, exports: &[String]) -> Result<()> {
    info!("{} Creating export file", emoji::WRENCH);
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }
    #[cfg(windows)]
    info!(
        "{} PLEASE set up the environment variables running:{}",
        emoji::INFO,
        export_file.display()
    );
    #[cfg(unix)]
    info!(
        "{} PLEASE set up the environment variables running:. {}",
        emoji::INFO,
        export_file.display()
    );
    info!(
        "{} This step must be done every time you open a new terminal.",
        emoji::WARN
    );
    Ok(())
}
