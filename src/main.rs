use std::{collections::HashSet, path::PathBuf, rc::Rc};

use anyhow::{Result, anyhow, bail};
use clap::Parser;
use infer::InferRunner;
use types::{Monorepo, RawTarget, Repository, Target, TargetName};

pub mod graph;
pub mod infer;
pub mod paths;
pub mod types;

#[derive(clap::Subcommand)]
enum Commands {
    Changeset,
}

#[derive(clap::Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None => {
            println!("empty command not allowed");
        }
        Some(c) => match c {
            Commands::Changeset => get_changeset().unwrap(),
        },
    };
}

fn get_changeset() -> Result<()> {
    use std::io::{self, Read};

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let files_to_find_diff: Vec<PathBuf> = input
        .trim()
        .split_whitespace()
        .map(|p| PathBuf::from(p))
        .collect();

    let monorepo = Monorepo::new()?;
    let pkgs = monorepo.get_nabs_packages();
    let pkgs: HashSet<PathBuf> = HashSet::from_iter(pkgs);
    let to_search = get_pkgs_to_search(&files_to_find_diff, &pkgs)?;
    let monorepo: Rc<dyn Repository> = Rc::new(monorepo);

    let runner = InferRunner::default(&monorepo);
    let mut targets = Vec::new();
    for p in &pkgs {
        let val = p
            .to_str()
            .ok_or(anyhow!("could not parse path: {:?}", p))?
            .to_string();
        let val = RawTarget::from_string_name(val)?;
        targets.push(val);
    }
    let (graph, our_targets) = runner.build_graph(targets)?;
    let our_targets: Vec<Target> = our_targets
        .into_iter()
        .filter(|t| to_search.contains(&t.name))
        .collect();
    let result = graph.rdeps(&our_targets)?;
    for target in result {
        println!("{}", target.name_as_string_ref());
    }
    Ok(())
}

fn get_pkgs_to_search(
    files_to_find_diff: &Vec<PathBuf>,
    pkgs: &HashSet<PathBuf>,
) -> Result<HashSet<TargetName>> {
    let mut pkgs_to_search = HashSet::new();
    for f in files_to_find_diff {
        let pkg = which_pkg(&f, &pkgs);
        match pkg {
            None => {
                bail!(
                    "could not find nabs package for file={}",
                    f.to_string_lossy()
                );
            }
            Some(v) => {
                let v = v
                    .to_str()
                    .ok_or(anyhow!("could not parse package path: {:?}", v))?;
                pkgs_to_search.insert(TargetName::new(v.to_string())?);
            }
        }
    }
    Ok(pkgs_to_search)
}

fn which_pkg(p: &PathBuf, pkgs: &HashSet<PathBuf>) -> Option<PathBuf> {
    let mut cur = Some(p.clone());
    while let Some(p) = cur {
        if pkgs.contains(&p) {
            return Some(p.clone());
        }
        cur = p.parent().map(|v| PathBuf::from(v));
    }
    None
}
