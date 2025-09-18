// This module handles the logic for the `login` command, which allows users to
// authenticate with their AWS credentials and save them for future use.

use crate::clients::s3::{S3Client, S3ClientOptions};
use crate::config::Config;
use crate::error::{PrefixloadError, Result};
use configparser::ini::Ini;
use requestty::Question;
use std::fs;

// Conditionally import Unix-specific modules for file permissions.
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// Prompts the user to interactively enter their AWS access key and secret key.
///
/// # Returns
///
/// A `Result` containing a tuple with the `(access_key, secret_key)` if successful,
/// or a `PrefixloadError` if input parsing fails.
fn input_credentials() -> Result<(String, String)> {
    let access_question = Question::input("access_key")
        .message("Enter AWS Access Key ID:")
        .build();

    let secret_question = Question::password("secret_key")
        .message("Enter AWS Secret Access Key:")
        .mask('*')
        .build();

    let access_key = requestty::prompt_one(access_question)?
        .as_string()
        .map(String::from)
        .ok_or_else(|| PrefixloadError::Custom("Failed to parse access key.".to_string()))?;

    let secret_key = requestty::prompt_one(secret_question)?
        .as_string()
        .map(String::from)
        .ok_or_else(|| PrefixloadError::Custom("Failed to parse secret key.".to_string()))?;

    Ok((access_key, secret_key))
}

/// Validates the provided AWS credentials by attempting to access the configured S3 bucket.
///
/// # Arguments
///
/// * `access_key` - The AWS Access Key ID.
/// * `secret_key` - The AWS Secret Access Key.
///
/// # Returns
///
/// An empty `Result` (`Ok(())`) if the credentials are valid and the bucket is accessible,
/// otherwise a `PrefixloadError`.
async fn credentials_valid(access_key: &str, secret_key: &str) -> Result<()> {
    let config: Config = Config::load()?;

    let s3_options = S3ClientOptions::default()
        .with_access_key(access_key.to_string())
        .with_secret_key(secret_key.to_string())
        .with_endpoint(config.endpoint.clone());

    let s3_client = S3Client::new(s3_options).await?;

    s3_client.check_bucket_access(&config.bucket).await?;

    Ok(())
}

/// Saves the AWS credentials to the standard `~/.aws/credentials` file.
///
/// On Unix-like systems, it sets the file permissions to `0o600` for security.
///
/// # Arguments
///
/// * `access_key` - The AWS Access Key ID to save.
/// * `secret_key` - The AWS Secret Access Key to save.
///
/// # Returns
///
/// An empty `Result` (`Ok(())`) if saving is successful, otherwise a `PrefixloadError`.
fn save_credentials_to_file(access_key: &str, secret_key: &str) -> Result<()> {
    let home_dir = dirs_next::home_dir()
        .ok_or_else(|| PrefixloadError::Custom("Cannot find home directory".to_string()))?;

    let aws_dir = home_dir.join(".aws");

    fs::create_dir_all(&aws_dir).map_err(|err| {
        PrefixloadError::Custom(format!("Cannot create {}: {}", aws_dir.display(), err))
    })?;

    let credentials_path = aws_dir.join("credentials");
    let mut config = Ini::new();

    config.set("default", "aws_access_key_id", Some(access_key.to_string()));
    config.set(
        "default",
        "aws_secret_access_key",
        Some(secret_key.to_string()),
    );

    config.write(&credentials_path).map_err(|err| {
        PrefixloadError::Custom(format!(
            "Cannot write to {}: {}",
            credentials_path.display(),
            err
        ))
    })?;

    // On Unix, set file permissions to `0o600` (read/write for owner only).
    #[cfg(unix)]
    {
        let mut perms = fs::metadata(&credentials_path)?.permissions();
        perms.set_mode(0o600);
        fs::set_permissions(&credentials_path, perms)?;
    }

    Ok(())
}

/// The main entry point for the `login` command.
///
/// It orchestrates the process of getting, validating, and saving credentials.
///
/// # Returns
///
/// A `Result` containing a success message string or a `PrefixloadError`.
pub async fn run() -> Result<String> {
    // Get credentials from user input.
    let (access_key, secret_key) = input_credentials()?;

    // Validate the credentials.
    match credentials_valid(&access_key, &secret_key).await {
        Ok(()) => {
            // If valid, save them to the file.
            save_credentials_to_file(&access_key, &secret_key)?;
            Ok("Credentials have been saved successfully!".to_string())
        }
        Err(err) => Err(PrefixloadError::Custom(format!(
            "Credentials not valid: {}",
            err
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    #[serial]
    fn test_save_credentials_to_file() {
        let dir = tempdir().unwrap();
        // By setting the HOME environment variable, we can control where `dirs_next::home_dir()` looks.
        // This allows us to use a temporary directory for our test credentials file.
        unsafe {
            std::env::set_var("HOME", dir.path());
        }

        let access_key = "test_access_key";
        let secret_key = "test_secret_key";

        // Execute the function to save credentials
        let result = save_credentials_to_file(access_key, secret_key);
        assert!(result.is_ok());

        // Verify the credentials file was created and has the correct content
        let credentials_path = dir.path().join(".aws").join("credentials");
        assert!(credentials_path.exists());

        let content = fs::read_to_string(&credentials_path).unwrap();

        assert!(content.contains(&format!("aws_access_key_id={}", access_key)));
        assert!(content.contains(&format!("aws_secret_access_key={}", secret_key)));

        // On Unix, verify file permissions are set to 600
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let perms = fs::metadata(&credentials_path).unwrap().permissions();
            assert_eq!(perms.mode() & 0o777, 0o600);
        }

        // Clean up the environment variable
        unsafe {
            std::env::remove_var("HOME");
        }
    }
}
