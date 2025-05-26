use anyhow::Result;
use changeset::get_changeset;
use graph::print_graph;

mod changeset;
mod graph;


#[derive(clap::Subcommand)]
pub enum Commands {
    Changeset,
    Graph,
}

pub fn run_command(command: Option<Commands>) -> Result<()> {
    match command {
        None => {
            eprintln!("empty command not allowed");
            Ok(())
        }
        Some(c) => match c {
            Commands::Changeset => get_changeset(),
            Commands::Graph => print_graph(),
        },
    }
}
