use crate::emoji;
#[cfg(windows)]
use crate::targets::Target;
use anyhow::{bail, Result};
use dirs::home_dir;
use flate2::bufread::GzDecoder;
use log::info;
#[cfg(windows)]
use std::collections::HashSet;
use std::{
    fs::{create_dir_all, File},
    io::{copy, BufReader},
    path::Path,
};
use tar::Archive;
use xz2::read::XzDecoder;

pub mod espidf;
pub mod gcc_toolchain;
pub mod llvm_toolchain;
pub mod rust_toolchain;

/// Returns the path to the home directory.
pub fn get_home_dir() -> String {
    home_dir().unwrap().display().to_string()
}

/// Downloads a file from a URL and uncompresses it, if necesary, to the output directory.
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
        if let Err(_e) = create_dir_all(output_directory) {
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
        copy(&mut resp, &mut out)?;
    }
    Ok(format!("{}/{}", output_directory, file_name))
}
