use crate::error::Result;
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

fn store_credentials(access_key: &str, secret_key: &str) -> Result<()> {
    println!(
        "Storing:\n  access = {}\n  secret = {}",
        access_key, secret_key
    );
    Ok(())
}

pub async fn run() -> Result<String> {
    let (access_key, secret_key) = input_credentials()?;

    store_credentials(&access_key, &secret_key)?;

    Ok("Credentials not stored".to_string())
}
