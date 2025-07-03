use clap::Parser;
use prefixload::cli::Cli;

fn main() {
    let cli = Cli::parse();

    match cli.run() {
        Ok(result) => println!("{}", result),
        Err(err) => {
            eprintln!("Error: {}", err);
            std::process::exit(1);
        }
    }
}
