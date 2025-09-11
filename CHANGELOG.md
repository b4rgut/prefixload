# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.1](https://github.com/b4rgut/prefixload/compare/v0.8.0...v0.8.1) - 2025-09-08

### Other

- update README.md

## [0.8.0](https://github.com/b4rgut/prefixload/compare/v0.7.1...v0.8.0) - 2025-09-08

### Added

- *(error)* add custom error variant for user-defined messages

### Fixed

- *(cli/login)* return Prefixload Custom error for login command
- *(cli/config)* return Prefixload Custom error for config command +

### Other

- *(cli)* move CLI and commands into dedicated src/cli module

## [0.4.0](https://github.com/b4rgut/prefixload/compare/v0.3.1...v0.4.0) - 2025-07-19

### Other

- *(config)* [**breaking**] hide `config_path` helper and enrich Rustdocs
- *(command/config)* fix test

## [0.3.1](https://github.com/b4rgut/prefixload/compare/v0.3.0...v0.3.1) - 2025-07-19

### Fixed

- delete an extra newline in the message output

## [0.3.0](https://github.com/b4rgut/prefixload/compare/v0.2.0...v0.3.0) - 2025-07-19

### Fixed

- delete an extra newline in the message output

### Other

- *(cli/config)* make `handle_config_show` side-effect-free

## [0.2.0](https://github.com/b4rgut/prefixload/compare/v0.1.1...v0.2.0) - 2025-07-19

### Added

- *(cli/config)* add full handler stack for `config` subcommands
- *(config)* implement YAML config management with embedded default

### Other

- *(config)* add unit tests for CLI config handlers
- *(config)* add comprehensive unit tests for Config handling

## [0.1.1](https://github.com/b4rgut/prefixload/compare/v0.1.0...v0.1.1) - 2025-07-18

### Added

- *(cli)* fill in the skeleton of the cli utility

## [0.1.0](https://github.com/b4rgut/prefixload/releases/tag/v0.1.0) - 2025-07-18

### Added

- initial commit

### Fixed

- *(ci)* github workflow
