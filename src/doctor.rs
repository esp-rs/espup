//! Environment health checks (`espup doctor`) and shell exports (`espup env`).

use crate::env::get_export_file;
use directories::BaseDirs;
use std::{
    env,
    path::{Path, PathBuf},
    process::Command,
};

/// One check line.
pub struct Check {
    pub name: &'static str,
    pub ok: bool,
    pub detail: String,
    /// Optional tools show as WARN and do not fail the doctor exit code.
    pub optional: bool,
}

fn toolchain_dir(name: &str) -> PathBuf {
    let rustup = env::var("RUSTUP_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            BaseDirs::new()
                .map(|b| b.home_dir().join(".rustup"))
                .unwrap_or_else(|| PathBuf::from(".rustup"))
        });
    rustup.join("toolchains").join(name)
}

fn path_has(dir: &Path) -> bool {
    env::var_os("PATH")
        .map(|p| env::split_paths(&p).any(|d| d == dir))
        .unwrap_or(false)
}

fn run_ok(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if out.status.success() {
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        None
    }
}

/// Collect doctor checks for the default (or given) Xtensa toolchain name.
pub fn collect_checks(name: &str) -> Vec<Check> {
    let mut checks = Vec::new();
    let tc = toolchain_dir(name);

    let rustup = run_ok("rustup", &["--version"]);
    checks.push(Check {
        name: "rustup",
        ok: rustup.is_some(),
        detail: rustup.unwrap_or_else(|| "not found on PATH".into()),
        optional: false,
    });

    checks.push(Check {
        name: "toolchain dir",
        ok: tc.is_dir(),
        detail: tc.display().to_string(),
        optional: false,
    });

    let rustc = run_ok("rustc", &["+esp", "--version"]);
    checks.push(Check {
        name: "rustc +esp",
        ok: rustc.is_some(),
        detail: rustc.unwrap_or_else(|| {
            format!(
                "missing — re-run `espup install` or `rustup toolchain link {} {}`",
                name,
                tc.display()
            )
        }),
        optional: false,
    });

    let target_list = run_ok("rustup", &["+esp", "target", "list", "--installed"]);
    let has_xtensa = target_list
        .as_deref()
        .map(|s| s.contains("xtensa-esp32s3") || s.contains("xtensa"))
        .unwrap_or(false);
    checks.push(Check {
        name: "xtensa target",
        ok: true,
        detail: if has_xtensa {
            "installed".into()
        } else {
            "not listed (no_std uses -Zbuild-std=core,alloc — OK)".into()
        },
        optional: false,
    });

    match get_export_file(None).ok() {
        Some(p) => {
            let exists = p.is_file();
            let sourced = env::var("LIBCLANG_PATH").is_ok()
                || env::var("CLANG_PATH").is_ok()
                || path_has(&tc.join("xtensa-esp-elf-clang"));
            checks.push(Check {
                name: "export file",
                ok: exists,
                detail: if !exists {
                    format!("missing: {} (run `espup install`)", p.display())
                } else if sourced {
                    format!("{} (env looks loaded)", p.display())
                } else {
                    format!(
                        "{} exists but not sourced — run: . {}   (or eval \"$(espup env)\")",
                        p.display(),
                        p.display()
                    )
                },
                optional: false,
            });
        }
        None => checks.push(Check {
            name: "export file",
            ok: false,
            detail: "cannot resolve home directory".into(),
            optional: false,
        }),
    }

    let libclang = env::var("LIBCLANG_PATH").ok();
    checks.push(Check {
        name: "LIBCLANG_PATH",
        ok: libclang
            .as_ref()
            .map(|p| Path::new(p).exists())
            .unwrap_or(false),
        detail: libclang.unwrap_or_else(|| {
            let guess = tc.join("xtensa-esp-elf-clang");
            if guess.exists() {
                format!("unset (expected under {})", guess.display())
            } else {
                "unset — source export-esp.sh or eval \"$(espup env)\"".into()
            }
        }),
        optional: false,
    });

    for tool in ["ldproxy", "espflash"] {
        let v = run_ok(tool, &["--version"]).or_else(|| run_ok(tool, &["-V"]));
        checks.push(Check {
            name: tool,
            ok: v.is_some(),
            detail: v.unwrap_or_else(|| "not found (optional; cargo install …)".into()),
            optional: true,
        });
    }

    let mcu = env::var("MCU").ok();
    checks.push(Check {
        name: "MCU env",
        ok: true,
        detail: mcu.unwrap_or_else(|| "unset (usually set in project .cargo/config.toml)".into()),
        optional: true,
    });

    checks
}

/// Print doctor report; returns true if all required checks passed.
pub fn print_doctor(name: &str) -> bool {
    let checks = collect_checks(name);
    let mut all_ok = true;
    println!("espup doctor — toolchain '{name}'\n");
    for c in &checks {
        let mark = if c.ok {
            "OK  "
        } else if c.optional {
            "WARN"
        } else {
            "FAIL"
        };
        if !c.ok && !c.optional {
            all_ok = false;
        }
        println!("  [{mark}] {:<16} {}", c.name, c.detail);
    }
    if all_ok {
        println!("\nAll required checks passed.");
    } else {
        println!(
            "\nSome required checks failed. Fix the FAIL lines above, then re-run `espup doctor`."
        );
    }
    all_ok
}

/// Print export lines suitable for `eval "$(espup env)"` (unix).
pub fn print_env_exports(export_file: Option<PathBuf>) -> Result<(), String> {
    let path = get_export_file(export_file).map_err(|e| e.to_string())?;
    if !path.is_file() {
        return Err(format!(
            "export file not found: {} — run `espup install` first",
            path.display()
        ));
    }
    let contents = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    for line in contents.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        println!("{t}");
    }
    Ok(())
}
