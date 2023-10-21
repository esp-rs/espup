use log::warn;
use std::env;
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

const LEGACY_EXPORT_FILE: &str = "export-esp.ps1";

// TODO: REVIEW CFGS
#[cfg(windows)]
/// Sets an environment variable for the current user.
pub fn set_environment_variable(key: &str, value: &str) -> Result<(), Error> {
    env::set_var(key, value);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_WRITE)?;
    environment_key.set_value(key, &value)?;
    Ok(())
}

#[cfg(windows)]
/// Deletes an environment variable for the current user.
pub fn delete_environment_variable(key: &str) -> Result<(), Error> {
    if env::var_os(key).is_none() {
        return Ok(());
    }

    env::remove_var(key);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    environment_key.delete_value(key)?;
    Ok(())
}

pub(crate) fn do_write_env_files(toolchain_dir: &Path) -> Result<(), Error> {
    let windows_shells: Vec<shell::Shell> = vec![Box::new(Batch), Box::new(Powershell)];
    for sh in windows_shells.into_iter() {
        let script = sh.env_script(toolchain_dir);
        script.write()?;
    }

    Ok(())
}

pub(crate) fn do_add_to_path(toolchain_dir: &Path) -> Result<(), Error> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    set_environment_variable("PATH", path)?;

    let xtensa_gcc = std::env::var_os("XTENSA_GCC").unwrap_or_default();
    set_environment_variable("XTENSA_GCC", xtensa_gcc)?;

    let riscv_gcc = std::env::var_os("RISCV_GCC").unwrap_or_default();
    set_environment_variable("RISCV_GCC", riscv_gcc)?;

    let libclang_path = std::env::var_os("LIBCLANG_PATH");
    if let Some(libclang_path) = libclang_path {
        set_environment_variable("LIBCLANG_PATH", libclang_path)?;
    }

    let clang_path = std::env::var_os("CLANG_PATH");
    if let Some(libclang_path) = clang_path {
        set_environment_variable("CLANG_PATH", clang_path)?;
    }

    remove_legacy_export_file()?;

    Ok(())
}

fn remove_legacy_export_file() -> Result<(), Error> {
    let legacy_file = get_home_dir().join(LEGACY_EXPORT_FILE);
    if legacy_file.exists() {
        remove_file(&legacy_file)?;
    }

    Ok(())
}
// TODO: REMOVE LEGACY FILE
