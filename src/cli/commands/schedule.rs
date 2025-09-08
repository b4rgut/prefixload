use crate::error::Result;

pub async fn run(cron: &str) -> Result<String> {
    Ok(format!("I am a Cron expression: {}", cron))
}
