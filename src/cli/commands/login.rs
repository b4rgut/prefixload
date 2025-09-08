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
        .unwrap()
        .to_owned();
    let secret_key = requestty::prompt_one(secret_question)?
        .as_string()
        .unwrap()
        .to_owned();

    Ok((access_key, secret_key))
}

async fn credentials_valid(access_key: &str, secret_key: &str) -> Result<bool> {
    let config: Config = Config::load()?;

    let s3_options = S3ClientOptions::default()
        .with_access_key(access_key.to_string())
        .with_secret_key(secret_key.to_string())
        .with_endpoint(config.endpoint.clone());

    let s3_client = S3Client::new(s3_options).await.unwrap();

    let result = s3_client.check_bucket_access(&config.bucket).await;

    Ok(result.is_ok())
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

    if credentials_valid(&access_key, &secret_key).await? {
        store_credentials(&access_key, &secret_key)?;
        return Ok("Credentials have been saved successfully!".to_string());
    }

    Err(PrefixloadError::Custom("Credentials not valid".to_string()))
}
