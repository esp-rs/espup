//! Different toolchains source and installation tools.

#[cfg(windows)]
use crate::env::set_env;
use crate::{
    cli::InstallOpts,
    env::{create_export_file, get_export_file, print_post_install_msg},
    error::Error,
    host_triple::get_host_triple,
    targets::Target,
    toolchain::{
        gcc::{Gcc, RISCV_GCC, XTENSA_GCC},
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
    io::{copy, Write},
    path::{Path, PathBuf},
    sync::atomic::{self, AtomicUsize},
};
use tar::Archive;
use tokio::{fs::remove_dir_all, sync::mpsc};
use tokio_retry::{strategy::FixedInterval, Retry};
use tokio_stream::StreamExt;
use xz2::read::XzDecoder;
use zip::ZipArchive;

pub mod gcc;
pub mod llvm;
pub mod rust;

lazy_static::lazy_static! {
    pub static ref PROCESS_BARS: indicatif::MultiProgress = indicatif::MultiProgress::new();
    pub static ref DOWNLOAD_CNT: AtomicUsize = AtomicUsize::new(0);
}

pub enum InstallMode {
    Install,
    Update,
}

#[async_trait]
pub trait Installable {
    /// Install some application, returning a vector of any required exports
    async fn install(&self) -> Result<Vec<String>, Error>;
    /// Returns the name of the toolchain being installeds
    fn name(&self) -> String;
}

/// Get https proxy from environment variables(if any)
///
/// sadly there is not standard on the environment variable name for the proxy, but it seems
/// that the most common are:
///
/// - https_proxy(or http_proxy for http)
/// - HTTPS_PROXY(or HTTP_PROXY for http)
/// - all_proxy
/// - ALL_PROXY
///
/// hence we will check for all of them
fn https_proxy() -> Option<String> {
    for proxy in ["https_proxy", "HTTPS_PROXY", "all_proxy", "ALL_PROXY"] {
        if let Ok(proxy_addr) = std::env::var(proxy) {
            info!("Get Proxy from env var: {}={}", proxy, proxy_addr);
            return Some(proxy_addr);
        }
    }
    None
}

/// Build a reqwest client with proxy if env var is set
fn build_proxy_blocking_client() -> Result<Client, Error> {
    let mut builder = reqwest::blocking::Client::builder();
    if let Some(proxy) = https_proxy() {
        builder = builder.proxy(reqwest::Proxy::https(&proxy).unwrap());
    }
    let client = builder.build()?;
    Ok(client)
}

/// Build a reqwest client with proxy if env var is set
fn build_proxy_async_client() -> Result<reqwest::Client, Error> {
    let mut builder = reqwest::Client::builder();
    if let Some(proxy) = https_proxy() {
        builder = builder.proxy(reqwest::Proxy::https(&proxy).unwrap());
    }
    let client = builder.build()?;
    Ok(client)
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
            "File '{}' already exists, deleting it before download",
            file_path
        );
        remove_file(&file_path)?;
    } else if !Path::new(&output_directory).exists() {
        debug!("Creating directory: '{}'", output_directory);
        create_dir_all(output_directory)
            .map_err(|_| Error::CreateDirectory(output_directory.to_string()))?;
    }

    let resp = {
        let client = build_proxy_async_client()?;
        let resp = client.get(&url).send().await?;
        if !resp.status().is_success() {
            return Err(Error::HttpError(resp.status().to_string()));
        }
        resp
    };
    let bytes = {
        let len = resp.content_length();

        // draw a progress bar
        let sty = indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
        )
        .unwrap()
        .progress_chars("##-");
        let bar = len
            .map(indicatif::ProgressBar::new)
            .unwrap_or(indicatif::ProgressBar::no_length());
        let bar = PROCESS_BARS.add(bar);
        bar.set_style(sty);
        bar.set_message(file_name.to_string());
        DOWNLOAD_CNT.fetch_add(1, atomic::Ordering::Relaxed);

        let mut size_downloaded = 0;
        let mut stream = resp.bytes_stream();
        let mut bytes = bytes::BytesMut::new();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result?;
            size_downloaded += chunk.len();
            bar.set_position(size_downloaded as u64);

            bytes.extend(&chunk);
        }
        bar.finish_with_message(format!("{} download complete", file_name));
        // leave the progress bar after completion
        if DOWNLOAD_CNT.fetch_sub(1, atomic::Ordering::Relaxed) == 1 {
            // clear all progress bars
            PROCESS_BARS.clear().unwrap();
            info!("All downloads complete");
        }
        // wait while DOWNLOAD_CNT is not zero

        bytes.freeze()
    };
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
                            copy(&mut file, &mut outfile)?;
                        }
                    }
                } else {
                    zipfile.extract(output_directory).unwrap();
                }
            }
            "gz" => {
                debug!("Extracting tar.gz file to '{}'", output_directory);

                let bytes = bytes.to_vec();
                let tarfile = GzDecoder::new(bytes.as_slice());
                let mut archive = Archive::new(tarfile);
                archive.unpack(output_directory)?;
            }
            "xz" => {
                debug!("Extracting tar.xz file to '{}'", output_directory);
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
        debug!("Creating file: '{}'", file_path);
        let mut out = File::create(&file_path)?;
        out.write_all(&bytes)?;
    }
    Ok(file_path)
}

/// Installs or updates the Espressif Rust ecosystem.
pub async fn install(args: InstallOpts, install_mode: InstallMode) -> Result<()> {
    match install_mode {
        InstallMode::Install => info!("Installing the Espressif Rust ecosystem"),
        InstallMode::Update => info!("Updating the Espressif Rust ecosystem"),
    }
    let export_file = get_export_file(args.export_file)?;
    let mut exports: Vec<String> = Vec::new();
    let host_triple = get_host_triple(args.default_host)?;
    let xtensa_rust_version = if let Some(toolchain_version) = &args.toolchain_version {
        if !args.skip_version_parse {
            XtensaRust::parse_version(toolchain_version)?
        } else {
            toolchain_version.clone()
        }
    } else {
        // Get the latest version of the Xtensa Rust toolchain. If that fails, return an error::GithubTokenInvalid
        XtensaRust::get_latest_version()
            .await
            .map_err(|_| Error::GithubTokenInvalid)?
    };
    let toolchain_dir = get_rustup_home().join("toolchains").join(args.name);
    let llvm: Llvm = Llvm::new(
        &toolchain_dir,
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
            &toolchain_dir,
        ))
    } else {
        None
    };

    debug!(
        "Arguments:
            - Export file: {:?}
            - Host triple: {}
            - LLVM Toolchain: {:?}
            - Nightly version: {:?}
            - Rust Toolchain: {:?}
            - Skip version parsing: {}
            - Targets: {:?}
            - Toolchain path: {:?}
            - Toolchain version: {:?}",
        &export_file,
        host_triple,
        &llvm,
        &args.nightly_version,
        xtensa_rust,
        &args.skip_version_parse,
        targets,
        &toolchain_dir,
        args.toolchain_version,
    );

    check_rust_installation().await?;

    // Build up a vector of installable applications, all of which implement the
    // `Installable` async trait.
    let mut to_install = Vec::<Box<dyn Installable + Send + Sync>>::new();

    if let Some(ref xtensa_rust) = xtensa_rust {
        to_install.push(Box::new(xtensa_rust.to_owned()));
    }

    // Check if ther is any Xtensa target
    if targets.iter().any(|t| t.is_xtensa()) {
        to_install.push(Box::new(llvm.to_owned()));
    }

    if targets.iter().any(|t| t.is_riscv()) {
        let riscv_target = RiscVTarget::new(&args.nightly_version);
        to_install.push(Box::new(riscv_target));
    }

    if !args.std {
        if targets
            .iter()
            .any(|t| t == &Target::ESP32 || t == &Target::ESP32S2 || t == &Target::ESP32S3)
        {
            let xtensa_gcc = Gcc::new(XTENSA_GCC, &host_triple, &toolchain_dir);
            to_install.push(Box::new(xtensa_gcc));
        }

        // By default only install the Espressif RISC-V toolchain if the user explicitly wants to
        if args.esp_riscv_gcc && targets.iter().any(|t| t != &Target::ESP32) {
            let riscv_gcc = Gcc::new(RISCV_GCC, &host_triple, &toolchain_dir);
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
                if let Err(ref err) = res {
                    warn!(
                        "Installation for '{}' failed, retrying. Error: {}",
                        app.name(),
                        err
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
    #[cfg(windows)]
    set_env()?;
    match install_mode {
        InstallMode::Install => info!("Installation successfully completed!"),
        InstallMode::Update => info!("Update successfully completed!"),
    }

    print_post_install_msg(&export_file)?;
    Ok(())
}

/// Queries the GitHub API and returns the JSON response.
pub fn github_query(url: &str) -> Result<serde_json::Value, Error> {
    debug!("Querying GitHub API: '{}'", url);
    let mut headers = header::HeaderMap::new();
    headers.insert(header::USER_AGENT, "espup".parse().unwrap());
    headers.insert(
        header::ACCEPT,
        "application/vnd.github+json".parse().unwrap(),
    );

    headers.insert("X-GitHub-Api-Version", "2022-11-28".parse().unwrap());
    if let Some(token) = env::var_os("GITHUB_TOKEN") {
        debug!("Auth header added");
        headers.insert(
            "Authorization",
            format!("Bearer {}", token.to_string_lossy())
                .parse()
                .unwrap(),
        );
    }
    let client = build_proxy_blocking_client()?;
    let json: Result<serde_json::Value, Error> = retry(
        Fixed::from_millis(100).take(5),
        || -> Result<serde_json::Value, Error> {
            let res = client.get(url).headers(headers.clone()).send()?.text()?;
            if res.contains(
                "https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting",
            ) {
                return Err(Error::GithubRateLimit);
            }

            if res.contains("Bad credentials") {
                return Err(Error::GithubTokenInvalid);
            }

            let json: serde_json::Value =
                serde_json::from_str(&res).map_err(|_| Error::SerializeJson)?;
            Ok(json)
        },
    )
    .map_err(|err| err.error);
    json
}

/// Checks if the directory exists and deletes it if it does.
pub async fn remove_dir(path: &Path) -> Result<()> {
    if path.exists() {
        debug!(
            "Deleting the Xtensa Rust toolchain located in '{}'",
            &path.display()
        );
        remove_dir_all(&path)
            .await
            .map_err(|_| Error::RemoveDirectory(path.display().to_string()))?;
    }
    Ok(())
}
