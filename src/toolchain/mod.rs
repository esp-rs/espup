use crate::{emoji, error::Error};
use async_trait::async_trait;
use flate2::bufread::GzDecoder;
use log::info;
use miette::Result;
use std::{
    fs::{create_dir_all, File},
    io::Write,
    path::Path,
};
use tar::Archive;
use xz2::read::XzDecoder;

pub mod espidf;
pub mod gcc;
pub mod llvm;
pub mod rust;

#[async_trait]
pub trait Installable {
    /// Install some application, returning a vector of any required exports
    async fn install(&self) -> Result<Vec<String>, Error>;
}

/// Downloads a file from a URL and uncompresses it, if necesary, to the output directory.
pub async fn download_file(
    url: String,
    file_name: &str,
    output_directory: &str,
    uncompress: bool,
) -> Result<String, Error> {
    let file_path = format!("{}/{}", output_directory, file_name);
    if Path::new(&file_path).exists() {
        info!("{} Using cached file: '{}'", emoji::INFO, file_path);
        return Ok(file_path);
    } else if !Path::new(&output_directory).exists() {
        info!(
            "{} Creating directory: '{}'",
            emoji::WRENCH,
            output_directory
        );
        if let Err(_e) = create_dir_all(output_directory) {
            return Err(Error::FailedToCreateDirectory(output_directory.to_string()));
        }
    }
    info!(
        "{} Downloading file {} from {}",
        emoji::DOWNLOAD,
        file_name,
        url
    );
    let resp = reqwest::get(&url).await?;
    let bytes = resp.bytes().await?;
    if uncompress {
        let extension = Path::new(file_name).extension().unwrap().to_str().unwrap();
        match extension {
            "zip" => {
                let mut tmpfile = tempfile::tempfile()?;
                tmpfile.write_all(&bytes)?;
                let mut zipfile = zip::ZipArchive::new(tmpfile).unwrap();
                zipfile.extract(output_directory).unwrap();
            }
            "gz" => {
                info!(
                    "{} Uncompressing tar.gz file to '{}'",
                    emoji::WRENCH,
                    output_directory
                );

                let bytes = bytes.to_vec();
                let tarfile = GzDecoder::new(bytes.as_slice());
                let mut archive = Archive::new(tarfile);
                archive.unpack(output_directory)?;
            }
            "xz" => {
                info!(
                    "{} Uncompressing tar.xz file to '{}'",
                    emoji::WRENCH,
                    output_directory
                );
                let bytes = bytes.to_vec();
                let tarfile = XzDecoder::new(bytes.as_slice());
                let mut archive = Archive::new(tarfile);
                archive.unpack(output_directory)?;
            }
            _ => {
                return Err(Error::UnsuportedFileExtension(extension.to_string()));
            }
        }
    } else {
        info!("{} Creating file: '{}'", emoji::WRENCH, file_path);
        let mut out = File::create(file_path)?;
        out.write_all(&bytes)?;
    }
    Ok(format!("{}/{}", output_directory, file_name))
}

#[cfg(test)]
mod tests {
    use crate::toolchain::download_file;
    use std::{fs::File, io::Write};

    #[tokio::test]
    async fn test_download_file() {
        // Returns the correct file path when the file already exists
        let temp_dir = tempfile::TempDir::new().unwrap();
        let file_name = "test.txt";
        let output_directory = temp_dir.path().to_str().unwrap();
        let file_path = format!("{}/{}", output_directory, file_name);
        let mut file = File::create(file_path.clone()).unwrap();
        file.write_all(b"test content").unwrap();

        let url = "https://example.com/test.txt";
        let result = download_file(url.to_string(), file_name, output_directory, false).await;
        assert!(result.is_ok());
        let path = result.unwrap();
        assert_eq!(path, file_path);

        // Creates the output directory if it does not exist
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_directory = temp_dir.path().join("test");
        let file_name = "test.txt";

        let url = "https://example.com/test.txt";
        let result = download_file(
            url.to_string(),
            file_name,
            output_directory.to_str().unwrap(),
            false,
        )
        .await;
        assert!(result.is_ok());
        let path = result.unwrap();
        #[cfg(windows)]
        let path = path.replace('/', "\\");
        assert_eq!(path, output_directory.join(file_name).to_str().unwrap());
        assert!(output_directory.exists());

        // Downloads a ZIP file and uncompresses it
        let temp_dir = tempfile::TempDir::new().unwrap();
        let output_directory = temp_dir.path().to_str().unwrap();
        let file_name = "espup.zip";
        let url = "https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-unknown-linux-gnu.zip";
        let result = download_file(url.to_string(), file_name, output_directory, true).await;
        assert!(result.is_ok());
        let extracted_file = temp_dir.path().join("espup");
        assert!(extracted_file.exists());
    }
}
