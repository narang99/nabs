use anyhow::Result;
use changeset::get_changeset;

pub mod changeset;

#[derive(clap::Subcommand)]
pub enum Commands {
    Changeset,
}

pub fn run_command(command: Option<Commands>) -> Result<()> {
    match command {
        None => {
            println!("empty command not allowed");
            Ok(())
        }
        Some(c) => match c {
            Commands::Changeset => get_changeset(),
        },
    }
}
