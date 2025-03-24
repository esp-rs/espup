# espup

[![Crates.io](https://img.shields.io/crates/v/espup.svg)](https://crates.io/crates/espup)
![MSRV](https://img.shields.io/badge/MSRV-1.81.0-blue?labelColor=1C2C2E&logo=Rust&style=flat-square)
[![Continuous Integration](https://github.com/esp-rs/espup/actions/workflows/ci.yaml/badge.svg)](https://github.com/esp-rs/espup/actions/workflows/ci.yaml)
[![Security audit](https://github.com/esp-rs/espup/actions/workflows/audit.yaml/badge.svg)](https://github.com/esp-rs/espup/actions/workflows/audit.yaml)
[![Matrix](https://img.shields.io/matrix/esp-rs:matrix.org?label=join%20matrix&color=BEC5C9&labelColor=1C2C2E&logo=matrix&style=flat-square)](https://matrix.to/#/#esp-rs:matrix.org)


> `rustup` for [esp-rs](https://github.com/esp-rs/)

`espup` is a tool for installing and maintaining the required toolchains for developing applications in Rust for Espressif SoC's.

To better understand what `espup` installs, see [the installation chapter of `The Rust on ESP Book`](https://esp-rs.github.io/book/installation/index.html)

## Requirements

Before running or installing `espup`, make sure that [`rustup`](https://rustup.rs/) is installed.

Linux systems also require the following packages:
- Ubuntu/Debian
  ```sh
  sudo apt-get install -y gcc build-essential curl pkg-config
  ```
- Fedora
  ```sh
  sudo dnf -y install perl gcc
  ```
  - `perl` is required to build `openssl-sys`
- openSUSE Thumbleweed/Leap
  ```
  sudo zypper install -y gcc ninja make
  ```

## Installation

```sh
cargo install espup --locked
```

It's also possible to use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) or to directly download the pre-compiled [release binaries](https://github.com/esp-rs/espup/releases).

<details>

<summary>Commands to install pre-compiled release binaries</summary>

- Linux aarch64
  ```sh
  curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-unknown-linux-gnu -o espup
  chmod a+x espup
  ```
- Linux x86_64
  ```sh
  curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-unknown-linux-gnu -o espup
  chmod a+x espup
  ```
- macOS aarch64
  ```sh
  curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-apple-darwin -o espup
  chmod a+x espup
  ```
- macOS x86_64
  ```sh
  curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-apple-darwin -o espup
  chmod a+x espup
  ```
- Windows MSVC
  ```powershell
  Invoke-WebRequest 'https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-pc-windows-msvc.exe' -OutFile .\espup.exe
  ```

</details>

## Quickstart

See [Usage](#usage) section for more details.

```sh
espup install
# Unix
. $HOME/export-esp.sh
# Windows does not require sourcing any file
```

> [!IMPORTANT]
> The generated export file, by default `export-esp`, needs to be sourced in every terminal in Unix systems before building an application. On Windows, environment variables are automatically injected into your system and don't need to be sourced.

## Usage

```
Usage: espup <COMMAND>

Commands:
  completions  Generate completions for the given shell
  install      Installs Espressif Rust ecosystem
  uninstall    Uninstalls Espressif Rust ecosystem
  update       Updates Xtensa Rust toolchain
  help         Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help
  -V, --version  Print version
```
### Completions Subcommand

For detailed instructions on how to enable tab completion, see [Enable tab completion for Bash, Fish, Zsh, or PowerShell](#enable-tab-completion-for-bash-fish-zsh-or-powershell) section.

```
Usage: espup completions [OPTIONS] <SHELL>

Arguments:
  <SHELL>  Shell to generate completions for [possible values: bash, elvish, fish, powershell, zsh]

Options:
  -l, --log-level <LOG_LEVEL>  Verbosity level of the logs [default: info] [possible values: debug, info, warn, error]
  -h, --help                   Print help
```

### Install Subcommand

> [!NOTE]
> #### Xtensa Rust destination path
>  Installation paths can be modified by setting the environment variables [`CARGO_HOME`](https://doc.rust-lang.org/cargo/reference/environment-variables.html) and [`RUSTUP_HOME`](https://rust-lang.github.io/rustup/environment-variables.html) before running the `install` command. By default, toolchains will be installed under `<rustup_home>/toolchains/esp`, although this can be changed using the `-a/--name` option.

> [!NOTE]
> #### GitHub API
>  During the installation process, several GitHub queries are made, [which are subject to certain limits](https://docs.github.com/en/rest/overview/resources-in-the-rest-api?apiVersion=2022-11-28#rate-limiting). Our number of queries should not hit the limit unless you are running `espup install` command numerous times in a short span of time. We recommend setting the [`GITHUB_TOKEN` environment variable](https://docs.github.com/en/actions/security-guides/automatic-token-authentication#about-the-github_token-secret) when using `espup` in CI, if you want to use `espup` on CI, recommend using it via the [`xtensa-toolchain` action](https://github.com/esp-rs/xtensa-toolchain/), and making sure `GITHUB_TOKEN` is not set when using it on a host machine. See https://github.com/esp-rs/xtensa-toolchain/issues/15 for more details on this.

```
Usage: espup install [OPTIONS]

Options:
  -d, --default-host <DEFAULT_HOST>
          Target triple of the host

          [possible values: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86_64-pc-windows-msvc, x86_64-pc-windows-gnu, x86_64-apple-darwin, aarch64-apple-darwin]

-r, --esp-riscv-gcc
          Install Espressif RISC-V toolchain built with croostool-ng

          Only install this if you don't want to use the systems RISC-V toolchain

  -f, --export-file <EXPORT_FILE>
          Relative or full path for the export file that will be generated. If no path is provided, the file will be generated under home directory (https://docs.rs/dirs/latest/dirs/fn.home_dir.html)

          [env: ESPUP_EXPORT_FILE=]

  -e, --extended-llvm
          Extends the LLVM installation.

          This will install the whole LLVM instead of only installing the libs.

  -l, --log-level <LOG_LEVEL>
          Verbosity level of the logs

          [default: info]
          [possible values: debug, info, warn, error]

  -a, --name <NAME>
          Xtensa Rust toolchain name

          [default: esp]

  -n, --nightly-version <NIGHTLY_VERSION>
          Nightly Rust toolchain version

          [default: nightly]

  -k, --skip-version-parse
          Skips parsing Xtensa Rust version

  -s, --std
          Only install toolchains required for STD applications.

          With this option, espup will skip GCC installation (it will be handled by esp-idf-sys), hence you won't be able to build no_std applications.

  -t, --targets <TARGETS>
          Comma or space separated list of targets [esp32,esp32c2,esp32c3,esp32c6,esp32h2,esp32s2,esp32s3,esp32p4,all]

          [default: all]

  -v, --toolchain-version <TOOLCHAIN_VERSION>
          Xtensa Rust toolchain version

  -h, --help
          Print help (see a summary with '-h')
```

### Uninstall Subcommand

```
Usage: espup uninstall [OPTIONS]

Options:
  -l, --log-level <LOG_LEVEL>  Verbosity level of the logs [default: info] [possible values: debug, info, warn, error]
  -a, --name <NAME>            Xtensa Rust toolchain name [default: esp]
  -h, --help                   Print help
```

### Update Subcommand

```
Usage: espup update [OPTIONS]

Options:
  -d, --default-host <DEFAULT_HOST>
          Target triple of the host

          [possible values: x86_64-unknown-linux-gnu, aarch64-unknown-linux-gnu, x86_64-pc-windows-msvc, x86_64-pc-windows-gnu, x86_64-apple-darwin, aarch64-apple-darwin]

  -f, --export-file <EXPORT_FILE>
          Relative or full path for the export file that will be generated. If no path is provided, the file will be generated under home directory (https://docs.rs/dirs/latest/dirs/fn.home_dir.html)

          [env: ESPUP_EXPORT_FILE=]

  -e, --extended-llvm
          Extends the LLVM installation.

          This will install the whole LLVM instead of only installing the libs.

  -l, --log-level <LOG_LEVEL>
          Verbosity level of the logs

          [default: info]
          [possible values: debug, info, warn, error]

  -a, --name <NAME>
          Xtensa Rust toolchain name

          [default: esp]

  -n, --nightly-version <NIGHTLY_VERSION>
          Nightly Rust toolchain version

          [default: nightly]

  -k, --skip-version-parse
          Skips parsing Xtensa Rust version

  -s, --std
          Only install toolchains required for STD applications.

          With this option, espup will skip GCC installation (it will be handled by esp-idf-sys), hence you won't be able to build no_std applications.

  -t, --targets <TARGETS>
          Comma or space separated list of targets [esp32,esp32c2,esp32c3,esp32c6,esp32h2,esp32s2,esp32s3,all]

          [default: all]

  -v, --toolchain-version <TOOLCHAIN_VERSION>
          Xtensa Rust toolchain version

  -h, --help
          Print help (see a summary with '-h')
```

## Enable tab completion for Bash, Fish, Zsh, or PowerShell

`espup` supports generating completion scripts for Bash, Fish, Zsh, and
PowerShell. See `espup help completions` for full details, but the gist is as
simple as using one of the following:

```console
# Bash
$ espup completions bash > ~/.local/share/bash-completion/completions/espup

# Bash (macOS/Homebrew)
$ espup completions bash > $(brew --prefix)/etc/bash_completion.d/espup.bash-completion

# Fish
$ mkdir -p ~/.config/fish/completions
$ espup completions fish > ~/.config/fish/completions/espup.fish

# Zsh
$ espup completions zsh > ~/.zfunc/_espup

# PowerShell v5.0+
$ espup completions powershell >> $PROFILE.CurrentUserCurrentHost
# or
$ espup completions powershell | Out-String | Invoke-Expression
```

**Note**: you may need to restart your shell in order for the changes to take
effect.

For `zsh`, you must then add the following line in your `~/.zshrc` before
`compinit`:

```zsh
fpath+=~/.zfunc
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
