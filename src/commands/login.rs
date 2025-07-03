use crate::error::Result;

pub fn run(access_key: &str) -> Result<String> {
    Ok(format!("Access key: {}", access_key))
}
