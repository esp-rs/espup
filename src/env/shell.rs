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

use directories::BaseDirs;
use std::env;
use std::fs::OpenOptions;
use std::io::{Result, Write};
use std::path::{Path, PathBuf};

pub(crate) type Shell = Box<dyn UnixShell>;

#[derive(Debug, PartialEq)]
pub(crate) struct ShellScript {
    content: &'static str,
    name: &'static str,
    toolchain_dir: PathBuf,
}

impl ShellScript {
    pub(crate) fn write(&self) -> Result<()> {
        let env_file_path = self.toolchain_dir.join(self.name);
        let mut env_file: String = self.content.to_string();

        let xtensa_gcc = std::env::var("XTENSA_GCC").unwrap_or_default();
        env_file = env_file.replace("{xtensa_gcc}", &xtensa_gcc);

        let riscv_gcc = std::env::var("RISCV_GCC").unwrap_or_default();
        env_file = env_file.replace("{riscv_gcc}", &riscv_gcc);

        let libclang_path = std::env::var("LIBCLANG_PATH").unwrap_or_default();
        env_file = env_file.replace("{libclang_path}", &libclang_path);

        write_file(&env_file_path, &env_file)?;
        Ok(())
    }
}

// Cross-platform non-POSIX shells have not been assessed for integration yet
fn enumerate_shells() -> Vec<Shell> {
    vec![
        Box::new(Posix),
        Box::new(Bash),
        Box::new(Zsh),
        Box::new(Fish),
    ]
}

pub(crate) fn get_available_shells() -> impl Iterator<Item = Shell> {
    enumerate_shells().into_iter().filter(|sh| sh.does_exist())
}

pub(crate) trait UnixShell {
    // Detects if a shell "exists". Users have multiple shells, so an "eager"
    // heuristic should be used, assuming shells exist if any traces do.
    fn does_exist(&self) -> bool;

    // Gives all rcfiles of a given shell that Rustup is concerned with.
    // Used primarily in checking rcfiles for cleanup.
    fn rcfiles(&self) -> Vec<PathBuf>;

    // Gives rcs that should be written to.
    fn update_rcs(&self) -> Vec<PathBuf>;

    // Writes the relevant env file.
    fn env_script(&self, toolchain_dir: &Path) -> ShellScript {
        ShellScript {
            name: "env",
            content: include_str!("env.sh"),
            toolchain_dir: toolchain_dir.to_path_buf(),
        }
    }

    fn source_string(&self, toolchain_dir: &str) -> Result<String> {
        Ok(format!(r#". "{}/env""#, toolchain_dir))
    }
}

struct Posix;
impl UnixShell for Posix {
    fn does_exist(&self) -> bool {
        true
    }

    fn rcfiles(&self) -> Vec<PathBuf> {
        vec![BaseDirs::new().unwrap().home_dir().join(".profile")]
    }

    fn update_rcs(&self) -> Vec<PathBuf> {
        // Write to .profile even if it doesn't exist. It's the only rc in the
        // POSIX spec so it should always be set up.
        self.rcfiles()
    }
}

struct Bash;

impl UnixShell for Bash {
    fn does_exist(&self) -> bool {
        !self.update_rcs().is_empty()
    }

    fn rcfiles(&self) -> Vec<PathBuf> {
        // Bash also may read .profile, however Rustup already includes handling
        // .profile as part of POSIX and always does setup for POSIX shells.
        [".bash_profile", ".bash_login", ".bashrc"]
            .iter()
            .map(|rc| BaseDirs::new().unwrap().home_dir().join(rc))
            .collect()
        // TODO: Verify the output of this
    }

    fn update_rcs(&self) -> Vec<PathBuf> {
        self.rcfiles()
            .into_iter()
            .filter(|rc| rc.is_file())
            .collect()
    }
}

struct Zsh;

impl Zsh {
    fn zdotdir() -> Result<PathBuf> {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        if matches!(std::env::var("SHELL"), Ok(sh) if sh.contains("zsh")) {
            match std::env::var("ZDOTDIR") {
                Ok(dir) if !dir.is_empty() => Ok(PathBuf::from(dir)),
                _ => panic!("ZDOTDIR not set"),
                // TODO:IMPROVE
            }
        } else {
            match std::process::Command::new("zsh")
                .args(["-c", "'echo $ZDOTDIR'"])
                .output()
            {
                Ok(io) if !io.stdout.is_empty() => Ok(PathBuf::from(OsStr::from_bytes(&io.stdout))),
                _ => panic!("ZDOTDIR not set"),
                // TODO:IMPROVE
            }
        }
    }
}

impl UnixShell for Zsh {
    fn does_exist(&self) -> bool {
        // zsh has to either be the shell or be callable for zsh setup.
        matches!(std::env::var("SHELL"), Ok(sh) if sh.contains("zsh"))
            || find_cmd(&["zsh"]).is_some()
    }

    fn rcfiles(&self) -> Vec<PathBuf> {
        let home_dir: Option<PathBuf> = Some(BaseDirs::new().unwrap().home_dir().into());
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

struct Fish;

impl UnixShell for Fish {
    fn does_exist(&self) -> bool {
        // fish has to either be the shell or be callable for fish setup.
        matches!(std::env::var("SHELL"), Ok(sh) if sh.contains("fish"))
            || find_cmd(&["fish"]).is_some()
    }

    // > "$XDG_CONFIG_HOME/fish/conf.d" (or "~/.config/fish/conf.d" if that variable is unset) for the user
    // from <https://github.com/fish-shell/fish-shell/issues/3170#issuecomment-228311857>
    fn rcfiles(&self) -> Vec<PathBuf> {
        let p0 = std::env::var("XDG_CONFIG_HOME").ok().map(|p| {
            let mut path = PathBuf::from(p);
            path.push("fish/conf.d/espup.fish");
            path
        });

        let p1 = BaseDirs::new()
            .unwrap()
            .home_dir()
            .join(".config/fish/conf.d/espup.fish");

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

    fn source_string(&self, toolchain_dir: &str) -> Result<String> {
        Ok(format!(r#". "{}/env.fish""#, toolchain_dir))
    }
}

pub(crate) fn find_cmd<'a>(cmds: &[&'a str]) -> Option<&'a str> {
    cmds.iter().cloned().find(|&s| has_cmd(s))
}

fn has_cmd(cmd: &str) -> bool {
    let cmd = format!("{}{}", cmd, env::consts::EXE_SUFFIX);
    let path = std::env::var_os("PATH").unwrap_or_default();
    env::split_paths(&path)
        .map(|p| p.join(&cmd))
        .any(|p| p.exists())
}

pub fn write_file(path: &Path, contents: &str) -> Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .create(true)
        .open(path)?;

    Write::write_all(&mut file, contents.as_bytes())?;

    file.sync_data()?;

    Ok(())
}
