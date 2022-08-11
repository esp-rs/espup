use anyhow::Context;
use std::fs::File;
use std::io::Cursor;
use std::{path::{Component::Normal, Path, PathBuf}};
use std::{fs, io};
use tar::Archive;
use xz2::read::XzDecoder;
use crate::config::{get_dist_path, get_tool_path};
use flate2::bufread::GzDecoder;
use std::io::BufReader;
use tokio::runtime::Handle;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

pub fn unzip(file_path: String, output_directory: String) -> Result<()> {
    let file_name = std::path::Path::new(&file_path);
    let file = fs::File::open(&file_name).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let file_outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // Add path prefix to extract the file
        let mut outpath = std::path::PathBuf::new();
        outpath.push(&output_directory);
        outpath.push(file_outpath);

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (&*file.name()).ends_with('/') {
            println!("* extracted: \"{}\"", outpath.display());
            fs::create_dir_all(&outpath).unwrap();
        } else {
            println!(
                "* extracted: \"{}\" ({} bytes)",
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
    Ok(())
}

pub fn unzip_strip_prefix(
    file_path: String,
    output_directory: String,
    strip_prefix: &str,
) -> Result<()> {
    let file_name = std::path::Path::new(&file_path);
    let file = fs::File::open(&file_name).unwrap();

    let mut archive = zip::ZipArchive::new(file).unwrap();

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).unwrap();
        let file_outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        // Add path prefix to extract the file
        let mut outpath = std::path::PathBuf::new();
        outpath.push(&output_directory);

        // Skip files in top level directories which are not under directory with prefix
        if !file_outpath.starts_with(strip_prefix) {
            println!("* skipped: \"{}\"", file_outpath.display());
            continue;
        }

        let stripped_file_outpath = file_outpath.strip_prefix(strip_prefix).unwrap();
        outpath.push(stripped_file_outpath);

        {
            let comment = file.comment();
            if !comment.is_empty() {
                println!("File {} comment: {}", i, comment);
            }
        }

        if (&*file.name()).ends_with('/') {
            if !Path::new(file.name()).exists() {
                println!("* created: \"{}\"", outpath.display());
                fs::create_dir_all(&outpath).unwrap();
            }
        } else {
            println!(
                "* extracted: \"{}\" ({} bytes)",
                outpath.display(),
                file.size()
            );
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(&p).unwrap();
                }
            }
            let mut outfile = fs::File::create(&outpath).unwrap();
            io::copy(&mut file, &mut outfile).unwrap();
        }
    }
    Ok(())
}

pub fn untar_strip_prefix(
    file_path: String,
    output_directory: String,
    strip_prefix: &str,
) -> Result<()> {
    println!(
        "Untar: file {} output_dir {} strip_prefix {}",
        file_path, output_directory, strip_prefix
    );
    let tar_xz = File::open(file_path)?;
    let tar = XzDecoder::new(tar_xz);
    let mut archive = Archive::new(tar);
    archive
        .entries()?
        .filter_map(|e| e.ok())
        .map(|mut entry| -> Result<PathBuf> {
            let path = entry.path()?.strip_prefix(strip_prefix)?.to_owned();
            let full_path = format!("{}/{}", output_directory, path.display().to_string());
            entry.unpack(&full_path)?;
            Ok(full_path.parse().unwrap())
        })
        .filter_map(|e| e.ok())
        .for_each(|x| println!("> {}", x.display()));
    Ok(())
}

async fn fetch_url(url: String, output: String) -> Result<()> {
    let response = reqwest::get(&url).await;
    match response {
        Ok(r) => {
            let mut file = std::fs::File::create(output)?;
            let mut content = Cursor::new(r.bytes().await?);
            std::io::copy(&mut content, &mut file)?;

            return Ok(());
        }
        _ => {
            println!("Download of {} failed", url);
            // Exit code is 0, there is temporal issue with Windows Installer which does not recover from error exit code
            #[cfg(windows)]
            std::process::exit(0);
            #[cfg(unix)]
            std::process::exit(1);
        }
    };
}

async fn download_file(url: String, output: String) -> Result<()> {
    if Path::new(&output).exists() {
        println!("Using cached archive: {}", output);
        return Ok(());
    }
    println!("Downloading {} to {}", url, output);
    fetch_url(url, output).await
}

pub fn download_package(package_url: String, package_archive: String) -> Result<()> {
    let handle = Handle::current().clone();
    let th = std::thread::spawn(move || {
        handle
            .block_on(download_file(package_url, package_archive))
            .unwrap();
    });
    Ok(th.join().unwrap())
}

pub fn prepare_package(
    package_url: String,
    package_archive: &str,
    output_directory: String,
) -> Result<()> {
    if Path::new(&output_directory).exists() {
        println!("Using cached directory: {}", output_directory);
        return Ok(());
    }

    let dist_path = get_dist_path("");
    if !Path::new(&dist_path).exists() {
        println!("Creating dist directory: {}", dist_path);
        match fs::create_dir_all(&dist_path) {
            Ok(_) => {
                println!("Ok");
            }
            Err(_e) => {
                println!("Failed");
            }
        }
    }

    let package_archive = get_dist_path(package_archive);

    match download_package(package_url, package_archive.clone()) {
        Ok(_) => {
            println!("Ok");
        }
        Err(_e) => {
            println!("Failed");
        }
    }
    unzip(package_archive, output_directory).unwrap();
    Ok(())
}

pub fn prepare_single_binary(
    package_url: &str,
    binary_name: &str,
    output_directory: &str,
) -> String {
    let tool_path = get_tool_path(output_directory.to_string());
    let binary_path = format!("{}/{}", tool_path, binary_name);

    if Path::new(&binary_path).exists() {
        println!("Using cached tool: {}", binary_path);
        return binary_path;
    }

    if !Path::new(&tool_path).exists() {
        println!("Creating tool directory: {}", tool_path);
        match fs::create_dir_all(&tool_path) {
            Ok(_) => {
                println!("Ok");
            }
            Err(_e) => {
                println!("Failed");
            }
        }
    }

    match download_package(package_url.to_string(), binary_path.to_string()) {
        Ok(_) => {
            println!("Ok");
        }
        Err(_e) => {
            println!("Failed");
        }
    }
    return binary_path;
}

pub fn prepare_package_strip_prefix(
    package_url: &str,
    output_directory: String,
    strip_prefix: &str,
) -> Result<()> {
    println!("prepare_package_strip_prefix: 
                        -pacakge_url: {}
                        -output dir: {}
                        -strip_prefix: {}", &package_url, &output_directory, &strip_prefix);

    if Path::new(&output_directory).exists() {
        println!("Using cached directory: {}", output_directory);
        return Ok(());
    }
    let tools_path = get_tool_path("".to_string());
    if !Path::new(&tools_path).exists() {
        println!("Creating tools directory: {}", tools_path);
        match fs::create_dir_all(&tools_path) {
            Ok(_) => {
                println!("tools_path created");
            }
            Err(_e) => {
                println!("tools_path creating failed");
            }
        }
    } 
    let resp = reqwest::blocking::get(package_url).unwrap();
    let content_br = BufReader::new(resp);
    let tarfile = XzDecoder::new(content_br);
    let mut archive = Archive::new(tarfile);
    archive.unpack(&tools_path)?;
    let extracted_folder = format!("{}{}", &tools_path, strip_prefix);
    println!("Renaming: {} to {}", &extracted_folder, &output_directory);
    fs::rename(extracted_folder, output_directory)?;
    Ok(())
}



pub fn remove_package(package_archive: &str, output_directory: &str) -> Result<()> {
    if Path::new(package_archive).exists() {
        fs::remove_file(package_archive)
            .with_context(|| format!("Unable to delete `{}`", package_archive))?;
    }
    if Path::new(output_directory).exists() {
        fs::remove_dir_all(output_directory)
            .with_context(|| format!("Unable to delete `{}`", output_directory))?;
    }
    Ok(())
}
