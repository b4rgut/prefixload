use crate::clients::s3::{S3Client, S3ClientOptions};
use crate::config::Config;
use crate::error::{PrefixloadError, Result};
use requestty::Question;

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
        .map(ToOwned::to_owned)
        .ok_or_else(|| PrefixloadError::Custom("Failed to parse access key.".to_string()))?;

    let secret_key = requestty::prompt_one(secret_question)?
        .as_string()
        .map(ToOwned::to_owned)
        .ok_or_else(|| PrefixloadError::Custom("Failed to parse secret key.".to_string()))?;

    Ok((access_key, secret_key))
}

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

fn store_credentials(access_key: &str, secret_key: &str) -> Result<()> {
    println!(
        "Storing:\n  access = {}\n  secret = {}",
        access_key, secret_key
    );
    Ok(())
}

pub async fn run() -> Result<String> {
    let (access_key, secret_key) = input_credentials()?;

    match credentials_valid(&access_key, &secret_key).await {
        Ok(()) => {
            store_credentials(&access_key, &secret_key)?;
            Ok("Credentials have been saved successfully!".to_string())
        }
        Err(err) => Err(PrefixloadError::Custom(format!(
            "Credentials not valid: {}",
            err
        ))),
    }
}
