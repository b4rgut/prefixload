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
                    logger.log(&format!("  - Object <{}> is not synced. Uploading...", file_name));
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
