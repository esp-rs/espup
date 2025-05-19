# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Unreleased

### Changed
- Improved GitHub API error handling (#496)
- update `zip` dependency to 3.0.0

## [0.15.0] - 2025-04-08

### Changed
- Install `stable` Rust toolchain instead of `nightly` for RISC-V devices (#487)

## [0.14.1] - 2025-03-04

### Added
- Add support for LLVM esp-19.1.2_20250225 (#477, #479)

### Fixed
- Return an error if GET request fails (#471)
- Fix RISC-V installation error (#480)

## [0.14.0] - 2024-12-17

### Added
- Smoother large file download&proxy support (#463)
- Add GitHub API errors to clarify what failed (#464)

### Fixed
- When queriying GitHub for the list of releases, retrieve more items (#462)

### Changed
- `espup` now prints why an install step failed (#461)

## [0.13.0] - 2024-10-30

### Changed
- Update GCC version to 14.2.0 (#442)
- Update LLVM version to esp-18.1.2_20240912 (#452)

## [0.12.2] - 2024-07-18

### Fixed
- Fix extended LLVM mode regression for LLVM versions < 17 introduced by #432. (#437)

## [0.12.1] - 2024-07-15

### Fixed
- Make both `libclang.so` available again when installing the extended LLVM for LLVM versions >= 17 (#432)

## [0.12.0] - 2024-06-12

### Added
- Added support for SOCKS5 proxy (#423)

### Changed
- Update LLVM version to `esp-17.0.1_20240419` (#427)
- Update dependencies (#429)

## [0.11.0] - 2024-02-02

### Added
- Added support for specifying the location of the export file via `ESPUP_EXPORT_FILE` (#403)
- Added support for ESP32-P4 (#408)

### Fixed
- [Windows]: Avoid duplicating system environment variables into user environment variables (#411)

## [0.10.0]

### Fixed
- `skip-version-parse` argument should require `toolchain-version` (#396)
- If there is a minified LLVM installation, `--extended-llvm` now installs the full LLVM (#400)

### Changed
- Update LLVM version to `esp-16.0.4-20231113` (#398)

## [0.9.0] - 2023-11-10

### Added
- Added new `--esp-riscv-gcc` flag to install esp-riscv-gcc toolchain instead of the system one (#391)

### Changed
- New Default behavior: install esp-riscv-gcc only if the user explicitly uses the `--esp-riscv-gcc` flag (#391)

## [0.8.0] - 2023-11-02

### Added
- Add symlink to LLVM in Unix systems (#380)

### Changed
- Reduce logs verbosity, add docstrings, and use async methods (#384)
- Change how Windows environment is configured (#389)

## [0.7.0] - 2023-10-18

### Changed
- Update GCC version to 13.2 (#373)
- Update logging format and log messages (#375, #376)

## [0.6.1] - 2023-10-04

### Changed
- Remove unnecessary CI jobs (#369)

### Fixed
- Create $RUSTUP_HOME/tmp if needed (#365)
- Complete Xtensa Rust versions when provided one is incomplete (#366)

## [0.6.0] - 2023-10-02

### Added
- Add a flag to skip Xtensa Rust version parsing (#352)
- Add warn message when failed to detect Xtensa Rust (#357)

### Changed
- Update dependencies
- Use `RUSTUP_HOME` tmp folder (#348)
- Improve `remove_dir_all` errors (#346)

### Fixed
- Fix temorary folders/files cleanup (#344)
- Fix Clippy lint (#335)

### Removed

## [0.5.0] - 2023-08-11

## [0.4.1] - 2023-05-18

## [0.4.0] - 2023-04-24

## [0.3.2] - 2023-03-13

## [0.3.1] - 2023-03-06

## [0.3.0] - 2023-02-21

## [0.2.9] - 2023-02-14

## [0.2.8] - 2023-02-03

## [0.2.7] - 2023-02-03

## [0.2.6] - 2023-01-13

## [0.2.5] - 2023-01-11

## [0.2.4] - 2022-12-14

## [0.2.3] - 2022-11-17

## [0.2.2] - 2022-11-17

## [0.2.1] - 2022-11-04

## [0.2.0] - 2022-11-03

## [0.1.0] - 2022-10-07

[0.15.0]: https://github.com/esp-rs/espup/compare/v0.14.1...v0.15.0
[0.14.1]: https://github.com/esp-rs/espup/compare/v0.14.0...v0.14.1
[0.14.0]: https://github.com/esp-rs/espup/compare/v0.13.0...v0.14.0
[0.13.0]: https://github.com/esp-rs/espup/compare/v0.12.2...v0.13.0
[0.12.2]: https://github.com/esp-rs/espup/compare/v0.12.1...v0.12.2
[0.12.1]: https://github.com/esp-rs/espup/compare/v0.12.0...v0.12.1
[0.12.0]: https://github.com/esp-rs/espup/compare/v0.11.0...v0.12.0
[0.11.0]: https://github.com/esp-rs/espup/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/esp-rs/espup/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/esp-rs/espup/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/esp-rs/espup/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/esp-rs/espup/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/esp-rs/espup/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/esp-rs/espup/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/esp-rs/espup/compare/v0.4.1...v0.5.0
[0.4.1]: https://github.com/esp-rs/espup/compare/v0.4.0...v0.4.1
[0.4.0]: https://github.com/esp-rs/espup/compare/v0.3.2...v0.4.0
[0.3.2]: https://github.com/esp-rs/espup/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/esp-rs/espup/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/esp-rs/espup/compare/v0.2.9...v0.3.0
[0.2.9]: https://github.com/esp-rs/espup/compare/v0.2.8...v0.2.9
[0.2.8]: https://github.com/esp-rs/espup/compare/v0.2.7...v0.2.8
[0.2.7]: https://github.com/esp-rs/espup/compare/v0.2.6...v0.2.7
[0.2.6]: https://github.com/esp-rs/espup/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/esp-rs/espup/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/esp-rs/espup/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/esp-rs/espup/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/esp-rs/espup/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/esp-rs/espup/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/esp-rs/espup/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/esp-rs/espup/releases/tag/v0.1.0
