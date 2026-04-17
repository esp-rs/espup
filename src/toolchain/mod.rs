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
        rust::{RiscVTarget, XtensaRust, check_rust_installation, get_rustup_home},
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
    fs::{File, OpenOptions, create_dir_all, remove_file},
    io::{BufReader, Write, copy},
    path::{Path, PathBuf},
    sync::atomic::{self, AtomicBool, AtomicUsize},
};
use tar::Archive;
use tokio::{fs::remove_dir_all, sync::mpsc};
use tokio_retry2::{Retry, RetryError, strategy::FixedInterval};
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

static DISABLE_HTTP_TIMEOUTS: AtomicBool = AtomicBool::new(false);

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
            info!("Get Proxy from env var: {proxy}={proxy_addr}");
            return Some(proxy_addr);
        }
    }
    None
}

fn disable_http_timeouts() -> bool {
    DISABLE_HTTP_TIMEOUTS.load(atomic::Ordering::Relaxed)
}

fn set_disable_http_timeouts(disable: bool) {
    DISABLE_HTTP_TIMEOUTS.store(disable, atomic::Ordering::Relaxed);
}

/// Build a reqwest client with proxy if env var is set
fn build_proxy_blocking_client() -> Result<Client, Error> {
    let mut builder = reqwest::blocking::Client::builder();
    if disable_http_timeouts() {
        debug!("HTTP timeouts disabled for blocking client");
        builder = builder.timeout(None);
    }
    if let Some(proxy) = https_proxy() {
        builder = builder.proxy(reqwest::Proxy::https(&proxy).unwrap());
    }
    let client = builder.build()?;
    Ok(client)
}

/// Build a reqwest client with proxy if env var is set
fn build_proxy_async_client() -> Result<reqwest::Client, Error> {
    let mut builder = reqwest::Client::builder();
    if disable_http_timeouts() {
        debug!("HTTP timeouts disabled; async client already uses no timeout by default");
    }
    if let Some(proxy) = https_proxy() {
        builder = builder.proxy(reqwest::Proxy::https(&proxy).unwrap());
    }
    let client = builder.build()?;
    Ok(client)
}

fn create_download_progress_bar(file_name: &str, total_len: Option<u64>) -> indicatif::ProgressBar {
    let sty = indicatif::ProgressStyle::with_template(
        "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len:7} {msg}",
    )
    .unwrap()
    .progress_chars("##-");
    let bar = total_len
        .map(indicatif::ProgressBar::new)
        .unwrap_or(indicatif::ProgressBar::no_length());
    let bar = PROCESS_BARS.add(bar);
    bar.set_style(sty);
    bar.set_message(file_name.to_string());
    DOWNLOAD_CNT.fetch_add(1, atomic::Ordering::Relaxed);
    bar
}

fn finish_download_progress_bar(bar: indicatif::ProgressBar, message: String) {
    bar.finish_with_message(message);
    if DOWNLOAD_CNT.fetch_sub(1, atomic::Ordering::Relaxed) == 1 {
        PROCESS_BARS.clear().unwrap();
        info!("All downloads complete");
    }
}

async fn download_file_with_resume(
    url: &str,
    file_name: &str,
    destination: &Path,
) -> Result<(), Error> {
    const MAX_DOWNLOAD_RETRIES: usize = 10;

    let client = build_proxy_async_client()?;
    let mut downloaded = destination
        .metadata()
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let bar = create_download_progress_bar(file_name, None);
    if downloaded > 0 {
        bar.set_position(downloaded);
        info!("Found partial download for '{file_name}', resuming from byte {downloaded}");
    }

    let mut retries = 0;
    loop {
        let mut request = client.get(url);
        if downloaded > 0 {
            request = request.header(header::RANGE, format!("bytes={downloaded}-"));
        }

        let response = match request.send().await {
            Ok(response) => response,
            Err(err) if retries < MAX_DOWNLOAD_RETRIES => {
                retries += 1;
                warn!(
                    "Download of '{file_name}' failed before receiving data, retrying ({retries}/{MAX_DOWNLOAD_RETRIES}): {err}"
                );
                continue;
            }
            Err(err) => {
                finish_download_progress_bar(bar, format!("{file_name} download failed"));
                return Err(err.into());
            }
        };

        match response.status() {
            status if downloaded == 0 && status.is_success() => {}
            reqwest::StatusCode::PARTIAL_CONTENT if downloaded > 0 => {}
            reqwest::StatusCode::RANGE_NOT_SATISFIABLE if downloaded > 0 => {
                warn!(
                    "Partial download for '{file_name}' can no longer be resumed, restarting from scratch"
                );
                remove_file(destination)?;
                downloaded = 0;
                bar.set_position(0);
                continue;
            }
            status if downloaded > 0 && status.is_success() => {
                warn!("Server ignored resume request for '{file_name}', restarting from scratch");
                remove_file(destination)?;
                downloaded = 0;
                bar.set_position(0);
                continue;
            }
            status => {
                finish_download_progress_bar(bar, format!("{file_name} download failed"));
                return Err(Error::HttpError(status.to_string()));
            }
        }

        let total_len = if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
            response
                .content_length()
                .map(|remaining| remaining + downloaded)
        } else {
            response.content_length()
        };
        if let Some(total_len) = total_len {
            bar.set_length(total_len);
        }

        let mut output = OpenOptions::new()
            .create(true)
            .append(downloaded > 0)
            .truncate(downloaded == 0)
            .write(true)
            .open(destination)?;

        let mut stream = response.bytes_stream();
        let mut completed = true;
        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    output.write_all(&chunk)?;
                    downloaded += chunk.len() as u64;
                    bar.set_position(downloaded);
                }
                Err(err) if retries < MAX_DOWNLOAD_RETRIES => {
                    retries += 1;
                    completed = false;
                    warn!(
                        "Download of '{file_name}' was interrupted at byte {downloaded}, retrying ({retries}/{MAX_DOWNLOAD_RETRIES}): {err}"
                    );
                    break;
                }
                Err(err) => {
                    finish_download_progress_bar(bar, format!("{file_name} download failed"));
                    return Err(err.into());
                }
            }
        }
        output.flush()?;

        if completed {
            if let Some(total_len) = total_len
                && downloaded < total_len
            {
                if retries < MAX_DOWNLOAD_RETRIES {
                    retries += 1;
                    warn!(
                        "Download of '{file_name}' ended early at byte {downloaded}/{total_len}, retrying ({retries}/{MAX_DOWNLOAD_RETRIES})"
                    );
                    continue;
                }
                finish_download_progress_bar(bar, format!("{file_name} download failed"));
                return Err(Error::HttpError(format!(
                    "Incomplete download for '{file_name}': received {downloaded} of {total_len} bytes"
                )));
            }

            finish_download_progress_bar(bar, format!("{file_name} download complete"));
            return Ok(());
        }
    }
}

fn extract_downloaded_file(
    file_name: &str,
    archive_path: &Path,
    output_directory: &str,
    strip: bool,
) -> Result<(), Error> {
    let mut extension = archive_path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default();

    // Resumable downloads are stored as '<name>.<ext>.part'.
    // Detect the real archive extension from the stem when needed.
    if extension == "part" {
        extension = archive_path
            .file_stem()
            .and_then(|stem| Path::new(stem).extension())
            .and_then(|ext| ext.to_str())
            .unwrap_or("part");
    }

    match extension {
        "zip" => {
            let file = File::open(archive_path)?;
            let mut zipfile = ZipArchive::new(file).unwrap();
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
            debug!("Extracting tar.gz file to '{output_directory}'");
            let tarfile = File::open(archive_path)?;
            let tarfile = GzDecoder::new(BufReader::new(tarfile));
            let mut archive = Archive::new(tarfile);
            archive.unpack(output_directory)?;
        }
        "xz" => {
            debug!("Extracting tar.xz file to '{output_directory}'");
            let tarfile = File::open(archive_path)?;
            let tarfile = XzDecoder::new(tarfile);
            let mut archive = Archive::new(tarfile);
            archive.unpack(output_directory)?;
        }
        _ => {
            return Err(Error::UnsuportedFileExtension(extension.to_string()));
        }
    }

    debug!("Extracted '{file_name}' to '{output_directory}'");
    Ok(())
}

/// Downloads a file from a URL and uncompresses it, if necesary, to the output directory.
pub async fn download_file(
    url: String,
    file_name: &str,
    output_directory: &str,
    uncompress: bool,
    strip: bool,
) -> Result<String, Error> {
    let file_path = Path::new(output_directory).join(file_name);
    let partial_file_path = PathBuf::from(format!("{}.part", file_path.display()));

    if !Path::new(output_directory).exists() {
        debug!("Creating directory: '{output_directory}'");
        create_dir_all(output_directory)
            .map_err(|_| Error::CreateDirectory(output_directory.to_string()))?;
    } else if file_path.exists() {
        warn!(
            "File '{}' already exists, deleting it before download",
            file_path.display()
        );
        remove_file(&file_path)?;
    }

    download_file_with_resume(&url, file_name, &partial_file_path).await?;

    if uncompress {
        extract_downloaded_file(file_name, &partial_file_path, output_directory, strip)?;
        remove_file(&partial_file_path)?;
    } else {
        debug!("Creating file: '{}'", file_path.display());
        std::fs::rename(&partial_file_path, &file_path)?;
    }

    Ok(file_path.display().to_string())
}

/// Installs or updates the Espressif Rust ecosystem.
pub async fn install(args: InstallOpts, install_mode: InstallMode) -> Result<()> {
    set_disable_http_timeouts(args.disable_timeouts);
    if args.disable_timeouts {
        info!("HTTP timeouts disabled");
    }

    match install_mode {
        InstallMode::Install => info!("Installing the Espressif Rust ecosystem"),
        InstallMode::Update => info!("Updating the Espressif Rust ecosystem"),
    }
    let export_file = get_export_file(args.export_file)?;
    let mut exports: Vec<String> = Vec::new();
    let host_triple = get_host_triple(args.default_host)?;
    let xtensa_rust_version = if let Some(toolchain_version) = &args.toolchain_version {
        if !args.skip_version_parse {
            XtensaRust::find_latest_version_on_github(toolchain_version)?
        } else {
            toolchain_version.clone()
        }
    } else {
        // Get the latest version of the Xtensa Rust toolchain
        XtensaRust::get_latest_version().await.map_err(|e| {
            warn!("Failed to get latest Xtensa Rust version: {e}");
            e
        })?
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
            - Disable timeouts: {}
            - Host triple: {}
            - LLVM Toolchain: {:?}
            - Stable version: {:?}
            - Rust Toolchain: {:?}
            - Skip version parsing: {}
            - Targets: {:?}
            - Toolchain path: {:?}
            - Toolchain version: {:?}",
        &export_file,
        &args.disable_timeouts,
        host_triple,
        &llvm,
        &args.stable_version,
        xtensa_rust,
        &args.skip_version_parse,
        targets,
        &toolchain_dir,
        args.crosstool_toolchain_version,
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
        let riscv_target = RiscVTarget::new(&args.stable_version);
        to_install.push(Box::new(riscv_target));
    }

    if !args.std {
        if targets
            .iter()
            .any(|t| t == &Target::ESP32 || t == &Target::ESP32S2 || t == &Target::ESP32S3)
        {
            let xtensa_gcc = Gcc::new(
                XTENSA_GCC,
                &host_triple,
                &toolchain_dir,
                args.crosstool_toolchain_version.clone(),
            );
            to_install.push(Box::new(xtensa_gcc));
        }

        // By default only install the Espressif RISC-V toolchain if the user explicitly wants to
        if args.esp_riscv_gcc && targets.iter().any(|t| t != &Target::ESP32) {
            let riscv_gcc = Gcc::new(
                RISCV_GCC,
                &host_triple,
                &toolchain_dir,
                args.crosstool_toolchain_version.clone(),
            );
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
                res.map_err(RetryError::transient)
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
    debug!("Querying GitHub API: '{url}'");
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
            let response = client.get(url).headers(headers.clone()).send()?;
            let status = response.status();

            if !status.is_success() {
                return Err(Error::HttpError(format!(
                    "GitHub API returned status code: {status}"
                )));
            }

            let res = response.text()?;

            // Check for rate limiting response
            if res.contains(
                "https://docs.github.com/rest/overview/resources-in-the-rest-api#rate-limiting",
            ) {
                return Err(Error::GithubRateLimit);
            }

            // Check for authentication errors
            if res.contains("Bad credentials") {
                return Err(Error::GithubTokenInvalid);
            }

            // Try to parse the JSON
            serde_json::from_str(&res).map_err(|_| Error::SerializeJson)
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
