use crate::cli::ConfigCommand;
use crate::error::Result;

#[warn(unused_variables)]
pub fn run(cmd: ConfigCommand) -> Result<String> {
    Ok(format!("runing command ->"))
}
