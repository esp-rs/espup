//! Environment variables set up and export file support.

use crate::error::Error;
use directories::BaseDirs;
use log::debug;
use std::{
    env,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};
#[cfg(windows)]
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

#[cfg(windows)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.ps1";
#[cfg(not(windows))]
const DEFAULT_EXPORT_FILE: &str = "export-esp.sh";

#[cfg(windows)]
/// Sets an environment variable for the current user.
pub fn set_env_variable(key: &str, value: &str) -> Result<(), Error> {
    use std::ptr;
    use winapi::shared::minwindef::*;
    use winapi::um::winuser::{
        SendMessageTimeoutA, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
    };

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_WRITE)?;
    environment_key.set_value(key, &value)?;

    // Tell other processes to update their environment
    #[allow(clippy::unnecessary_cast)]
    unsafe {
        SendMessageTimeoutA(
            HWND_BROADCAST,
            WM_SETTINGCHANGE,
            0 as WPARAM,
            "Environment\0".as_ptr() as LPARAM,
            SMTO_ABORTIFHUNG,
            5000,
            ptr::null_mut(),
        );
    }

    Ok(())
}

#[cfg(windows)]
/// Deletes an environment variable for the current user.
pub fn delete_env_variable(key: &str) -> Result<(), Error> {
    let root = RegKey::predef(HKEY_CURRENT_USER);
    let environment = root.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;

    let reg_value = environment.get_raw_value(key);
    if reg_value.is_err() {
        return Ok(());
    }

    env::remove_var(key);

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let environment_key = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)?;
    environment_key.delete_value(key)?;
    Ok(())
}

/// Returns the absolute path to the export file, uses the DEFAULT_EXPORT_FILE if no arg is provided.
pub fn get_export_file(export_file: Option<PathBuf>) -> Result<PathBuf, Error> {
    if let Some(export_file) = export_file {
        if export_file.is_dir() {
            return Err(Error::InvalidDestination(export_file.display().to_string()));
        }
        if export_file.is_absolute() {
            Ok(export_file)
        } else {
            let current_dir = env::current_dir()?;
            Ok(current_dir.join(export_file))
        }
    } else {
        Ok(BaseDirs::new()
            .unwrap()
            .home_dir()
            .join(DEFAULT_EXPORT_FILE))
    }
}

/// Creates the export file with the necessary environment variables.
pub fn create_export_file(export_file: &PathBuf, exports: &[String]) -> Result<(), Error> {
    debug!("Creating export file");
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        #[cfg(windows)]
        let e = e.replace('/', r"\");
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

#[cfg(windows)]
// Get the windows PATH variable out of the registry as a String.
pub fn get_windows_path_var() -> Result<String, Error> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu.open_subkey("Environment")?;
    let path: String = env.get_value("Path")?;
    Ok(path)
}

#[cfg(windows)]
/// Instructions to export the environment variables.
pub fn set_env() -> Result<(), Error> {
    let mut path = get_windows_path_var()?;

    if let Ok(xtensa_gcc) = env::var("XTENSA_GCC") {
        let xtensa_gcc: &str = &xtensa_gcc;
        if !path.contains(xtensa_gcc) {
            path = format!("{};{}", xtensa_gcc, path);
        }
    }

    if let Ok(riscv_gcc) = env::var("RISCV_GCC") {
        let riscv_gcc: &str = &riscv_gcc;
        if !path.contains(riscv_gcc) {
            path = format!("{};{}", riscv_gcc, path);
        }
    }

    if let Ok(libclang_path) = env::var("LIBCLANG_PATH") {
        set_env_variable("LIBCLANG_PATH", &libclang_path)?;
    }

    if let Ok(libclang_bin_path) = env::var("LIBCLANG_BIN_PATH") {
        let libclang_bin_path: &str = &libclang_bin_path;
        if !path.contains(libclang_bin_path) {
            path = format!("{};{}", libclang_bin_path, path);
        }
    }

    if let Ok(clang_path) = env::var("CLANG_PATH") {
        let clang_path: &str = &clang_path;
        if !path.contains(clang_path) {
            path = format!("{};{}", clang_path, path);
        }
    }

    set_env_variable("PATH", &path)?;
    Ok(())
}

/// Instructions to export the environment variables.
pub fn print_post_install_msg(export_file: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    if cfg!(windows) {
        println!(
            "\n\tYour environments variables have been updated! Shell may need to be restarted for changes to be effective"
        );
        println!(
            "\tA file was created at '{}' showing the injected environment variables",
            export_file.display()
        );
    }
    #[cfg(unix)]
    if cfg!(unix) {
        println!(
            "\n\tTo get started, you need to set up some environment variables by running: '. {}'",
            export_file.display()
        );
        println!(
            "\tThis step must be done every time you open a new terminal.\n\t    See other methods for setting the environment in https://esp-rs.github.io/book/installation/riscv-and-xtensa.html#3-set-up-the-environment-variables",
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::env::{create_export_file, get_export_file, DEFAULT_EXPORT_FILE};
    use directories::BaseDirs;
    use std::{
        env::current_dir,
        fs::{create_dir_all, read_to_string},
        path::PathBuf,
    };
    use tempfile::TempDir;

    #[test]
    #[allow(unused_variables)]
    fn test_get_export_file() {
        // No arg provided
        let home_dir = BaseDirs::new().unwrap().home_dir().to_path_buf();
        let export_file = home_dir.join(DEFAULT_EXPORT_FILE);
        assert!(matches!(get_export_file(None), Ok(export_file)));
        // Relative path
        let current_dir = current_dir().unwrap();
        let export_file = current_dir.join("export.sh");
        assert!(matches!(
            get_export_file(Some(PathBuf::from("export.sh"))),
            Ok(export_file)
        ));
        // Absolute path
        let export_file = PathBuf::from("/home/user/export.sh");
        assert!(matches!(
            get_export_file(Some(PathBuf::from("/home/user/export.sh"))),
            Ok(export_file)
        ));
        // Path is a directory instead of a file
        assert!(get_export_file(Some(home_dir)).is_err());
    }

    #[test]
    fn test_create_export_file() {
        // Creates the export file and writes the correct content to it
        let temp_dir = TempDir::new().unwrap();
        let export_file = temp_dir.path().join("export.sh");
        let exports = vec![
            "export VAR1=value1".to_string(),
            "export VAR2=value2".to_string(),
        ];
        create_export_file(&export_file, &exports).unwrap();
        let contents = read_to_string(export_file).unwrap();
        assert_eq!(contents, "export VAR1=value1\nexport VAR2=value2\n");

        // Returns the correct error when it fails to create the export file (it already exists)
        let temp_dir = TempDir::new().unwrap();
        let export_file = temp_dir.path().join("export.sh");
        create_dir_all(&export_file).unwrap();
        let exports = vec![
            "export VAR1=value1".to_string(),
            "export VAR2=value2".to_string(),
        ];
        assert!(create_export_file(&export_file, &exports).is_err());
    }
}
