use dirs::home_dir;
use espflash::Chip;
use flate2::bufread::GzDecoder;
use std::env;
use std::fs;
use std::io::BufReader;
use std::io::Cursor;
use std::path::Path;
use std::process::Stdio;
use tar::Archive;
use tokio::runtime::Handle;
use xz2::read::XzDecoder;

pub fn parse_targets(build_target: &str) -> Result<Vec<Chip>, String> {
    println!("Parsing targets: {}", build_target);
    let mut chips: Vec<Chip> = Vec::new();
    if build_target.contains("all") {
        chips.push(Chip::Esp32);
        chips.push(Chip::Esp32s2);
        chips.push(Chip::Esp32s3);
        chips.push(Chip::Esp32c3);
        return Ok(chips);
    }
    let targets: Vec<&str>;
    if build_target.contains(' ') || build_target.contains(',') {
        targets = build_target.split([',', ' ']).collect();
    } else {
        targets = vec![build_target];
    }
    for target in targets {
        match target {
            "esp32" => chips.push(Chip::Esp32),
            "esp32s2" => chips.push(Chip::Esp32s2),
            "esp32s3" => chips.push(Chip::Esp32s3),
            "esp32c3" => chips.push(Chip::Esp32c3),
            _ => {
                return Err(format!("Unknown target: {}", target));
            }
        };
    }

    Ok(chips)
}

pub fn parse_llvm_version(llvm_version: &str) -> Result<String, String> {
    let parsed_version = match llvm_version {
        "13" => "esp-13.0.0-20211203",
        "14" => "esp-14.0.0-20220415",
        "15" => "", // TODO: Fill when released
        _ => {
            return Err(format!("Unknown LLVM Version: {}", llvm_version));
        }
    };

    Ok(parsed_version.to_string())
}

pub fn get_llvm_version_with_underscores(llvm_version: &str) -> String {
    let version: Vec<&str> = llvm_version.split("-").collect();
    let llvm_dot_version = version[1];
    llvm_dot_version.replace(".", "_")
}

pub fn get_artifact_file_extension(arch: &str) -> &str {
    match arch {
        "x86_64-pc-windows-msvc" => "zip",
        "x86_64-pc-windows-gnu" => "zip",
        _ => "tar.xz",
    }
}

pub fn get_llvm_arch(arch: &str) -> &str {
    match arch {
        "aarch64-apple-darwin" => "macos",
        "x86_64-apple-darwin" => "macos",
        "x86_64-unknown-linux-gnu" => "linux-amd64",
        "x86_64-pc-windows-msvc" => "win64",
        "x86_64-pc-windows-gnu" => "win64",
        _ => arch,
    }
}

pub fn get_gcc_arch(arch: &str) -> &str {
    match arch {
        "aarch64-apple-darwin" => "macos",
        "aarch64-unknown-linux-gnu" => "linux-arm64",
        "x86_64-apple-darwin" => "macos",
        "x86_64-unknown-linux-gnu" => "linux-amd64",
        "x86_64-pc-windows-msvc" => "win64",
        "x86_64-pc-windows-gnu" => "win64",
        _ => arch,
    }
}

pub fn get_rust_installer(arch: &str) -> &str {
    match arch {
        "x86_64-pc-windows-msvc" => "",
        "x86_64-pc-windows-gnu" => "",
        _ => "./install.sh",
    }
}

pub fn get_espressif_base_path() -> String {
    env::var("IDF_TOOLS_PATH")
        .unwrap_or_else(|_e| home_dir().unwrap().display().to_string() + "/.espressif/")
}

pub fn get_tool_path(tool_name: &str) -> String {
    format!("{}/tools/{}", get_espressif_base_path(), tool_name)
}

pub fn prepare_package_strip_prefix(
    package_url: &str,
    output_directory: String,
    strip_prefix: &str,
) -> Result<(), String> {
    println!(
        "prepare_package_strip_prefix: 
                        -pacakge_url: {}
                        -output dir: {}
                        -strip_prefix: {}",
        &package_url, &output_directory, &strip_prefix
    );

    if Path::new(&output_directory).exists() {
        println!("Using cached directory: {}", output_directory);
        return Ok(());
    }
    let tools_path = get_tool_path("");
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
    if package_url.contains(".xz") {
        let tarfile = XzDecoder::new(content_br);
        let mut archive = Archive::new(tarfile);
        archive.unpack(&tools_path).unwrap();
    } else {
        let tarfile = GzDecoder::new(content_br);
        let mut archive = Archive::new(tarfile);
        archive.unpack(&tools_path).unwrap();
    }
    if !strip_prefix.is_empty() {
        let extracted_folder = format!("{}{}", &tools_path, strip_prefix);
        println!("Renaming: {} to {}", &extracted_folder, &output_directory);
        fs::rename(extracted_folder, output_directory).unwrap();
    }
    Ok(())
}

#[cfg(windows)]
pub fn run_command(
    shell: String,
    arguments: Vec<String>,
    command: String,
) -> std::result::Result<(), clap::Error> {
    // println!("arguments = {:?}", arguments);
    let mut child_process = std::process::Command::new(shell)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {
        let child_stdin = child_process.stdin.as_mut().unwrap();
        child_stdin.write_all(&*command.into_bytes())?;
        // Close stdin to finish and avoid indefinite blocking
        drop(child_stdin);
    }
    let _output = child_process.wait_with_output()?;

    // println!("output = {:?}", output);

    Ok(())
}

#[cfg(unix)]
pub fn run_command(
    shell: String,
    arguments: Vec<String>,
    command: String,
) -> std::result::Result<(), clap::Error> {
    // Unix - pass command as parameter for initializer
    let mut arguments = arguments.clone();
    if !command.is_empty() {
        arguments.push(command);
    }

    //println!("arguments = {:?}", arguments);
    std::process::Command::new(shell)
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    {}
    // let output = child_process.wait_with_output()?;
    // println!("output = {:?}", output);
    Ok(())
}

pub fn prepare_single_binary(
    package_url: &str,
    binary_name: &str,
    output_directory: &str,
) -> String {
    let tool_path = get_tool_path(output_directory);
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

pub fn get_python_env_path(idf_version: &str, python_version: &str) -> String {
    let tools_path = get_espressif_base_path();
    format!(
        "{}/python_env/idf{}_py{}_env",
        tools_path, idf_version, python_version
    )
}

pub fn download_package(package_url: String, package_archive: String) -> Result<(), String> {
    let handle = Handle::current().clone();
    let th = std::thread::spawn(move || {
        handle
            .block_on(download_file(
                package_url.to_string(),
                package_archive.to_string(),
            ))
            .unwrap();
    });
    Ok(th.join().unwrap())
}

async fn download_file(url: String, output: String) -> Result<(), String> {
    if Path::new(&output).exists() {
        println!("Using cached archive: {}", output);
        return Ok(());
    }
    println!("Downloading {} to {}", url, output);
    fetch_url(url, output).await
}

async fn fetch_url(url: String, output: String) -> Result<(), String> {
    let response = reqwest::get(&url).await;
    match response {
        Ok(r) => {
            let mut file = std::fs::File::create(output).unwrap();
            let mut content = Cursor::new(r.bytes().await.unwrap());
            std::io::copy(&mut content, &mut file).unwrap();

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
