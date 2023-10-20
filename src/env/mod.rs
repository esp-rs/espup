//! Environment variables set up and export file support.

use crate::error::Error;
use log::info;
use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
};

pub mod shell;
#[cfg(unix)]
pub mod unix;
#[cfg(windows)]
pub mod windows;

#[cfg(windows)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.ps1";
#[cfg(unix)]
const DEFAULT_EXPORT_FILE: &str = "export-esp.sh";

/// Returns the absolute path to the export file, uses the DEFAULT_EXPORT_FILE if no arg is provided.
pub fn get_export_file(
    export_file: Option<PathBuf>,
    toolchain_dir: &Path,
) -> Result<PathBuf, Error> {
    if let Some(export_file) = export_file {
        if export_file.is_dir() {
            return Err(Error::InvalidDestination(export_file.display().to_string()));
        }
        if export_file.is_absolute() {
            Ok(export_file)
        } else {
            let current_dir = std::env::current_dir()?;
            Ok(current_dir.join(export_file))
        }
    } else {
        Ok(toolchain_dir.join(DEFAULT_EXPORT_FILE))
    }
}

/// Creates the export file with the necessary environment variables.
pub fn create_export_file(export_file: &PathBuf, exports: &[String]) -> Result<(), Error> {
    info!("Creating export file");
    let mut file = File::create(export_file)?;
    for e in exports.iter() {
        #[cfg(windows)]
        let e = e.replace('/', r"\");
        file.write_all(e.as_bytes())?;
        file.write_all(b"\n")?;
    }

    Ok(())
}

/// Instructions to export the environment variables.
pub fn export_environment(export_file: &Path, toolchain_dir: &Path) -> Result<(), Error> {
    #[cfg(windows)]
    if cfg!(windows) {
        set_environment_variable("PATH", &env::var("PATH").unwrap())?;
        warn!(
            "Your environments variables have been updated! Shell may need to be restarted for changes to be effective"
        );
        warn!(
            "A file was created at '{}' showing the injected environment variables",
            export_file.display()
        );
    }
    #[cfg(unix)]
    if cfg!(unix) {
        let source_command = format!(". {}", export_file.display());

        // Check if the GCC_RISCV environment variable is set
        unix::do_write_env_files(toolchain_dir)?;
        unix::do_add_to_path(toolchain_dir)?;
        println!(
            "\n\tTo get started, you need to set up some environment variables by running: '{}'",
            source_command
        );
        println!(
            "\tThis step must be done every time you open a new terminal.\n\t    See other methods for setting the environment in https://esp-rs.github.io/book/installation/riscv-and-xtensa.html#3-set-up-the-environment-variables",
        );
    }
    Ok(())
}

// #[cfg(test)]
// mod tests {
//     use crate::env::{create_export_file, get_export_file, DEFAULT_EXPORT_FILE};
//     use directories::BaseDirs;
//     use std::{env::current_dir, path::PathBuf};

//     #[test]
//     #[allow(unused_variables)]
//     fn test_get_export_file() {
//         // No arg provided
//         let home_dir = BaseDirs::new().unwrap().home_dir().to_path_buf();
//         let export_file = home_dir.join(DEFAULT_EXPORT_FILE);
//         assert!(matches!(get_export_file(None), Ok(export_file)));
//         // Relative path
//         let current_dir = current_dir().unwrap();
//         let export_file = current_dir.join("export.sh");
//         assert!(matches!(
//             get_export_file(Some(PathBuf::from("export.sh"))),
//             Ok(export_file)
//         ));
//         // Absolute path
//         let export_file = PathBuf::from("/home/user/export.sh");
//         assert!(matches!(
//             get_export_file(Some(PathBuf::from("/home/user/export.sh"))),
//             Ok(export_file)
//         ));
//         // Path is a directory instead of a file
//         assert!(get_export_file(Some(home_dir)).is_err());
//     }

//     #[test]
//     fn test_create_export_file() {
//         // Creates the export file and writes the correct content to it
//         let temp_dir = tempfile::TempDir::new().unwrap();
//         let export_file = temp_dir.path().join("export.sh");
//         let exports = vec![
//             "export VAR1=value1".to_string(),
//             "export VAR2=value2".to_string(),
//         ];
//         create_export_file(&export_file, &exports).unwrap();
//         let contents = std::fs::read_to_string(export_file).unwrap();
//         assert_eq!(contents, "export VAR1=value1\nexport VAR2=value2\n");

//         // Returns the correct error when it fails to create the export file (it already exists)
//         let temp_dir = tempfile::TempDir::new().unwrap();
//         let export_file = temp_dir.path().join("export.sh");
//         std::fs::create_dir_all(&export_file).unwrap();
//         let exports = vec![
//             "export VAR1=value1".to_string(),
//             "export VAR2=value2".to_string(),
//         ];
//         assert!(create_export_file(&export_file, &exports).is_err());
//     }
// }