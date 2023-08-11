//! Different toolchains source and installation tools.

use crate::{
    cli::InstallOpts,
    emoji,
    env::{create_export_file, export_environment, get_export_file},
    error::Error,
    host_triple::get_host_triple,
    targets::Target,
    toolchain::{
        gcc::Gcc,
        llvm::Llvm,
        rust::{check_rust_installation, get_rustup_home, RiscVTarget, XtensaRust},
    },
};
use async_trait::async_trait;
use flate2::bufread::GzDecoder;
use log::{debug, info, warn};
use miette::Result;
use reqwest::{blocking::Client, header};
use retry::{delay::Fixed, retry};
use std::{
    env,
    fs::{create_dir_all, remove_file, File},
    io::Write,
    path::{Path, PathBuf},
};
use tar::Archive;
use tokio::sync::mpsc;
use tokio_retry::{strategy::FixedInterval, Retry};
use xz2::read::XzDecoder;
use zip::ZipArchive;

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
    strip: bool,
) -> Result<String, Error> {
    let file_path = format!("{output_directory}/{file_name}");
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
            return Err(Error::CreateDirectory(output_directory.to_string()));
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
                let mut zipfile = ZipArchive::new(tmpfile).unwrap();
                if strip {
                    for i in 0..zipfile.len() {
                        let mut file = zipfile.by_index(i).unwrap();
                        if !file.name().starts_with("esp/") {
                            continue;
                        }

                        let file_path = PathBuf::from(file.name().to_string());
                        let stripped_name = file_path.strip_prefix("esp/").unwrap();
                        let outpath = Path::new(output_directory).join(stripped_name);

                        if file.name().ends_with('/') {
                            create_dir_all(&outpath)?;
                        } else {
                            create_dir_all(outpath.parent().unwrap())?;
                            let mut outfile = File::create(&outpath)?;
                            std::io::copy(&mut file, &mut outfile)?;
                        }
                    }
                } else {
                    zipfile.extract(output_directory).unwrap();
                }
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
    Ok(format!("{output_directory}/{file_name}"))
}

/// Installs or updates the Espressif Rust ecosystem.
pub async fn install(args: InstallOpts) -> Result<()> {
    let export_file = get_export_file(args.export_file)?;
    let mut exports: Vec<String> = Vec::new();
    let host_triple = get_host_triple(args.default_host)?;
    let xtensa_rust_version = if let Some(toolchain_version) = &args.toolchain_version {
        toolchain_version.clone()
    } else {
        XtensaRust::get_latest_version().await?
    };
    let install_path = get_rustup_home().join("toolchains").join(args.name);
    let llvm: Llvm = Llvm::new(
        &install_path,
        &host_triple,
        args.extended_llvm,
        &xtensa_rust_version,
    )?;
    let targets = args.targets;
    let xtensa_rust = if targets.contains(&Target::ESP32)
        || targets.contains(&Target::ESP32S2)
        || targets.contains(&Target::ESP32S3)
    {
        Some(XtensaRust::new(
            &xtensa_rust_version,
            &host_triple,
            &install_path,
        ))
    } else {
        None
    };

    debug!(
        "{} Arguments:
            - Export file: {:?}
            - Host triple: {}
            - LLVM Toolchain: {:?}
            - Nightly version: {:?}
            - Rust Toolchain: {:?}
            - Targets: {:?}
            - Toolchain path: {:?}
            - Toolchain version: {:?}",
        emoji::INFO,
        &export_file,
        host_triple,
        &llvm,
        &args.nightly_version,
        xtensa_rust,
        targets,
        &install_path,
        args.toolchain_version,
    );

    check_rust_installation().await?;

    // Build up a vector of installable applications, all of which implement the
    // `Installable` async trait.
    let mut to_install = Vec::<Box<dyn Installable + Send + Sync>>::new();

    if let Some(ref xtensa_rust) = xtensa_rust {
        to_install.push(Box::new(xtensa_rust.to_owned()));
    }

    to_install.push(Box::new(llvm));

    if targets.iter().any(|t| t.is_riscv()) {
        let riscv_target = RiscVTarget::new(&args.nightly_version);
        to_install.push(Box::new(riscv_target));
    }

    if !args.std {
        targets.iter().for_each(|target| {
            if target.is_xtensa() {
                let gcc = Gcc::new(target, &host_triple, &install_path);
                to_install.push(Box::new(gcc));
            }
        });
        // All RISC-V targets use the same GCC toolchain
        // ESP32S2 and ESP32S3 also install the RISC-V toolchain for their ULP coprocessor
        if targets.iter().any(|t| t != &Target::ESP32) {
            let riscv_gcc = Gcc::new_riscv(&host_triple, &install_path);
            to_install.push(Box::new(riscv_gcc));
        }
    }

    // With a list of applications to install, install them all in parallel.
    let installable_items = to_install.len();
    let (tx, mut rx) = mpsc::channel::<Result<Vec<String>, Error>>(installable_items);
    for app in to_install {
        let tx = tx.clone();
        let retry_strategy = FixedInterval::from_millis(50).take(3);
        tokio::spawn(async move {
            let res = Retry::spawn(retry_strategy, || async {
                let res = app.install().await;
                if res.is_err() {
                    warn!(
                        "{} Installation for '{}' failed, retrying",
                        emoji::WARN,
                        app.name()
                    );
                }
                res
            })
            .await;
            tx.send(res).await.unwrap();
        });
    }

    // Read the results of the install tasks as they complete.
    for _ in 0..installable_items {
        let names = rx.recv().await.unwrap()?;
        exports.extend(names);
    }

    create_export_file(&export_file, &exports)?;
    export_environment(&export_file)?;
    Ok(())
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
                return Err(Error::GithubQuery);
            }
            let json: serde_json::Value =
                serde_json::from_str(&res).map_err(|_| Error::SerializeJson)?;
            Ok(json)
        },
    )
    .unwrap();
    Ok(json)
}
