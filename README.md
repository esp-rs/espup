# EspUp
[![Continuous Integration](https://github.com/SergioGasquez/espup/actions/workflows/ci.yaml/badge.svg)](https://github.com/SergioGasquez/espup/actions/workflows/ci.yaml)
[![Security audit](https://github.com/SergioGasquez/espup/actions/workflows/audit.yaml/badge.svg)](https://github.com/SergioGasquez/espup/actions/workflows/audit.yaml)
[![Open in Remote - Containers](https://img.shields.io/static/v1?label=Remote%20-%20Containers&message=Open&color=blue&logo=visualstudiocode)](https://vscode.dev/redirect?url=vscode://ms-vscode-remote.remote-containers/cloneInVolume?url=https://github.com/SergioGasquez/espup)

Tool for installing and maintaining ESP Rust toolchain.
> **Warning**
>
>  This crate is still in early development (See [Known Issues section](#known-issues)). Use at your own risk and, please, report any issues that you find!

## Requirements
### Windows
- Python must be installed and the version should be between `3.6` and `3.10`.

## Installation
```sh
cargo install espup --git https://github.com/SergioGasquez/espup
```
## Usage
> **Warning**
>
>  Only install subcommand is available at the momment.
```sh
USAGE:
    espup <SUBCOMMAND>

OPTIONS:
    -h, --help    Print help information

SUBCOMMANDS:
    help         Print this message or the help of the given subcommand(s)
    install      Installs esp-rs environment
    reinstall    Reinstalls esp-rs environment
    uninstall    Uninstalls esp-rs environment
    update       Updates esp-rs Rust toolchain
```
### Install Subcommand
EspUp allows you to customise your installation paths by setting the environment
variables [`CARGO_HOME`](https://doc.rust-lang.org/cargo/reference/environment-variables.html)
and [`RUSTUP_HOME`](https://rust-lang.github.io/rustup/environment-variables.html) before running the executable.
Xtensa Rust toolchain will be installed under `<rustup_home>/toolchains/esp`.
```sh
Installs esp-rs environment

USAGE:
    espup install [OPTIONS]

OPTIONS:
    -c, --extra-crates <EXTRA_CRATES>
            Comma or space list of extra crates to install

            [default: cargo-espflash]

    -d, --toolchain-destination <TOOLCHAIN_DESTINATION>
            Xtensa Rust toolchain instalation folder

    -e, --espidf-version <ESPIDF_VERSION>
            ESP-IDF version to install. If empty, no esp-idf is installed. Version format:

            - `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository.

            - `tag:<tag>`: Uses the tag `<tag>` of the `esp-idf` repository.

            - `branch:<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.

            - `v<major>.<minor>` or `<major>.<minor>`: Uses the tag `v<major>.<minor>` of the
            `esp-idf` repository.

            - `<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.

    -f, --export-file <EXPORT_FILE>
            Destination of the generated export file

            [default: export-esp.sh]

    -h, --help
            Print help information

    -m, --profile-minimal
            Minifies the installation

    -n, --nightly-version <NIGHTLY_VERSION>
            Nightly Rust toolchain version

            [default: nightly]

    -q, --quiet
            Less output per occurrence

    -t, --targets <TARGETS>
            Comma or space separated list of targets [esp32,esp32s2,esp32s3,esp32c3,all]

            [default: all]

    -v, --verbose
            More output per occurrence

    -x, --toolchain-version <TOOLCHAIN_VERSION>
            Xtensa Rust toolchain version

            [default: 1.62.1.0]
```
### Uninstall Subcommand

```sh
Uninstalls esp-rs environment

USAGE:
    espup uninstall [OPTIONS]

OPTIONS:
    -c, --remove-clang
            Removes clang

    -e, --espidf-version <ESPIDF_VERSION>
            ESP-IDF version to uninstall. If empty, no esp-idf is uninsalled. Version format:

            - `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository.

            - `tag:<tag>`: Uses the tag `<tag>` of the `esp-idf` repository.

            - `branch:<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.

            - `v<major>.<minor>` or `<major>.<minor>`: Uses the tag `v<major>.<minor>` of the
            `esp-idf` repository.

            - `<branch>`: Uses the branch `<branch>` of the `esp-idf` repository.

    -h, --help
            Print help information

    -l, --log-level <LOG_LEVEL>
            Verbosity level of the logs

            [default: info]
            [possible values: debug, info, warn, error]
```
## Known Issues
 - When installing esp-idf in Windows, only `all` targets is wokring. If you try to install
 any esp-idf version for any target combination that does not include all of them, you will
 have issues activating the environment.

## Troubleshooting
- In Windows, when installing esp-idf fails with
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
*_Solution_*: This is due to python `3.11` being used. Use a python version between `3.6` and `3.10`

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in
the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without
any additional terms or conditions.
