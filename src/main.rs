use clap::Parser;
use prefixload::cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match cli.run().await {
        Ok(result) => println!("{}", result),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}
