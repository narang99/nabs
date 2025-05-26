use clap::Parser;
use commands::{Commands, run_command};

mod commands;
mod graph;
mod infer;
mod paths;
mod types;

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn main() {
    pretty_env_logger::init();
    let cli = Cli::parse();
    match run_command(cli.command) {
        Err(e) => {
            println!("error: {}", e);
            std::process::exit(-1);
        }
        Ok(()) => {}
    }
}
