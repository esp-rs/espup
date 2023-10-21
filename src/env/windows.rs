use crate::{env::get_home_dir, error::Error};
use miette::Result;
use std::{
    env,
    fs::{remove_file, Path},
};
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

const LEGACY_EXPORT_FILE: &str = "export-esp.ps1";

// Clean the environment for Windows.
pub(super) fn clean_env(toolchain_dir: &Path) -> Result<(), Error> {
    delete_env_variable("LIBCLANG_PATH")?;
    delete_env_variable("CLANG_PATH")?;
    if let Some(path) = env::var("PATH") {
        set_env_variable("PATH", &path)?;
    };

    remove_legacy_export_file()?;

    Ok(())
}

/// Deletes an environment variable for the current user.
fn delete_env_variable(key: &str) -> Result<(), Error> {
    if env::var(key).is_none() {
        return Ok(());
    }

    env::remove_var(key);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    environment_key.delete_value(key)?;
    Ok(())
}

/// Sets an environment variable for the current user.
fn set_env_variable(key: &str, value: &str) -> Result<(), Error> {
    env::set_var(key, value);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_WRITE)?;
    environment_key.set_value(key, &value)?;
    Ok(())
}

// Delete the legacy export file.
fn remove_legacy_export_file() -> Result<(), Error> {
    let legacy_file = get_home_dir().join(LEGACY_EXPORT_FILE);
    if legacy_file.exists() {
        remove_file(&legacy_file)?;
    }

    Ok(())
}

// Update the environment for Windows.
pub(super) fn update_env(toolchain_dir: &Path) -> Result<(), Error> {
    let mut path = env::var("PATH").unwrap_or_default();

    // TODO: FIX THIS
    if let Some(xtensa_gcc) = env::var("XTENSA_GCC") {
        if !path.contains(xtensa_gcc) {
            path = format!("{};{}", xtensa_gcc, path);
        }
    }

    if let Some(riscv_gcc) = env::var("RISCV_GCC") {
        if !path.contains(riscv_gcc) {
            path = format!("{};{}", riscv_gcc, path);
        }
    }

    if let Some(libclang_path) = env::var("LIBCLANG_PATH") {
        set_env_variable("LIBCLANG_PATH", libclang_path)?;
    }

    if let Some(clang_path) = env::var("CLANG_PATH") {
        if !path.contains(clang_path) {
            path = format!("{};{}", clang_path, path);
        }
    }
    set_env_variable("PATH", &path)?;

    remove_legacy_export_file()?;

    Ok(())
}

// Write the environment files for Windows.
pub(super) fn write_env_files(toolchain_dir: &Path) -> Result<(), Error> {
    let windows_shells: Vec<shell::Shell> = vec![Box::new(Batch), Box::new(Powershell)];
    for sh in windows_shells.into_iter() {
        let script = sh.env_script(toolchain_dir);
        script.write()?;
    }

    Ok(())
}
