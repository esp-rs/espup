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
    RegKey,
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
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
        HWND_BROADCAST, SMTO_ABORTIFHUNG, SendMessageTimeoutA, WM_SETTINGCHANGE,
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
            c"Environment".as_ptr() as LPARAM,
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

    unsafe {
        env::remove_var(key);
    }

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
/// Persists toolchain environment variables for the current Windows user.
pub fn set_env() -> Result<(), Error> {
    persist_windows_env(
        get_windows_path_var()?,
        |key| env::var(key).ok(),
        set_env_variable,
    )
}

// Keep registry writes injectable so persistence can be tested without changing the host.
#[cfg(any(windows, test))]
fn persist_windows_env(
    mut path: String,
    get_variable: impl Fn(&str) -> Option<String>,
    mut set_variable: impl FnMut(&str, &str) -> Result<(), Error>,
) -> Result<(), Error> {
    if let Some(xtensa_gcc) = get_variable("XTENSA_GCC") {
        let xtensa_gcc: &str = &xtensa_gcc;
        if !path.contains(xtensa_gcc) {
            path = format!("{xtensa_gcc};{path}");
        }
    }

    if let Some(riscv_gcc) = get_variable("RISCV_GCC") {
        let riscv_gcc: &str = &riscv_gcc;
        if !path.contains(riscv_gcc) {
            path = format!("{riscv_gcc};{path}");
        }
    }

    if let Some(libclang_path) = get_variable("LIBCLANG_PATH") {
        set_variable("LIBCLANG_PATH", &libclang_path)?;
    }

    if let Some(libclang_bin_path) = get_variable("LIBCLANG_BIN_PATH") {
        let libclang_bin_path: &str = &libclang_bin_path;
        if !path.contains(libclang_bin_path) {
            path = format!("{libclang_bin_path};{path}");
        }
    }

    if let Some(clang_path) = get_variable("CLANG_PATH") {
        set_variable("CLANG_PATH", &clang_path)?;
    }

    set_variable("PATH", &path)?;
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
        println!(
            "\tIf you get still get errors, try manually adding the environment variables by running '{}'",
            export_file.display()
        );
        println!("\tDiagnose anytime with: espup doctor");
    }
    #[cfg(unix)]
    if cfg!(unix) {
        println!(
            "\n\tTo get started, you need to set up some environment variables by running: '. {}'",
            export_file.display()
        );
        println!(
            "\tThis step must be done every time you open a new terminal."
        );
        println!(
            "\tTip: eval once per shell with:  eval \"$(espup env)\""
        );
        println!(
            "\tOr add to ~/.zshrc / ~/.bashrc:  [ -f {} ] && . {}",
            export_file.display(),
            export_file.display()
        );
        println!("\tDiagnose anytime with:  espup doctor");
        println!(
            "\tMore: https://github.com/esp-rs/espup/?tab=readme-ov-file#environment-variables-setup"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        env::{DEFAULT_EXPORT_FILE, create_export_file, get_export_file, persist_windows_env},
        error::Error,
    };
    use directories::BaseDirs;
    use std::{
        env::current_dir,
        fs::{create_dir_all, read_to_string},
        path::PathBuf,
    };
    use tempfile::TempDir;

    #[test]
    fn windows_clang_path_is_persisted_as_a_variable_not_a_path_entry() {
        let variables = std::collections::HashMap::from([
            ("CLANG_PATH", r"C:\esp\bin\clang.exe"),
            ("LIBCLANG_PATH", r"C:\esp\bin\libclang.dll"),
            ("LIBCLANG_BIN_PATH", r"C:\esp\bin"),
            ("XTENSA_GCC", r"C:\gcc\bin"),
        ]);
        let mut written = std::collections::HashMap::new();
        persist_windows_env(
            "existing".into(),
            |key| variables.get(key).map(|value| value.to_string()),
            |key, value| {
                written.insert(key.to_string(), value.to_string());
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(written["CLANG_PATH"], variables["CLANG_PATH"]);
        assert_eq!(written["LIBCLANG_PATH"], variables["LIBCLANG_PATH"]);
        assert_eq!(written["PATH"], r"C:\esp\bin;C:\gcc\bin;existing");
        assert!(!written["PATH"].contains("clang.exe"));
    }

    #[test]
    fn missing_windows_variables_leave_path_unchanged() {
        let mut written = Vec::new();
        persist_windows_env(
            "existing".into(),
            |_| None,
            |key, value| {
                written.push((key.to_string(), value.to_string()));
                Ok(())
            },
        )
        .unwrap();
        assert_eq!(written, [("PATH".into(), "existing".into())]);
    }

    #[test]
    fn windows_variable_write_errors_are_propagated() {
        let result = persist_windows_env(
            "existing".into(),
            |key| (key == "CLANG_PATH").then(|| "clang.exe".into()),
            |key, _| {
                assert_eq!(key, "CLANG_PATH");
                Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied).into())
            },
        );
        assert!(
            matches!(result, Err(Error::IoError(err)) if err.kind() == std::io::ErrorKind::PermissionDenied)
        );
    }

    #[test]
    fn test_get_export_file() {
        // No arg provided
        let home_dir = BaseDirs::new().unwrap().home_dir().to_path_buf();
        assert_eq!(
            get_export_file(None).unwrap(),
            home_dir.join(DEFAULT_EXPORT_FILE)
        );

        // Relative path
        assert_eq!(
            get_export_file(Some(PathBuf::from("export.sh"))).unwrap(),
            current_dir().unwrap().join("export.sh")
        );

        // Absolute path
        let absolute = if cfg!(windows) {
            PathBuf::from(r"C:\home\user\export.ps1")
        } else {
            PathBuf::from("/home/user/export.sh")
        };
        assert_eq!(get_export_file(Some(absolute.clone())).unwrap(), absolute);

        // Path is a directory instead of a file
        assert!(matches!(
            get_export_file(Some(home_dir)),
            Err(Error::InvalidDestination(_))
        ));
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
