use std::process::Stdio;
use espflash::Chip;
use crate::utils::*;
use std::path::Path;
pub fn check_rust_installation(nightly_version: &str) {
    match std::process::Command::new("rustup")
        .args(["toolchain", "list"])
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            println!("rustup found.");
            let result = String::from_utf8_lossy(&child_output.stdout);
            if !result.contains(nightly_version) {
                println!("nightly toolchain not found");
                install_rust_nightly(nightly_version);
            } else {
                println!("nightly toolchain found.");
            }
        }
        Err(e) => {
            println!("Error: {}", e);
            install_rustup();
        }
    }
}

pub fn install_riscv_target(version: &str){

    match std::process::Command::new("rustup")
        .arg("component")
        .arg("add")
        .arg("rust-src")
        .arg("--toolchain")
        .arg(version)
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            let result = String::from_utf8_lossy(&child_output.stdout);
            println!("Rust-src for RiscV target installed suscesfully: {}", result);
        }
        Err(e) => {
            println!("Rust-src for RiscV target installation failed: {}", e);
        }
    }

    match std::process::Command::new("rustup")
        .arg("target")
        .arg("add")
        .arg("--toolchain")
        .arg(version)
        .arg("riscv32imc-unknown-none-elf")
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            let result = String::from_utf8_lossy(&child_output.stdout);
            println!("RiscV target installed suscesfully: {}", result);
        }
        Err(e) => {
            println!("RiscV target installation failed: {}", e);
        }
    }
}

pub fn install_rustup() {
    #[cfg(windows)]
    let rustup_init_path =
        prepare_single_binary("https://win.rustup.rs/x86_64", "rustup-init.exe", "rustup");
    #[cfg(unix)]
    let rustup_init_path = prepare_single_binary("https://sh.rustup.rs/", "rustup-init", "rustup");
    println!("rustup stable");
    match std::process::Command::new(rustup_init_path)
        .arg("--default-toolchain")
        .arg("none")
        .arg("--profile")
        .arg("minimal")
        .arg("-y")
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            let result = String::from_utf8_lossy(&child_output.stdout);
            println!("{}", result);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}


pub fn install_rust_nightly(version: &str) {
    println!("installing nightly toolchain");
    match std::process::Command::new("rustup")
        .arg("toolchain")
        .arg("install")
        .arg(version)
        .arg("--profile")
        .arg("minimal")
        .stdout(Stdio::piped())
        .output()
    {
        Ok(child_output) => {
            let result = String::from_utf8_lossy(&child_output.stdout);
            println!("Result: {}", result);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }
}

pub fn install_gcc_targets(targets: Vec<Chip>) -> Result<Vec<String>, String> {
    let mut exports: Vec<String> = Vec::new();
    for target in targets {
        match target {
            Chip::Esp32 => {
                install_gcc("xtensa-esp32-elf");
                exports.push(format!(
                    "export PATH={}:$PATH",
                    get_tool_path("xtensa-esp32-elf/bin")
                ));
            }
            Chip::Esp32s2 => {
                install_gcc("xtensa-esp32s2-elf");
                exports.push(format!(
                    "export PATH={}:$PATH",
                    get_tool_path("xtensa-esp32s2-elf/bin")
                ));
            }
            Chip::Esp32s3 => {
                install_gcc("xtensa-esp32s3-elf");
                exports.push(format!(
                    "export PATH={}:$PATH",
                    get_tool_path("xtensa-esp32s3-elf/bin")
                ));
            }
            Chip::Esp32c3 => {
                install_gcc("riscv32-esp-elf");
                exports.push(format!(
                    "export PATH={}:$PATH",
                    get_tool_path("riscv32-esp-elf/bin")
                ));
            }
            _ => {
                println!("Unknown target")
            }
        }
    }
    Ok(exports)
}

pub fn install_gcc(gcc_target: &str) {
    let gcc_path = get_tool_path(gcc_target);
    println!("gcc path: {}", gcc_path);
    // if Path::new(&gcc_path).exists() {
    //     println!("Previous installation of GCC for target: {}", gcc_path);
    //     // return Ok(());
    // } else {
    // fs::create_dir_all(&gcc_path).unwrap();
    let gcc_file = format!(
        "{}-gcc8_4_0-esp-2021r2-patch3-{}.tar.gz",
        gcc_target,
        get_gcc_arch(guess_host_triple::guess_host_triple().unwrap())
    );
    let gcc_dist_url = format!(
        "https://github.com/espressif/crosstool-NG/releases/download/esp-2021r2-patch3/{}",
        gcc_file
    );
    match prepare_package_strip_prefix(&gcc_dist_url, gcc_path, "") {
        Ok(_) => {
            println!("Package {} ready", gcc_file);
        }
        Err(_e) => {
            println!("Unable to prepare {}", gcc_file);
        }
    }
    // }
}


pub fn install_espidf(targets: &str, version: String) -> Result<(), String> {
    let espidf_path = format!("{}/frameworks/esp-idf", get_espressif_base_path());
    println!("ESP-IDF Path: {}", espidf_path);

    #[cfg(windows)]
    match prepare_package(
        "https://dl.espressif.com/dl/idf-git/idf-git-2.30.1-win64.zip".to_string(),
        get_dist_path("idf-git-2.30.1-win64.zip").as_str(),
        get_tool_path("idf-git/2.30.1".to_string()),
    ) {
        Ok(_) => {
            println!("Ok");
        }
        Err(_e) => {
            println!("Failed");
        }
    }
    #[cfg(windows)]
    match prepare_package(
        "https://dl.espressif.com/dl/idf-python/idf-python-3.8.7-embed-win64.zip".to_string(),
        get_dist_path("idf-python-3.8.7-embed-win64.zip").as_str(),
        get_tool_path("idf-python/3.8.7".to_string()),
    ) {
        Ok(_) => {
            println!("Ok");
        }
        Err(_e) => {
            println!("Failed");
        }
    }

    #[cfg(windows)]
    let git_path = get_tool_path("idf-git/2.30.1/cmd/git.exe".to_string());
    #[cfg(unix)]
    let git_path = "/usr/bin/git".to_string();

    // TODO: See if needed
    // update_property("gitPath".to_string(), git_path.clone());

    #[cfg(windows)]
    let python_path = get_tool_path("idf-python/3.8.7/python.exe".to_string());
    #[cfg(unix)]
    let python_path = "/usr/bin/python3".to_string();

    let virtual_env_path = get_python_env_path("4.4", "3.8");
    // TODO: Use any git crate?
    if !Path::new(&espidf_path).exists() {
        // let clone_command = format!("git clone --shallow-since=2020-01-01 --jobs 8 --recursive git@github.com:espressif/esp-idf.git ");
        let mut arguments: Vec<String> = [].to_vec();
        arguments.push("clone".to_string());
        arguments.push("--jobs".to_string());
        arguments.push("8".to_string());
        arguments.push("--branch".to_string());
        arguments.push(version);
        arguments.push("--depth".to_string());
        arguments.push("1".to_string());
        arguments.push("--shallow-submodules".to_string());
        arguments.push("--recursive".to_string());
        arguments.push("https://github.com/espressif/esp-idf.git".to_string());
        // arguments.push("git@github.com:espressif/esp-idf.git".to_string());
        arguments.push(espidf_path.clone());
        println!("Cloning: {} {:?}", git_path, arguments);
        match run_command(git_path, arguments, "".to_string()) {
            Ok(_) => {
                println!("Cloned esp-idf suscessfuly");
            }
            Err(_e) => {
                println!("Cloned esp-idf failed");
            }
        }
    }
    println!("Installing esp-idf for {} with {}/install.sh", targets, espidf_path);
    let install_script_path = format!("{}/install.sh", espidf_path);
    let mut arguments: Vec<String> = [].to_vec();
    arguments.push(targets.to_string());
    match run_command(install_script_path, arguments, "".to_string()) {
        Ok(_) => {
            println!("ESP-IDF installation succeeded");
        }
        Err(_e) => {
            println!("ESP-IDF installation failed");
        }
    }
    // match std::process::Command::new(install_script_path)
    //     .arg("esp32 esp32s2")
    //     .stdout(Stdio::piped())
    //     .output()
    // {
    //     Ok(child_output) => {
    //         let result = String::from_utf8_lossy(&child_output.stdout);
    //         println!("ESP-IDF installation succeeded: {}", result);
    //     }
    //     Err(e) => {
    //         println!("ESP-IDF installation failed: {}", e);
    //     }
    // }

    println!("Installing CMake");
    let mut arguments: Vec<String> = [].to_vec();
    let mut idf_tools_scritp_path = format!("{}/tools/idf_tools.py", espidf_path);
    arguments.push(idf_tools_scritp_path);
    arguments.push("install".to_string());
    arguments.push("cmake".to_string());
    match run_command(python_path, arguments, "".to_string()) {
        Ok(_) => {
            println!("CMake installation succeeded");
        }
        Err(_e) => {
            println!("CMake installation failed");
        }
    }

    Ok(())
}
