use crate::error::Result;

pub async fn run(quiet: bool) -> Result<String> {
    Ok(format!("I async run command with quiet mode: {}", quiet))
}
