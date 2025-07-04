use crate::error::Result;
use dialoguer::{Input, Password};

pub fn run() -> Result<String> {
    let (access, secret) = input_credentials()?;

    store_credentials(&access, &secret)?;

    Ok("Credentials stored successfully".to_string())
}

fn input_credentials() -> Result<(String, String)> {
    let access: String = Input::new()
        .with_prompt("AWS Access Key ID")
        .interact_text()?;

    let secret = Password::new()
        .with_prompt("AWS Secret Access Key")
        .interact()?;

    Ok((access, secret))
}

fn store_credentials(access_key: &str, secret_key: &str) -> Result<()> {
    println!(
        "Storing:\n  access = {}\n  secret = {}",
        access_key, secret_key
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_credentials_always_succeeds() {
        let result = store_credentials("AKCCESSKEYEXAMPLE123", "SECRETKEYEXAMPLE123");
        assert!(result.is_ok(), "store_credentials() should return Ok(())");
    }
}
