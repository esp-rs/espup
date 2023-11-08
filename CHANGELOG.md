# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Added new `--esp-riscv-gcc` flag to install esp-riscv-gcc toolchain instead of the system one (#391)

### Fixed

### Changed
- New Default behavior: install esp-riscv-gcc only if the user explicitly uses the `--esp-riscv-gcc flag (#391)

### Removed

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

[Unreleased]: https://github.com/esp-rs/espup/compare/v0.8.0...HEAD
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
