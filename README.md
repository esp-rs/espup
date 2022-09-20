# EspUp
Tool for installing and maintaining ESP Rust toolchain.
> **Warning**
>
>  This crate is still in early development (See [Known Issues section](#known-issues)). Use at your own risk and report any issues that you find!

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
```
Installs esp-rs environment

USAGE:
    espup install [OPTIONS]

OPTIONS:
    -b, --build-target <BUILD_TARGET>
            Comma or space separated list of targets [esp32,esp32s2,esp32s3,esp32c3,all] [default:
            all]

    -c, --cargo-home <CARGO_HOME>
            Path to .cargo

    -d, --toolchain-destination <TOOLCHAIN_DESTINATION>
            Toolchain instalation folder

    -e, --extra-crates <EXTRA_CRATES>
            Comma or space list of extra crates to install [default: cargo-espflash]

    -f, --export-file <EXPORT_FILE>
            Destination of the export file generated

    -h, --help
            Print help information

    -l, --llvm-version <LLVM_VERSION>
            LLVM version. [13, 14, 15] [default: 14]

    -m, --minified-espidf
            [Only applies if using -s|--esp-idf-version]. Deletes some esp-idf folders to save space

    -n, --nightly-version <NIGHTLY_VERSION>
            Nightly Rust toolchain version [default: nightly]

    -q, --quiet
            Less output per occurrence

    -r, --rustup-home <RUSTUP_HOME>
            Path to .rustup

    -s, --espidf-version <ESPIDF_VERSION>
            ESP-IDF version to install. If empty, no esp-idf is installed. Format: -
            `commit:<hash>`: Uses the commit `<hash>` of the `esp-idf` repository. - `tag:<tag>`:
            Uses the tag `<tag>` of the `esp-idf` repository. - `branch:<branch>`: Uses the branch
            `<branch>` of the `esp-idf` repository. - `v<major>.<minor>` or `<major>.<minor>`: Uses
            the tag `v<major>.<minor>` of the `esp-idf` repository. - `<branch>`: Uses the branch
            `<branch>` of the `esp-idf` repository

    -t, --toolchain-version <TOOLCHAIN_VERSION>
            Xtensa Rust toolchain version [default: 1.62.1.0]

    -v, --verbose
            More output per occurrence

    -x, --clear-dist
            Removes cached distribution files
```
## Known Issues
 - Esp-idf is only isntalled properpy for `all` targets
 - Esp-idf source file path is not exported in the file nor displayed in terminal properly
 - Windows throws an error installing esp-idf. How to reproduce:
 ```
 cargo r -- install -s "release/v4.4
 ```
 Results in:

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
 - Windows Xtensa Rust installation is not working properly.

