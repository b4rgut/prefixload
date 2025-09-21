use crate::clients::s3::{S3Client, S3ClientOptions};
use crate::config::Config;
use crate::crypto::etag::calculate_s3_etag;
use crate::error::{PrefixloadError, Result};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Instant;

/// A simple logger that writes to stdout or a file, depending on the `quiet` flag.
struct Logger {
    file: Option<File>,
}

impl Logger {
    /// Creates a new logger. If `quiet` is true, it logs to a file in the
    /// platform-specific local data directory. Otherwise, it logs to stdout.
    fn new(quiet: bool) -> Result<Self> {
        if quiet {
            let mut log_path = dirs_next::data_local_dir().ok_or_else(|| {
                PrefixloadError::Custom("Could not find local data directory.".to_string())
            })?;
            log_path.push("prefixload");
            fs::create_dir_all(&log_path)?;
            log_path.push("run.log");

            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_path)
                .map_err(|e| {
                    PrefixloadError::Custom(format!(
                        "Failed to open log file at {}: {}",
                        log_path.display(),
                        e
                    ))
                })?;
            Ok(Logger { file: Some(file) })
        } else {
            Ok(Logger { file: None })
        }
    }

    /// Logs a message to the configured destination (stdout or file).
    fn log(&mut self, message: &str) {
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
        let formatted_message = format!("[{}] {}", timestamp, message);

        if let Some(file) = &mut self.file {
            // Errors are ignored here; we can't do much if logging fails.
            writeln!(file, "{}", formatted_message).ok();
        } else {
            println!("{}", formatted_message);
        }
    }
}

/// Scans the specified directory and returns a list of all files found within it.
/// This function is not recursive.
///
/// # Arguments
///
/// * `dir_path` - The path to the directory to scan.
///
/// # Returns
///
/// A `Result` containing a vector of `PathBuf`s for each file, or a `PrefixloadError`.
fn get_local_files(dir_path: &Path) -> Result<Vec<PathBuf>> {
    if !dir_path.is_dir() {
        return Err(PrefixloadError::Custom(format!(
            "Local directory path is not a valid directory: {}",
            dir_path.display()
        )));
    }

    let mut files = Vec::new();
    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(path);
        }
    }
    Ok(files)
}

/// The main entry point for the `run` command.
///
/// This function orchestrates the entire backup process:
/// 1. Initializes logging and timers.
/// 2. Loads configuration and S3 credentials.
/// 3. Scans the local directory for files.
/// 4. For each file matching a prefix rule, it calculates its ETag.
/// 5. It checks if the file is already synced to S3.
/// 6. If not synced, it uploads the file.
/// 7. Finally, it reports a summary of the operation.
pub async fn run(quiet: bool) -> Result<String> {
    let start_time = Instant::now();
    let mut logger = Logger::new(quiet)?;

    logger.log("Starting prefixload run...");

    let config = Config::load()?;

    let s3_options = S3ClientOptions::from_aws_config()
        .await?
        .with_endpoint(config.endpoint.clone())
        .with_region(config.region.clone())
        .with_force_path_style(config.force_path_style);

    let s3_client = S3Client::new(s3_options).await?;

    // Process files
    logger.log(&format!(
        "Scanning for files in: {}",
        config.local_directory_path.display()
    ));
    let local_files = get_local_files(&config.local_directory_path)?;
    logger.log(&format!("Found {} files to process.", local_files.len()));

    let mut uploaded_count = 0;
    let mut skipped_count = 0;
    let mut matched_count = 0;

    for file_path in &local_files {
        let file_name = match file_path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name,
            None => {
                logger.log(&format!(
                    "Skipping invalid file path: {}",
                    file_path.display()
                ));
                continue;
            }
        };

        for rule in &config.directory_struct {
            if file_name.starts_with(&rule.local_name_prefix) {
                matched_count += 1;
                logger.log(&format!("Processing matched file: {}", file_path.display()));

                let etag = calculate_s3_etag(&file_path, config.part_size)?;

                // Construct remote path
                let remote_key = Path::new(&rule.remote_path)
                    .join(file_name)
                    .to_string_lossy()
                    .to_string();

                let is_synced = s3_client
                    .is_object_synced(&etag, &config.bucket, &remote_key)
                    .await?;

                if is_synced {
                    logger.log(&format!(
                        "  - Object <{}> is already synced. Skipping upload.",
                        file_name
                    ));
                    skipped_count += 1;
                } else {
                    logger.log(&format!(
                        "  - Object <{}> is not synced. Uploading...",
                        file_name
                    ));
                    s3_client
                        .upload_file(&config.bucket, &remote_key, file_path)
                        .await?;
                    logger.log(&format!("  - Upload of <{}> complete.", file_name));
                    uploaded_count += 1;
                }
                // Found a matching rule, no need to check other rules for this file
                break;
            }
        }
    }

    let duration = start_time.elapsed();
    let final_message = format!(
        "Run finished in {:.2}s. Matched: {}, Uploaded: {}, Skipped: {}.",
        duration.as_secs_f32(),
        matched_count,
        uploaded_count,
        skipped_count
    );

    // If not in quiet mode, the final message is the function's Ok result.
    // If in quiet mode, the output is empty as it's all in the log file.
    if quiet {
        logger.log(&final_message);
        Ok("".to_string())
    } else {
        Ok(final_message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, DirectoryEntry};
    use serial_test::serial;
    use std::env;
    use tempfile::{TempDir, tempdir};
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    // Environment variable helpers
    #[cfg(windows)]
    const CONFIG_ENV: &str = "APPDATA";
    #[cfg(not(windows))]
    const CONFIG_ENV: &str = "XDG_CONFIG_HOME";

    #[cfg(windows)]
    const DATA_LOCAL_ENV: &str = "LOCALAPPDATA";
    #[cfg(not(windows))]
    const DATA_LOCAL_ENV: &str = "XDG_DATA_HOME";

    #[cfg(windows)]
    const HOME_ENV: &str = "USERPROFILE";
    #[cfg(not(windows))]
    const HOME_ENV: &str = "HOME";

    /// A test harness to sandbox the filesystem and network.
    struct TestHarness {
        pub config: Config,
        pub server: MockServer,
        // Temp Dirs are held to prevent them from being dropped and deleted
        _config_dir: TempDir,
        _data_dir: TempDir,
        _home_dir: TempDir,
        pub local_files_dir: TempDir,
    }

    /// Sets up a fully sandboxed test environment.
    async fn setup(rules: Vec<DirectoryEntry>, part_size: u64) -> TestHarness {
        let server = MockServer::start().await;
        let config_dir = tempdir().unwrap();
        let data_dir = tempdir().unwrap();
        let home_dir = tempdir().unwrap();
        let local_files_dir = tempdir().unwrap();

        // Set env vars to point to our temp dirs
        unsafe {
            env::set_var(CONFIG_ENV, config_dir.path());
            env::set_var(DATA_LOCAL_ENV, data_dir.path());
            env::set_var(HOME_ENV, home_dir.path());
        }

        // Create dummy AWS credentials file
        let aws_dir = home_dir.path().join(".aws");
        fs::create_dir(&aws_dir).unwrap();
        fs::write(
            aws_dir.join("credentials"),
            "[default]\naws_access_key_id=TESTKEY\naws_secret_access_key=TESTSECRET",
        )
        .unwrap();

        // Create a config object pointing to our test setup
        let config = Config {
            endpoint: server.uri(),
            bucket: "test-bucket".to_string(),
            region: "us-east-1".to_string(),
            force_path_style: true,
            part_size,
            local_directory_path: local_files_dir.path().to_path_buf(),
            directory_struct: rules,
        };

        // Write the config file
        let config_path = config_dir.path().join("prefixload/config.yml");
        fs::create_dir_all(config_path.parent().unwrap()).unwrap();
        fs::write(config_path, serde_yaml::to_string(&config).unwrap()).unwrap();

        TestHarness {
            config,
            server,
            _config_dir: config_dir,
            _data_dir: data_dir,
            _home_dir: home_dir,
            local_files_dir,
        }
    }

    /// Helper to create a temporary file with content.
    fn create_temp_file(dir: &Path, name: &str, content: &[u8]) -> PathBuf {
        let file_path = dir.join(name);
        let mut file = File::create(&file_path).unwrap();
        file.write_all(content).unwrap();
        file_path
    }

    #[test]
    fn test_get_local_files() {
        let dir = tempdir().unwrap();
        create_temp_file(dir.path(), "file1.txt", b"hello");
        create_temp_file(dir.path(), "file2.txt", b"world");
        fs::create_dir(dir.path().join("subdir")).unwrap();

        let files = get_local_files(dir.path()).unwrap();
        assert_eq!(files.len(), 2);
        assert!(files.iter().any(|p| p.ends_with("file1.txt")));
        assert!(files.iter().any(|p| p.ends_with("file2.txt")));
    }

    #[test]
    fn test_get_local_files_invalid_dir() {
        let result = get_local_files(Path::new("/non/existent/dir"));
        assert!(result.is_err());
    }

    #[test]
    #[serial]
    fn test_logger_quiet_mode() {
        let data_dir = tempdir().unwrap();
        unsafe {
            env::set_var(DATA_LOCAL_ENV, data_dir.path());
        }

        let mut logger = Logger::new(true).unwrap();
        logger.log("test message");

        let log_file_path = data_dir.path().join("prefixload/run.log");
        assert!(log_file_path.exists());
        let log_content = fs::read_to_string(log_file_path).unwrap();
        assert!(log_content.contains("test message"));

        unsafe {
            env::remove_var(DATA_LOCAL_ENV);
        }
    }

    #[tokio::test]
    #[serial]
    async fn test_run_uploads_new_file() {
        let harness = setup(
            vec![DirectoryEntry {
                local_name_prefix: "backup_".to_string(),
                remote_path: "backups".to_string(),
            }],
            5 * 1024 * 1024, // 5MB
        )
        .await;

        let file_content = b"this is a new backup";
        create_temp_file(harness.local_files_dir.path(), "backup_1.txt", file_content);

        // Mock S3: is_object_synced returns 404, upload returns 200
        Mock::given(method("HEAD"))
            .and(path("/test-bucket/backups/backup_1.txt"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&harness.server)
            .await;
        Mock::given(method("PUT"))
            .and(path("/test-bucket/backups/backup_1.txt"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&harness.server)
            .await;

        let result = run(false).await.unwrap();
        assert!(result.contains("Matched: 1, Uploaded: 1, Skipped: 0"));
    }

    #[tokio::test]
    #[serial]
    async fn test_run_skips_synced_file() {
        let harness = setup(
            vec![DirectoryEntry {
                local_name_prefix: "db_".to_string(),
                remote_path: "db".to_string(),
            }],
            5 * 1024 * 1024,
        )
        .await;

        let file_content = b"this is a synced backup";
        let file_path = create_temp_file(harness.local_files_dir.path(), "db_1.sql", file_content);
        let etag = calculate_s3_etag(file_path, harness.config.part_size).unwrap();

        // Mock S3: is_object_synced returns 200 with matching ETag
        Mock::given(method("HEAD"))
            .and(path("/test-bucket/db/db_1.sql"))
            .respond_with(ResponseTemplate::new(200).insert_header("ETag", format!("\"{}\"", etag)))
            .mount(&harness.server)
            .await;

        let result = run(false).await.unwrap();
        assert!(result.contains("Matched: 1, Uploaded: 0, Skipped: 1"));
    }

    #[tokio::test]
    #[serial]
    async fn test_run_ignores_unmatched_file() {
        let harness = setup(
            vec![DirectoryEntry {
                local_name_prefix: "backup_".to_string(),
                remote_path: "backups".to_string(),
            }],
            5 * 1024 * 1024,
        )
        .await;

        create_temp_file(
            harness.local_files_dir.path(),
            "some_other_file.txt",
            b"ignore me",
        );

        // No mocks needed as no S3 calls should be made

        let result = run(false).await.unwrap();
        assert!(result.contains("Matched: 0, Uploaded: 0, Skipped: 0"));
    }

    #[tokio::test]
    #[serial]
    async fn test_run_quiet_mode_logs_to_file() {
        let harness = setup(
            vec![DirectoryEntry {
                local_name_prefix: "backup_".to_string(),
                remote_path: "backups".to_string(),
            }],
            5 * 1024 * 1024,
        )
        .await;

        create_temp_file(harness.local_files_dir.path(), "backup_1.txt", b"content");

        // Mock S3 to simulate an upload
        Mock::given(method("HEAD"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&harness.server)
            .await;
        Mock::given(method("PUT"))
            .respond_with(ResponseTemplate::new(200))
            .mount(&harness.server)
            .await;

        // Run in quiet mode
        let result = run(true).await.unwrap();
        assert_eq!(result, ""); // Should return an empty string

        // Check the log file
        let log_file_path = harness._data_dir.path().join("prefixload/run.log");
        assert!(log_file_path.exists());
        let log_content = fs::read_to_string(log_file_path).unwrap();

        assert!(log_content.contains("Starting prefixload run"));
        assert!(log_content.contains("Processing matched file"));
        assert!(log_content.contains("Object <backup_1.txt> is not synced. Uploading"));
        assert!(log_content.contains("Run finished"));
        assert!(log_content.contains("Matched: 1, Uploaded: 1, Skipped: 0"));
    }
}
