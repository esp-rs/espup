use crate::chip::Chip;
use crate::emoji;
use crate::espidf::get_dist_path;
use anyhow::{bail, Result};
use dirs::home_dir;
use flate2::bufread::GzDecoder;
use log::{debug, info};
use std::{
    collections::HashSet,
    fs::{create_dir_all, remove_dir_all, File},
    io::{copy, BufReader, Write},
    path::{Path, PathBuf},
};
use tar::Archive;
use xz2::read::XzDecoder;

pub mod logging {
    use env_logger::{Builder, Env, WriteStyle};

    /// Initializes the logger
    pub fn initialize_logger(log_level: &str) {
        Builder::from_env(Env::default().default_filter_or(log_level))
            .format_target(false)
            .format_timestamp_secs()
            .write_style(WriteStyle::Always)
            .init();
    }
}

/// Deletes dist folder.
pub fn clear_dist_folder() -> Result<()> {
    info!("{} Clearing dist folder", emoji::WRENCH);
    remove_dir_all(&get_dist_path(""))?;
    Ok(())
}

/// Returns a vector of Chips from a comma or space separated string.
pub fn parse_targets(targets: &str) -> Result<HashSet<Chip>, String> {
    debug!("{} Parsing targets: {}", emoji::DEBUG, targets);
    let mut chips: HashSet<Chip> = HashSet::new();
    if targets.contains("all") {
        chips.insert(Chip::ESP32);
        chips.insert(Chip::ESP32S2);
        chips.insert(Chip::ESP32S3);
        chips.insert(Chip::ESP32C3);
        return Ok(chips);
    }
    let targets: HashSet<&str> = if targets.contains(' ') || targets.contains(',') {
        targets.split([',', ' ']).collect()
    } else {
        vec![targets].into_iter().collect()
    };
    for target in targets {
        match target {
            "esp32" => chips.insert(Chip::ESP32),
            "esp32s2" => chips.insert(Chip::ESP32S2),
            "esp32s3" => chips.insert(Chip::ESP32S3),
            "esp32c3" => chips.insert(Chip::ESP32C3),
            _ => {
                return Err(format!("Unknown target: {}", target));
            }
        };
    }
    debug!("{} Parsed targets: {:?}", emoji::DEBUG, chips);
    Ok(chips)
}

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

/// Creates the export file with the neccesary environment variables.
pub fn export_environment(export_file: &PathBuf, exports: &[String]) -> Result<()> {
    info!("{} Creating export file", emoji::WRENCH);
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }
    #[cfg(windows)]
    info!(
        "{} PLEASE set up the environment variables running: '.\\{}'",
        emoji::INFO,
        export_file.display()
    );
    #[cfg(unix)]
    info!(
        "{} PLEASE set up the environment variables running: '. {}'",
        emoji::INFO,
        export_file.display()
    );
    info!(
        "{} This step must be done every time you open a new terminal.",
        emoji::WARN
    );
    Ok(())
}

#[cfg(windows)]
/// For Windows, we need to check that we are installing all the targets if we are installing esp-idf.
pub fn check_arguments(targets: &HashSet<Chip>, espidf_version: &Option<String>) -> Result<()> {
    if espidf_version.is_some()
        && (!targets.contains(&Chip::ESP32)
            || !targets.contains(&Chip::ESP32C3)
            || !targets.contains(&Chip::ESP32S2)
            || !targets.contains(&Chip::ESP32S3))
    {
        bail!(
            "{} When installing esp-idf in Windows, only --targets \"all\" is supported.",
            emoji::ERROR
        );
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::utils::parse_targets;
    use crate::Chip;
    #[test]
    fn test_parse_targets() {
        assert_eq!(parse_targets("esp32"), Ok([Chip::ESP32].to_vec()));
        assert_eq!(
            parse_targets("esp32,esp32s2"),
            Ok([Chip::ESP32, Chip::ESP32S2].to_vec())
        );
        assert_eq!(
            parse_targets("esp32s3 esp32"),
            Ok([Chip::ESP32S3, Chip::ESP32].to_vec())
        );
        assert_eq!(
            parse_targets("esp32s3,esp32,esp32c3"),
            Ok([Chip::ESP32S3, Chip::ESP32, Chip::ESP32C3].to_vec())
        );
        assert_eq!(
            parse_targets("all"),
            Ok([Chip::ESP32, Chip::ESP32S2, Chip::ESP32S3, Chip::ESP32C3].to_vec())
        );
    }
}
