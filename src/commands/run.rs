use crate::error::Result;

pub fn run(quiet: bool) -> Result<String> {
    Ok(format!("Running with quiet mode: {}", quiet))
}
