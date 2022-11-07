# EspUp

[![Continuous Integration](https://github.com/esp-rs/espup/actions/workflows/ci.yaml/badge.svg)](https://github.com/esp-rs/espup/actions/workflows/ci.yaml)
[![Security audit](https://github.com/esp-rs/espup/actions/workflows/audit.yaml/badge.svg)](https://github.com/esp-rs/espup/actions/workflows/audit.yaml)
[![Open in Remote - Containers](https://img.shields.io/static/v1?label=Remote%20-%20Containers&message=Open&color=blue&logo=visualstudiocode)](https://vscode.dev/redirect?url=vscode://ms-vscode-remote.remote-containers/cloneInVolume?url=https://github.com/esp-rs/espup)
[![Matrix](https://img.shields.io/matrix/esp-rs:matrix.org?label=join%20matrix&color=BEC5C9&labelColor=1C2C2E&logo=matrix&style=flat-square)](https://matrix.to/#/#esp-rs:matrix.org)

> `rustup` for [esp-rs](https://github.com/esp-rs/)

`espup` is a tool for installing and maintaining the required toolchains for
developing applications in Rust for Espressif SoC's.

To better understand what `espup` installs, see [`Rust on ESP targets` chapter of `The Rust on ESP Book`](https://esp-rs.github.io/book/installation/index.html)

## Requirements
Before running or installing `espup`, make sure that the following dependencies are installed.
### Windows

- [git](https://git-scm.com/download/win).
- [Python](https://www.python.org/downloads/): Only required when installing ESP-IDF.

### Linux
- Ubuntu/Debian
```sh
sudo apt-get install -y git python3 python3-pip gcc build-essential curl pkg-config libudev-dev libtinfo5
```
  - `libudev-dev` is only required when installing `cargo-espflash`.
  - `python3` and `python3-pip` are only required when installing ESP-IDF.
  - `libtinfo5` is required by LLVM 15.
- Fedora
```sh
sudo dnf -y install git python3 python3-pip perl gcc systemd-devel ncurses-compat-libs
```
  - `systemd-devel` is only required when installing `cargo-espflash`.
  - `python3` and `python3-pip` are only required when installing ESP-IDF.
  - `perl` is required to build openssl-sys
  - `ncurses-compat-libs` is required by LLVM 15.
- openSUSE Thumbleweed/Leap
```
sudo zypper install -y git gcc libudev-devel ninja python3 python3-pip make libncurses5
```
  - `libudev-devel` is only required when installing `cargo-espflash`.
  - `python3` and `python3-pip` are only required when installing ESP-IDF.
  - `libncurses5` is required by LLVM 15.


## Installation

```sh
cargo install espup
```

It's also possible to use [cargo-binstall](https://github.com/cargo-bins/cargo-binstall) or to directly download the pre-compiled [release binaries](https://github.com/esp-rs/espup/releases):

### Linux aarch64
```sh
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-unknown-linux-gnu -o espup
chmod a+x espup
```

### Linux x86_64
```sh
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-unknown-linux-gnu -o espup
chmod a+x espup
```

### macOS aarch64
```sh
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-aarch64-apple-darwin -o espup
chmod a+x espup
```

### macOS x86_64
```sh
curl -L https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-apple-darwin -o espup
chmod a+x espup
```

### Windows MSVC
```powershell
Invoke-WebRequest 'https://github.com/esp-rs/espup/releases/latest/download/espup-x86_64-pc-windows-msvc.exe' -OutFile .\espup.exe
```

## Quickstart
See [Usage](#usage) section for more details.
### Install
- [`no_std`](https://esp-rs.github.io/book/overview/bare-metal.html)
  ```sh
  espup install
  # Unix
  . ./export-esp.sh
  # Windows
  .\export-esp.ps1
  ```
- [`std`](https://esp-rs.github.io/book/overview/using-the-standard-library.html):
  Installing `esp-idf` via `espup` is not mandatory, as [`esp-idf-sys`](https://github.com/esp-rs/esp-idf-sys) already takes care of it, but has some benefits.
  ```sh
  espup install --espidf-version <ESPIDF_VERSION>
  # Unix
  . ./export-esp.sh
  # Windows
  .\export-esp.ps1
  ```

> **Warning**
>
> The generated export file, by default `export-esp`, needs to be sourced in every terminal
> before building an application.
### Uninstall
```sh
espup uninstall
```

### Update
```sh
espup update
```

## Usage

```
Usage: espup <COMMAND>

Commands:
  install    Installs esp-rs environment
  uninstall  Uninstalls esp-rs environment
  update     Updates Xtensa Rust toolchain
  help       Print this message or the help of the given subcommand(s)

Options:
  -h, --help     Print help information
  -V, --version  Print version information
```

### Install Subcommand

> **Note**
>
>  Installation paths can be modified by setting the environment
variables [`CARGO_HOME`](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
and [`RUSTUP_HOME`](https://rust-lang.github.io/rustup/environment-variables.html)
before running the `install` command.
Xtensa Rust toolchain will be installed under `<rustup_home>/toolchains/esp`.


```
Usage: espup install [OPTIONS]

Options:
  -d, --default-host <DEFAULT_HOST>
          Target triple of the host

  -e, --espidf-version <ESPIDF_VERSION>
          ESP-IDF version to install. If empty, no esp-idf is installed. Version format:

          - `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository.

          - `tag:<tag>`: Uses the tag `<tag>` of the `esp-idf` repository.

          - `branch:<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.

          - `v<major>.<minor>` or `<major>.<minor>`: Uses the tag `v<major>.<minor>` of the `esp-idf` repository.

          - `<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.

          When using this option, `ldproxy` crate will also be installed.

  -f, --export-file <EXPORT_FILE>
          Destination of the generated export file

  -c, --extra-crates <EXTRA_CRATES>
          Comma or space list of extra crates to install

          [default: ]

  -x, --llvm-version <LLVM_VERSION>
          LLVM version

          [default: 15]
          [possible values: 15]

  -x, --llvm-version <LLVM_VERSION>
          LLVM version

          [default: 15]
          [possible values: 15]

  -l, --log-level <LOG_LEVEL>
          Verbosity level of the logs

          [default: info]
          [possible values: debug, info, warn, error]

  -n, --nightly-version <NIGHTLY_VERSION>
          Nightly Rust toolchain version

          [default: nightly]

  -m, --profile-minimal
          Minifies the installation

  -t, --targets <TARGETS>
          Comma or space separated list of targets [esp32,esp32s2,esp32s3,esp32c3,all]

          [default: all]

  -v, --toolchain-version <TOOLCHAIN_VERSION>
          Xtensa Rust toolchain version

  -h, --help
          Print help information (use `-h` for a summary)
```

### Uninstall Subcommand

```
Usage: espup uninstall [OPTIONS]

Options:
  -l, --log-level <LOG_LEVEL>  Verbosity level of the logs [default: info] [possible values: debug, info, warn, error]
  -h, --help                   Print help information
```

### Update Subcommand

```
Usage: espup update [OPTIONS]

Options:
  -d, --default-host <DEFAULT_HOST>
          Target triple of the host
  -l, --log-level <LOG_LEVEL>
          Verbosity level of the logs [default: info] [possible values: debug, info, warn, error]
  -v, --toolchain-version <TOOLCHAIN_VERSION>
          Xtensa Rust toolchain version
  -h, --help
          Print help information
```

## Known Issues or Limitations

- If installing esp-idf in Windows, only `all` targets is allowed.
- In Windows, when installing esp-idf fails with:
   ```
   ERROR: Could not find a version that satisfies the requirement windows-curses; sys_platform == "win32" (from esp-windows-curses) (from versions: none)
   ERROR: No matching distribution found for windows-curses; sys_platform == "win32"
   Traceback (most recent call last):
           File "<home_dir>/.espressif\esp-idf-ae062fbba3ded0aa\release-v4.4\tools\idf_tools.py", line 1973, in <module>
   main(sys.argv[1:])
           File "<home_dir>/.espressif\esp-idf-ae062fbba3ded0aa\release-v4.4\tools\idf_tools.py", line 1969, in main
   action_func(args)
           File "<home_dir>/.espressif\esp-idf-ae062fbba3ded0aa\release-v4.4\tools\idf_tools.py", line 1619, in action_install_python_env
   subprocess.check_call(run_args, stdout=sys.stdout, stderr=sys.stderr, env=env_copy)
           File "C:\Python311\Lib\subprocess.py", line 413, in check_call
   raise CalledProcessError(retcode, cmd)
   subprocess.CalledProcessError: Command '['<home_dir>/.espressif\\python_env\\idf4.4_py3.11_env\\Scripts\\python.exe', '-m', 'pip', 'install', '--no-warn-script-location', '-r', <home_dir>/.espressif\\esp-idf-ae062fbba3ded0aa\\release-v4.4\\requirements.txt', '--extra-index-url', 'https://dl.espressif.com/pypi']' returned non-zero exit status 1.
   Error: Could not install esp-idf
   ```
  *_Solution_*: Use a python version between `3.6` and `3.10` as `3.11` Python wheels are not yet released for Windows.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
