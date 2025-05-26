use std::{collections::HashSet, path::PathBuf, rc::Rc};

use anyhow::{anyhow, Result};
use log::{debug, info};

use crate::{infer::InferRunner, types::{Monorepo, RawTarget, Repository}};

pub fn print_graph() -> Result<()> {
    let monorepo: Rc<dyn Repository> = Rc::new(Monorepo::new()?);
    let pkgs: HashSet<PathBuf> = HashSet::from_iter(monorepo.get_nabs_packages());
    info!("all detected packages, {:?}", monorepo.get_nabs_packages());

    let runner = InferRunner::default(&monorepo);
    let targets = to_raw_targets(&pkgs)?;
    debug!("created raw targets {:?}", targets);
    let (graph, _) = runner.build_graph(targets)?;

    println!("graph:\n{}", graph);
    Ok(())
}


// todo: remove this duplication, same code inside changeset also
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