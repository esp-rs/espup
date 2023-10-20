use crate::env::shell;
use crate::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

// // TODO: USED FOR UNINSTALING
pub fn do_remove_from_path(toolchain_dir: &Path) -> Result<(), Error> {
    for sh in shell::get_available_shells() {
        let source_bytes = format!(
            "{}\n",
            sh.source_string(&toolchain_dir.display().to_string())?
        )
        .into_bytes();

        // Check more files for cleanup than normally are updated.
        for rc in sh.rcfiles().iter().filter(|rc| rc.is_file()) {
            let file = std::fs::read_to_string(rc).map_err(|_| Error::ReadingFile {
                name: "rcfile",
                path: PathBuf::from(&rc),
            })?;
            let file_bytes = file.into_bytes();
            // FIXME: This is whitespace sensitive where it should not be.
            if let Some(idx) = file_bytes
                .windows(source_bytes.len())
                .position(|w| w == source_bytes.as_slice())
            {
                // Here we rewrite the file without the offending line.
                let mut new_bytes = file_bytes[..idx].to_vec();
                new_bytes.extend(&file_bytes[idx + source_bytes.len()..]);
                let new_file = String::from_utf8(new_bytes).unwrap();
                let mut file = OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(rc)?;
                Write::write_all(&mut file, new_file.as_bytes())?;

                file.sync_data()?;
            }
        }
    }

    Ok(())
}

pub(crate) fn do_add_to_path(toolchain_dir: &Path) -> Result<(), Error> {
    for sh in shell::get_available_shells() {
        let source_cmd = sh.source_string(&toolchain_dir.display().to_string())?;
        let source_cmd_with_newline = format!("\n{}", &source_cmd);

        for rc in sh.update_rcs() {
            let file = std::fs::read_to_string(&rc).map_err(|_| Error::ReadingFile {
                name: "rcfile",
                path: PathBuf::from(&rc),
            });
            let cmd_to_write: &str = match file {
                Ok(contents) if contents.contains(&source_cmd) => continue,
                Ok(contents) if !contents.ends_with('\n') => &source_cmd_with_newline,
                _ => &source_cmd,
            };

            let mut dest_file = OpenOptions::new()
                .write(true)
                .append(true)
                .create(true)
                .open(&rc)?;

            writeln!(dest_file, "{cmd_to_write}")?;

            dest_file.sync_data()?;
        }
    }

    Ok(())
}

pub(crate) fn do_write_env_files(toolchain_dir: &Path) -> Result<(), Error> {
    let mut written = vec![];

    for sh in shell::get_available_shells() {
        let script = sh.env_script(toolchain_dir);
        // Only write each possible script once.
        if !written.contains(&script) {
            script.write()?;
            written.push(script);
        }
    }

    Ok(())
}
