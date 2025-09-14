# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.11.1](https://github.com/b4rgut/prefixload/compare/v0.11.0...v0.11.1) - 2025-09-14

### Added

- *(cli/commands/login)* improve error handling in login credentials
- *(cli/commands/config)* add fallback when syntax highlighting theme

### Fixed

- *(config)* unpredictable behavior related to I/O capture in tests
- *(config)* error handling in config file operations

### Other

- *(config)* add test for invalid YAML config loading

## [0.11.0](https://github.com/b4rgut/prefixload/compare/v0.10.0...v0.11.0) - 2025-09-14

### Added

- *(s3)* add S3 region and path style configuration options

### Fixed

- *(s3)* S3 bucket access check and improve tests

### Other

- *(S3)* add S3 auth header check to bucket access test

## [0.10.0](https://github.com/b4rgut/prefixload/compare/v0.9.0...v0.10.0) - 2025-09-13

### Added

- *(clients/s3)* accept any string type in S3ClientOptions builder

### Fixed

- *(clients/s3)* simplify S3 region configuration handling

### Other

- *(clients/s3)* add test for custom S3 region setting
- *(clients/s3)* add documentation to S3ClientOptions builder methods
- *(clients/s3)* remove comment for credentials lookup
- *(cli/commands/config)* add helper function to simplify config file

## [0.9.0](https://github.com/b4rgut/prefixload/compare/v0.8.1...v0.9.0) - 2025-09-13

### Fixed

- propagate config edit errors properly
- correct bucket field name and update config assignment

### Other

- src/config.rs
- extract config update logic into reusable function
- simplify config command handling by removing unnecessary Ok()
- change local_directory_path type from String to PathBuf
- [**breaking**] rename DirectoryEntry fields for clarity
- add field descriptions to ConfigSetArgs struct

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
