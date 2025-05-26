use std::{collections::HashSet, io::Read, path::PathBuf, rc::Rc};

use crate::graph::TargetGraph;
use crate::infer::InferRunner;
use crate::types::{Monorepo, RawTarget, Repository, Target, TargetName};
use anyhow::{Context, Result, anyhow};
use log::{debug, info};

pub fn get_changeset() -> Result<()> {
    let files_to_find_diff = get_input()?;

    let monorepo: Rc<dyn Repository> = Rc::new(Monorepo::new()?);
    let pkgs: HashSet<PathBuf> = HashSet::from_iter(monorepo.get_nabs_packages());
    info!("all detected packages, {:?}", pkgs);

    let to_search = get_pkgs_to_search(&files_to_find_diff, &pkgs)?;
    info!("changed packages: {:?}", to_search);

    let runner = InferRunner::default(&monorepo);
    let targets = to_raw_targets(&pkgs)?;
    let (graph, our_targets) = build_graph_and_our_targets(&runner, targets, &to_search)?;
    let result = graph.rdeps(&our_targets)?;
    for target in result {
        println!("{}", target.name_as_string_ref());
    }
    Ok(())
}

fn build_graph_and_our_targets(
    runner: &InferRunner,
    targets: Vec<RawTarget>,
    to_search: &HashSet<TargetName>,
) -> Result<(TargetGraph, Vec<Target>)> {
    let (graph, our_targets) = runner.build_graph(targets)?;
    let our_targets: Vec<Target> = our_targets
        .into_iter()
        .filter(|t| to_search.contains(&t.name))
        .collect();
    validate_and_warn_on_missing_targets(&our_targets, to_search);
    Ok((graph, our_targets))
}

fn to_raw_targets(pkgs: &HashSet<PathBuf>) -> Result<Vec<RawTarget>> {
    let mut targets = Vec::new();
    for p in pkgs {
        let val = p
            .to_str()
            .ok_or(anyhow!("could not parse path: {:?}", p))?
            .to_string();
        let val = RawTarget::from_string_name(val)?;
        targets.push(val);
    }
    Ok(targets)
}

fn get_input() -> Result<Vec<PathBuf>> {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input)?;
    let files_to_find_diff: Vec<PathBuf> = input
        .trim()
        .split_whitespace()
        .map(|p| PathBuf::from(p))
        .collect();
    Ok(files_to_find_diff)
}

fn validate_and_warn_on_missing_targets(
    our_targets: &Vec<Target>,
    to_search: &HashSet<TargetName>,
) {
    if our_targets.len() != to_search.len() {
        let ts: HashSet<&TargetName> = our_targets.iter().map(|v| &v.name).collect();
        for t in to_search {
            if !ts.contains(t) {
                eprintln!("warn: file={} not part of any package", t);
            }
        }
    }
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
                eprintln!("file={} is not part of any package", f.to_string_lossy());
            }
            Some(v) => {
                let v_str = v
                    .to_str()
                    .ok_or(anyhow!("could not parse package path: {:?}", v))?;
                pkgs_to_search.insert(
                    TargetName::new(v_str.to_string())
                        .context(anyhow!("failed to create target for {:?}", v))?,
                );
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
