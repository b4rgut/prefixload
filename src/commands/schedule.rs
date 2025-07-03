use crate::error::Result;

pub fn run(cron: &str) -> Result<String> {
    Ok(format!("Cron expression: {}", cron))
}
