use crate::{emoji, error::Error};
use async_trait::async_trait;
use flate2::bufread::GzDecoder;
use log::{debug, info, warn};
use miette::Result;
use reqwest::blocking::Client;
use reqwest::header;
use retry::{delay::Fixed, retry};
use std::{
    env,
    fs::{create_dir_all, remove_file, File},
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
    /// Returns the name of the toolchain being installeds
    fn name(&self) -> String;
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
        warn!(
            "{} File '{}' already exists, deleting it before download.",
            emoji::WARN,
            file_path
        );
        remove_file(&file_path)?;
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
        "{} Downloading file '{}' from '{}'",
        emoji::DOWNLOAD,
        &file_path,
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

/// Queries the GitHub API and returns the JSON response.
pub fn github_query(url: &str) -> Result<serde_json::Value, Error> {
    info!("{} Querying GitHub API: '{}'", emoji::INFO, url);
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, "espup".parse().unwrap());
    headers.insert(
        header::ACCEPT,
        "application/vnd.github+json".parse().unwrap(),
    );
    headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
    if let Some(token) = env::var_os("GITHUB_TOKEN") {
        debug!("{} Auth header added.", emoji::DEBUG);
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.to_string_lossy())
                .parse()
                .unwrap(),
        );
    }
    let client = Client::new();
    let json = retry(
        Fixed::from_millis(100).take(5),
        || -> Result<serde_json::Value, Error> {
            let res = client.get(url).headers(headers.clone()).send()?.text()?;
            if res.contains(
                "https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting",
            ) {
                warn!("{} GitHub rate limit exceeded", emoji::WARN);
                return Err(Error::FailedGithubQuery);
            }
            let json: serde_json::Value =
                serde_json::from_str(&res).map_err(|_| Error::FailedToSerializeJson)?;
            Ok(json)
        },
    )
    .unwrap();
    Ok(json)
}
