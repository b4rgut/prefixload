[![crates.io](https://img.shields.io/crates/v/prefixload.svg)](https://crates.io/crates/prefixload)
**🇺🇸 English | [🇷🇺 Русский](README_RU.md)**

# Prefixload

Prefixload is a small command-line utility written in Rust that periodically uploads files from a local directory to an Amazon S3 or S3-compatible bucket. It selects files for upload based on configurable filename prefixes.

This tool is useful for automating backups where files are organized and named with consistent prefixes (e.g., `db_backup_2025-09-21.sql`, `logs_myapp_2025-09-21.tar.gz`).

## Features

*   **Prefix-based Rules**: Configure rules in a YAML file to map file prefixes to specific remote directories in your S3 bucket.
*   **S3-Compatible**: Works with AWS S3 as well as other S3-compatible services like MinIO, Ceph, or Wasabi.
*   **Efficient Syncing**: Uses S3 ETags to check if a file is already synced, avoiding unnecessary re-uploads.
*   **Multipart Uploads**: Automatically handles large files using multipart uploads.
*   **Secure Credential Storage**: A `login` command helps you securely store your AWS credentials.

## Installation

You can install Prefixload directly from crates.io using Cargo:

```sh
cargo install prefixload
```

## Usage

The tool requires a one-time setup for credentials and configuration.

### 1. Login

First, configure your AWS credentials. The tool will prompt you for your Access Key ID and Secret Access Key and save them to the standard `~/.aws/credentials` file.

```sh
prefixload login
```

### 2. Configure

Next, set up your backup rules. The configuration is stored in a YAML file. To open it in your default editor, run:

```sh
prefixload config edit
```

This will open the configuration file where you can define your S3 endpoint, bucket, and prefix mapping rules.

### 3. Run a Backup

To perform a one-time backup based on your configuration, use the `run` command:

```sh
prefixload run
```

You can run it in quiet mode to suppress output and log to a file instead:
```sh
prefixload run --quiet
```

## Configuration

The configuration is located at `~/.config/prefixload/config.yml` (on Linux/macOS) or `%APPDATA%\prefixload\config.yml` (on Windows).

Here is an example of the `config.yml` file:

```yaml
# The endpoint URL of your S3-compatible storage
endpoint: "https://s3.example.com"

# The name of your S3 bucket to upload files to
bucket: "my-backup-bucket"

# The AWS region. Required by the SDK.
region: "us-east-1"

# Set to `true` for S3-compatible services that require path-style addressing (e.g., MinIO).
force_path_style: false

# The upload part size in bytes for multipart uploads (e.g., 15MB).
part_size: 15728640

# Path to the local directory where your files are stored.
local_directory_path: "/var/backups"

# Mapping rules for uploading files.
directory_struct:
  # Files starting with "db_backup_" will be uploaded to the "database/" directory in the bucket.
  - local_name_prefix: "db_backup_"
    remote_path: "database/"

  # Files starting with "app_logs_" will be uploaded to the "application_logs/" directory.
  - local_name_prefix: "app_logs_"
    remote_path: "application_logs/"
```
