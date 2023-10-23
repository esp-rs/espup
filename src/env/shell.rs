// THIS FILE IS BASED ON RUSTUP SOURCE CODE: https://github.com/rust-lang/rustup/blob/b900a6cd87e1f463a55ce02e956c24b2cccdd0f0/src/cli/self_update/shell.rs

//! Paths and Unix shells
//!
//! MacOS, Linux, FreeBSD, and many other OS model their design on Unix,
//! so handling them is relatively consistent. But only relatively.
//! POSIX postdates Unix by 20 years, and each "Unix-like" shell develops
//! unique quirks over time.
//!
//!
//! Windowing Managers, Desktop Environments, GUI Terminals, and PATHs
//!
//! Duplicating paths in PATH can cause performance issues when the OS searches
//! the same place multiple times. Traditionally, Unix configurations have
//! resolved this by setting up PATHs in the shell's login profile.
//!
//! This has its own issues. Login profiles are only intended to run once, but
//! changing the PATH is common enough that people may run it twice. Desktop
//! environments often choose to NOT start login shells in GUI terminals. Thus,
//! a trend has emerged to place PATH updates in other run-commands (rc) files,
//! leaving Rustup with few assumptions to build on for fulfilling its promise
//! to set up PATH appropriately.
//!
//! Rustup addresses this by:
//! 1) using a shell script that updates PATH if the path is not in PATH
//! 2) sourcing this script (`. /path/to/script`) in any appropriate rc file

#[cfg(unix)]
use crate::env::get_home_dir;
use crate::error::Error;
use miette::Result;
use std::{
    env,
    fs::OpenOptions,
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
pub(super) type Shell = Box<dyn UnixShell>;
#[cfg(windows)]
pub(super) type Shell = Box<dyn WindowsShell>;

#[derive(Debug, PartialEq)]
pub struct ShellScript {
    content: &'static str,
    name: &'static str,
    toolchain_dir: PathBuf,
}

impl ShellScript {
    pub(crate) fn write(&self) -> Result<(), Error> {
        let env_file_path = self.toolchain_dir.join(self.name);
        let mut env_file: String = self.content.to_string();

        let xtensa_gcc = env::var("XTENSA_GCC").unwrap_or_default();
        env_file = env_file.replace("{xtensa_gcc}", &xtensa_gcc);

        let riscv_gcc = env::var("RISCV_GCC").unwrap_or_default();
        env_file = env_file.replace("{riscv_gcc}", &riscv_gcc);

        let libclang_path = env::var("LIBCLANG_PATH").unwrap_or_default();
        env_file = env_file.replace("{libclang_path}", &libclang_path);
        #[cfg(windows)]
        if cfg!(windows) {
            let libclang_bin_path = env::var("LIBCLANG_BIN_PATH").unwrap_or_default();
            env_file = env_file.replace("{libclang_bin_path}", &libclang_bin_path);
        }

        let clang_path = env::var("CLANG_PATH").unwrap_or_default();
        env_file = env_file.replace("{clang_path}", &clang_path);

        write_file(&env_file_path, &env_file)?;
        Ok(())
    }
}

#[cfg(unix)]
/// Cross-platform non-POSIX shells have not been assessed for integration yet
fn enumerate_shells() -> Vec<Shell> {
    vec![
        Box::new(Posix),
        Box::new(Bash),
        Box::new(Zsh),
        Box::new(Fish),
    ]
}

#[cfg(unix)]
/// Returns all shells that exist on the system.
pub(super) fn get_available_shells() -> impl Iterator<Item = Shell> {
    enumerate_shells().into_iter().filter(|sh| sh.does_exist())
}

#[cfg(windows)]
pub trait WindowsShell {
    /// Writes the relevant env file.
    fn env_script(&self, toolchain_dir: &Path) -> ShellScript;

    /// Gives the source string for a given shell.
    fn source_string(&self, toolchain_dir: &str) -> Result<String, Error>;
}

#[cfg(windows)]
pub struct Batch;
#[cfg(windows)]
impl WindowsShell for Batch {
    fn env_script(&self, toolchain_dir: &Path) -> ShellScript {
        ShellScript {
            name: "env.bat",
            content: include_str!("env.bat"),
            toolchain_dir: toolchain_dir.to_path_buf(),
        }
    }

    fn source_string(&self, toolchain_dir: &str) -> Result<String, Error> {
        Ok(format!(r#"{}/env.bat""#, toolchain_dir))
    }
}

#[cfg(windows)]
pub struct Powershell;
#[cfg(windows)]
impl WindowsShell for Powershell {
    fn env_script(&self, toolchain_dir: &Path) -> ShellScript {
        ShellScript {
            name: "env.ps1",
            content: include_str!("env.ps1"),
            toolchain_dir: toolchain_dir.to_path_buf(),
        }
    }

    fn source_string(&self, toolchain_dir: &str) -> Result<String, Error> {
        Ok(format!(r#". "{}/env.ps1""#, toolchain_dir))
    }
}

#[cfg(unix)]
pub trait UnixShell {
    /// Detects if a shell "exists". Users have multiple shells, so an "eager"
    /// heuristic should be used, assuming shells exist if any traces do.
    fn does_exist(&self) -> bool;

    /// Gives all rcfiles of a given shell that Rustup is concerned with.
    /// Used primarily in checking rcfiles for cleanup.
    fn rcfiles(&self) -> Vec<PathBuf>;

    /// Gives rcs that should be written to.
    fn update_rcs(&self) -> Vec<PathBuf>;

    /// Writes the relevant env file.
    fn env_script(&self, toolchain_dir: &Path) -> ShellScript {
        ShellScript {
            name: "env",
            content: include_str!("env.sh"),
            toolchain_dir: toolchain_dir.to_path_buf(),
        }
    }

    /// Gives the source string for a given shell.
    fn source_string(&self, toolchain_dir: &str) -> Result<String, Error> {
        Ok(format!(r#". "{}/env""#, toolchain_dir))
    }
}

#[cfg(unix)]
struct Posix;
#[cfg(unix)]
impl UnixShell for Posix {
    fn does_exist(&self) -> bool {
        true
    }

    fn rcfiles(&self) -> Vec<PathBuf> {
        vec![get_home_dir().join(".profile")]
    }

    fn update_rcs(&self) -> Vec<PathBuf> {
        // Write to .profile even if it doesn't exist. It's the only rc in the
        // POSIX spec so it should always be set up.
        self.rcfiles()
    }
}

#[cfg(unix)]
struct Bash;
#[cfg(unix)]
impl UnixShell for Bash {
    fn does_exist(&self) -> bool {
        !self.update_rcs().is_empty()
    }

    fn rcfiles(&self) -> Vec<PathBuf> {
        // Bash also may read .profile, however Rustup already includes handling
        // .profile as part of POSIX and always does setup for POSIX shells.
        [".bash_profile", ".bash_login", ".bashrc"]
            .iter()
            .map(|rc| get_home_dir().join(rc))
            .collect()
    }

    fn update_rcs(&self) -> Vec<PathBuf> {
        self.rcfiles()
            .into_iter()
            .filter(|rc| rc.is_file())
            .collect()
    }
}

#[cfg(unix)]
struct Zsh;
#[cfg(unix)]
impl Zsh {
    fn zdotdir() -> Result<PathBuf, Error> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        if matches!(env::var("SHELL"), Ok(sh) if sh.contains("zsh")) {
            match env::var("ZDOTDIR") {
                Ok(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
                _ => Err(Error::Zdotdir),
            }
        } else {
            match std::process::Command::new("zsh")
                .args(["-c", "'echo $ZDOTDIR'"])
                .output()
            {
                Ok(io) if !io.stdout.is_empty() => Ok(PathBuf::from(OsStr::from_bytes(&io.stdout))),
                _ => Err(Error::Zdotdir),
            }
        }
    }
}

#[cfg(unix)]
impl UnixShell for Zsh {
    fn does_exist(&self) -> bool {
        // zsh has to either be the shell or be callable for zsh setup.
        matches!(env::var("SHELL"), Ok(sh) if sh.contains("zsh")) || find_cmd(&["zsh"]).is_some()
    }

    fn rcfiles(&self) -> Vec<PathBuf> {
        let home_dir: Option<PathBuf> = Some(get_home_dir());
        [Zsh::zdotdir().ok(), home_dir]
            .iter()
            .filter_map(|dir| dir.as_ref().map(|p| p.join(".zshenv")))
            .collect()
    }

    fn update_rcs(&self) -> Vec<PathBuf> {
        // zsh can change $ZDOTDIR both _before_ AND _during_ reading .zshenv,
        // so we: write to $ZDOTDIR/.zshenv if-exists ($ZDOTDIR changes before)
        // OR write to $HOME/.zshenv if it exists (change-during)
        // if neither exist, we create it ourselves, but using the same logic,
        // because we must still respond to whether $ZDOTDIR is set or unset.
        // In any case we only write once.
        self.rcfiles()
            .into_iter()
            .filter(|env| env.is_file())
            .chain(self.rcfiles())
            .take(1)
            .collect()
    }
}

#[cfg(unix)]
struct Fish;
#[cfg(unix)]
impl UnixShell for Fish {
    fn does_exist(&self) -> bool {
        // fish has to either be the shell or be callable for fish setup.
        matches!(env::var("SHELL"), Ok(sh) if sh.contains("fish")) || find_cmd(&["fish"]).is_some()
    }

    // > "$XDG_CONFIG_HOME/fish/conf.d" (or "~/.config/fish/conf.d" if that variable is unset) for the user
    // from <https://github.com/fish-shell/fish-shell/issues/3170#issuecomment-228311857>
    fn rcfiles(&self) -> Vec<PathBuf> {
        let p0 = env::var("XDG_CONFIG_HOME").ok().map(|p| {
            let mut path = PathBuf::from(p);
            path.push("fish/conf.d/espup.fish");
            path
        });

        let p1 = get_home_dir().join(".config/fish/conf.d/espup.fish");

        p0.into_iter().chain(Some(p1)).collect()
    }

    fn update_rcs(&self) -> Vec<PathBuf> {
        self.rcfiles()
    }

    fn env_script(&self, toolchain_dir: &Path) -> ShellScript {
        ShellScript {
            name: "env.fish",
            content: include_str!("env.fish"),
            toolchain_dir: toolchain_dir.to_path_buf(),
        }
    }

    fn source_string(&self, toolchain_dir: &str) -> Result<String, Error> {
        Ok(format!(r#". "{}/env.fish""#, toolchain_dir))
    }
}

#[cfg(unix)]
/// Finds the command for a given string.
pub(crate) fn find_cmd<'a>(cmds: &[&'a str]) -> Option<&'a str> {
    cmds.iter().cloned().find(|&s| has_cmd(s))
}

#[cfg(unix)]
/// Checks if a command exists in the PATH.
fn has_cmd(cmd: &str) -> bool {
    let cmd = format!("{}{}", cmd, env::consts::EXE_SUFFIX);
    let path = env::var("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|p| p.join(&cmd))
        .any(|p| p.exists())
}

/// Writes a file to a given path.
pub fn write_file(path: &Path, contents: &str) -> Result<(), Error> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    Write::write_all(&mut file, contents.as_bytes())?;

    file.sync_data()?;

    Ok(())
}
