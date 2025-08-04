//! LLVM Toolchain source and installation tools.

#[cfg(windows)]
use crate::env::{delete_env_variable, get_windows_path_var, set_env_variable};
use crate::{
    error::Error,
    host_triple::HostTriple,
    toolchain::{Installable, download_file, rust::RE_EXTENDED_SEMANTIC_VERSION},
};
use async_trait::async_trait;
#[cfg(unix)]
use directories::BaseDirs;
use log::{info, warn};
use miette::Result;
use regex::Regex;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::{env, fs::File};
#[cfg(unix)]
use std::{fs::create_dir_all, os::unix::fs::symlink};
use tokio::fs::remove_dir_all;

const DEFAULT_LLVM_REPOSITORY: &str = "https://github.com/espressif/llvm-project/releases/download";
const DEFAULT_LLVM_15_VERSION: &str = "esp-15.0.0-20221201";
#[cfg(windows)]
const OLD_LLVM_16_VERSION: &str = "esp-16.0.0-20230516";
const DEFAULT_LLVM_16_VERSION: &str = "esp-16.0.4-20231113";
const DEFAULT_LLVM_17_VERSION: &str = "esp-17.0.1_20240419";
const DEFAULT_LLVM_18_VERSION: &str = "esp-18.1.2_20240912";
const DEFAULT_LLVM_19_VERSION: &str = "esp-19.1.2_20250225";
pub const CLANG_NAME: &str = "xtensa-esp32-elf-clang";

#[derive(Debug, Clone, Default)]
pub struct Llvm {
    // /// If `true`, full LLVM, instead of only libraries, are installed.
    extended: bool,
    /// LLVM libs-only toolchain file name.
    pub file_name_libs: Option<String>,
    /// LLVM "full" toolchain file name.
    pub file_name_full: Option<String>,
    /// Host triple.
    pub host_triple: HostTriple,
    /// LLVM Toolchain path.
    pub path: PathBuf,
    /// The repository containing LLVM sources.
    pub repository_url: String,
    /// LLVM Version ["15", "16", "17"].
    pub version: String,
}

impl Llvm {
    /// Gets the name of the LLVM arch based on the host triple.
    fn get_arch(host_triple: &HostTriple, version: &str) -> String {
        if version == DEFAULT_LLVM_17_VERSION
            || version == DEFAULT_LLVM_18_VERSION
            || version == DEFAULT_LLVM_19_VERSION
        {
            let arch = match host_triple {
                HostTriple::Aarch64AppleDarwin => "aarch64-apple-darwin",
                HostTriple::X86_64AppleDarwin => "x86_64-apple-darwin",
                HostTriple::X86_64UnknownLinuxGnu => "x86_64-linux-gnu",
                HostTriple::Aarch64UnknownLinuxGnu => "aarch64-linux-gnu",
                HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => {
                    "x86_64-w64-mingw32"
                }
            };
            arch.to_string()
        } else {
            let arch = match host_triple {
                HostTriple::Aarch64AppleDarwin => "macos-arm64",
                HostTriple::X86_64AppleDarwin => "macos",
                HostTriple::X86_64UnknownLinuxGnu => "linux-amd64",
                HostTriple::Aarch64UnknownLinuxGnu => "linux-arm64",
                HostTriple::X86_64PcWindowsMsvc | HostTriple::X86_64PcWindowsGnu => "win64",
            };
            arch.to_string()
        }
    }

    /// Gets the binary path.
    fn get_lib_path(&self) -> String {
        match std::cfg!(windows) {
            true => format!("{}/esp-clang/bin", self.path.to_str().unwrap()).replace('/', "\\"),
            false => format!("{}/esp-clang/lib", self.path.to_str().unwrap()),
        }
    }

    /// Gets the binary path of clang
    fn get_bin_path(&self) -> String {
        match std::cfg!(windows) {
            true => format!("{}/esp-clang/bin/clang.exe", self.path.to_str().unwrap())
                .replace('/', "\\"),
            false => format!("{}/esp-clang/bin/clang", self.path.to_str().unwrap()),
        }
    }

    /// Create a new instance with default values and proper toolchain version.
    pub fn new(
        toolchain_path: &Path,
        host_triple: &HostTriple,
        extended: bool,
        xtensa_rust_version: &str,
    ) -> Result<Self, Error> {
        let re_extended: Regex = Regex::new(RE_EXTENDED_SEMANTIC_VERSION).unwrap();
        let (major, minor, patch, subpatch) = match re_extended.captures(xtensa_rust_version) {
            Some(version) => (
                version.get(1).unwrap().as_str().parse::<u8>().unwrap(),
                version.get(2).unwrap().as_str().parse::<u8>().unwrap(),
                version.get(3).unwrap().as_str().parse::<u8>().unwrap(),
                version.get(4).unwrap().as_str().parse::<u8>().unwrap(),
            ),
            None => return Err(Error::InvalidVersion(xtensa_rust_version.to_string())),
        };

        // Use LLVM 15 for versions 1.69.0.0 and below and LLVM 16 for versions 1.77.0 and bellow
        let version = if (major == 1 && minor == 69 && patch == 0 && subpatch == 0)
            || (major == 1 && minor < 69)
        {
            DEFAULT_LLVM_15_VERSION.to_string()
        } else if (major == 1 && minor == 77 && patch == 0 && subpatch == 0)
            || (major == 1 && minor < 77)
        {
            DEFAULT_LLVM_16_VERSION.to_string()
        } else if (major == 1 && minor == 81 && patch == 0 && subpatch == 0)
            || (major == 1 && minor < 81)
        {
            DEFAULT_LLVM_17_VERSION.to_string()
        } else if (major == 1 && minor == 84 && patch == 0 && subpatch == 0)
            || (major == 1 && minor < 84)
        {
            DEFAULT_LLVM_18_VERSION.to_string()
        } else {
            DEFAULT_LLVM_19_VERSION.to_string()
        };

        let name = if version == DEFAULT_LLVM_17_VERSION
            || version == DEFAULT_LLVM_18_VERSION
            || version == DEFAULT_LLVM_19_VERSION
        {
            "clang-"
        } else {
            "llvm-"
        };

        let (file_name_libs, file_name_full) = {
            let file_name_full = format!(
                "{}{}-{}.tar.xz",
                name,
                version,
                Self::get_arch(host_triple, &version)
            );

            let file_name_libs = if version != DEFAULT_LLVM_17_VERSION
                && version != DEFAULT_LLVM_18_VERSION
                && version != DEFAULT_LLVM_19_VERSION
            {
                format!("libs_{file_name_full}")
            } else {
                format!("libs-{file_name_full}")
            };

            // For LLVM 15 and 16 the "full" tarball was a superset of the "libs" tarball, so if
            // we're in extended LLVM mode we only need the "full" tarballs for those versions.
            //
            // Later LLVM versions are built such that the "full" tarball has a statically linked
            // `clang` binary and therefore doesn't contain libclang, and so then we need to fetch
            // both tarballs.
            if version == DEFAULT_LLVM_15_VERSION || version == DEFAULT_LLVM_16_VERSION {
                if extended {
                    (None, Some(file_name_full))
                } else {
                    (Some(file_name_libs), None)
                }
            } else if extended {
                (Some(file_name_libs), Some(file_name_full))
            } else {
                (Some(file_name_libs), None)
            }
        };

        let repository_url = format!("{DEFAULT_LLVM_REPOSITORY}/{version}");
        #[cfg(unix)]
        let path = toolchain_path.join(CLANG_NAME).join(&version);
        #[cfg(windows)]
        let path = toolchain_path.join(CLANG_NAME);

        Ok(Self {
            extended,
            file_name_libs,
            file_name_full,
            host_triple: host_triple.clone(),
            path,
            repository_url,
            version,
        })
    }

    /// Uninstall LLVM toolchain.
    pub async fn uninstall(toolchain_path: &Path) -> Result<(), Error> {
        info!("Uninstalling Xtensa LLVM");
        let llvm_path = toolchain_path.join(CLANG_NAME);
        if llvm_path.exists() {
            #[cfg(windows)]
            if cfg!(windows) {
                let mut updated_path = get_windows_path_var()?.replace(
                    &format!(
                        "{}\\{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        DEFAULT_LLVM_15_VERSION,
                    ),
                    "",
                );
                updated_path = updated_path.replace(
                    &format!(
                        "{}\\{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        OLD_LLVM_16_VERSION,
                    ),
                    "",
                );
                updated_path = updated_path.replace(
                    &format!(
                        "{}\\{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        DEFAULT_LLVM_16_VERSION,
                    ),
                    "",
                );
                updated_path = updated_path.replace(
                    &format!(
                        "{}\\{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        DEFAULT_LLVM_17_VERSION,
                    ),
                    "",
                );
                updated_path = updated_path.replace(
                    &format!(
                        "{}\\{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        DEFAULT_LLVM_18_VERSION,
                    ),
                    "",
                );
                updated_path = updated_path.replace(
                    &format!(
                        "{}\\{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                        DEFAULT_LLVM_19_VERSION,
                    ),
                    "",
                );
                updated_path = updated_path.replace(
                    &format!(
                        "{}\\esp-clang\\bin;",
                        llvm_path.display().to_string().replace('/', "\\"),
                    ),
                    "",
                );
                set_env_variable("PATH", &updated_path)?;
                delete_env_variable("LIBCLANG_PATH")?;
                delete_env_variable("CLANG_PATH")?;
            }
            remove_dir_all(&llvm_path)
                .await
                .map_err(|_| Error::RemoveDirectory(llvm_path.display().to_string()))?;
            #[cfg(unix)]
            if cfg!(unix) {
                let espup_dir = BaseDirs::new().unwrap().home_dir().join(".espup");

                if espup_dir.exists() {
                    remove_dir_all(espup_dir.display().to_string())
                        .await
                        .map_err(|_| Error::RemoveDirectory(espup_dir.display().to_string()))?;
                }
            }
        }
        Ok(())
    }
}

#[async_trait]
impl Installable for Llvm {
    async fn install(&self) -> Result<Vec<String>, Error> {
        let mut exports: Vec<String> = Vec::new();

        #[cfg(unix)]
        let install_path = if self.extended {
            Path::new(&self.path).join("esp-clang").join("include")
        } else {
            Path::new(&self.path).to_path_buf()
        };
        #[cfg(windows)]
        let install_path = if self.extended {
            self.path.join(&self.version).join("include")
        } else {
            self.path.join(&self.version)
        };

        if install_path.exists() {
            warn!(
                "Previous installation of LLVM exists in: '{}'. Reusing this installation",
                self.path.to_str().unwrap()
            );
        } else {
            info!("Installing Xtensa LLVM");
            if let Some(file_name_libs) = &self.file_name_libs {
                download_file(
                    format!("{}/{}", self.repository_url, file_name_libs),
                    "idf_tool_xtensa_elf_clang.libs.tar.xz",
                    self.path.to_str().unwrap(),
                    true,
                    false,
                )
                .await?;
            }
            if let Some(file_name_full) = &self.file_name_full {
                download_file(
                    format!("{}/{}", self.repository_url, file_name_full),
                    "idf_tool_xtensa_elf_clang.full.tar.xz",
                    self.path.to_str().unwrap(),
                    true,
                    false,
                )
                .await?;
            }
        }
        // Set environment variables.
        #[cfg(windows)]
        if cfg!(windows) {
            File::create(self.path.join(&self.version))?;
            let libclang_dll = format!("{}\\libclang.dll", self.get_lib_path());
            exports.push(format!("$Env:LIBCLANG_PATH = \"{libclang_dll}\""));
            exports.push(format!(
                "$Env:PATH = \"{};\" + $Env:PATH",
                self.get_lib_path()
            ));
            unsafe {
                env::set_var("LIBCLANG_BIN_PATH", self.get_lib_path());
                env::set_var("LIBCLANG_PATH", libclang_dll);
            }
        }
        #[cfg(unix)]
        if cfg!(unix) {
            exports.push(format!("export LIBCLANG_PATH=\"{}\"", self.get_lib_path()));
            let espup_dir = BaseDirs::new().unwrap().home_dir().join(".espup");

            if !espup_dir.exists() {
                create_dir_all(espup_dir.display().to_string())
                    .map_err(|_| Error::CreateDirectory(espup_dir.display().to_string()))?;
            }
            let llvm_symlink_path = espup_dir.join("esp-clang");
            if llvm_symlink_path.exists() {
                remove_dir_all(&llvm_symlink_path)
                    .await
                    .map_err(|_| Error::RemoveDirectory(llvm_symlink_path.display().to_string()))?;
            }
            info!(
                "Creating symlink between '{}' and '{}'",
                self.get_lib_path(),
                llvm_symlink_path.display()
            );
            symlink(self.get_lib_path(), llvm_symlink_path)?;
        }

        if self.extended {
            #[cfg(windows)]
            if cfg!(windows) {
                exports.push(format!("$Env:CLANG_PATH = \"{}\"", self.get_bin_path()));
                unsafe {
                    env::set_var("CLANG_PATH", self.get_bin_path());
                }
            }
            #[cfg(unix)]
            exports.push(format!("export CLANG_PATH=\"{}\"", self.get_bin_path()));
        }

        Ok(exports)
    }

    fn name(&self) -> String {
        "LLVM".to_string()
    }
}
