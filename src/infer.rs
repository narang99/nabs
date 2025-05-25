use std::collections::HashSet;

use anyhow::{Context, Result, bail};

use crate::graph::{Target, TargetGraph};

// this is what we use to infer `Target` from
// its the underlying physical representation of a target
// as an example, this could be a disk based target
// or a hashmap based target (in unit tests)
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct RawTarget {
    pub name: String,
}

#[derive(Debug)]
pub struct Single {
    target: Target,
    parents: Vec<RawTarget>,
}

#[derive(Debug)]
pub enum InferredTarget {
    One(Single),
    Many(Vec<Single>),
    Nothing,
}

#[derive(Clone, Copy, Debug)]
pub enum Next {
    Continue,
    Break,
}

#[derive(Debug)]
pub struct InferResult {
    pub it: InferredTarget,
    pub what_next: Next,
}

pub trait Infer {
    // given a target, an infer would return us the true inferred target
    // and a list of raw targets of parent dependencies
    // or no target
    // - a single inferrer can return multiple targets
    // - multiple inferrers can return single targets
    // - an inferrer can break early
    // - an inferrer can give no target

    // - nabs.json inferrer: returns multiple targets and requires short-circuiting
    // - running cargo inferrer on poetry gives None
    // - a project containing both cargo.toml and requirements.txt returns 2 targets (1 target per inferrer)
    // - We want to differentiate between allowed multiple targets and unintended multiple targets
    // note that this function is not pure, it would basically do IO right now
    // we could abstract that away, but is that necessary?

    fn from_raw_target(&self, t: &RawTarget) -> Result<InferResult>;
}

pub struct InferRunner {
    infers: Vec<Box<dyn Infer>>,
}

impl InferRunner {
    pub fn new(infers: Vec<Box<dyn Infer>>) -> Self {
        InferRunner { infers }
    }

    // given a set of raw targets to start from
    // this would build a graph where it would link parents successively
    // a raw target can actually be associated with multiple targets
    // what do we do in that case?
    // an example is returning {name: a, flavor: cargo}, {name: a, flavor: poetry}
    // in this case, flavor should match of the returned parent dep
    pub fn build_graph(&self, start: Vec<RawTarget>) -> Result<TargetGraph> {
        let mut g = TargetGraph::new();
        for s in start {
            self.build_graph_rec(&mut g, &s)?;
        }
        Ok(g)
    }

    // we are given a raw target
    // we get the parents
    // add them to graph
    // then add us
    // we also need to add an edge for us and them
    // so we need to return the nodes that resulted from us being inserted
    // there can be multiple nodes
    fn build_graph_rec(&self, g: &mut TargetGraph, raw: &RawTarget) -> Result<Vec<Target>> {
        let our_inferred_targets = self.run_inf(raw)?;
        for our in our_inferred_targets.iter() {
            // for one of our targets, we need to build graph of parents
            if g.contains_node(&our.target) {
                continue;
            }
            g.add_node(our.target.clone());
            for p in &our.parents {
                let parent_targets = self.build_graph_rec(g, p)?;
                for pt in parent_targets {
                    g.add_edge(&pt, &our.target).expect(
                        &format!("unexpected corruption, failed in adding edge for {:?} and {:?} even though they should be in the graph", p, our.target)
                    );
                }
            }
        }
        Ok(our_inferred_targets.into_iter().map(|i| i.target).collect())
    }

    fn run_inf(&self, raw: &RawTarget) -> Result<Vec<Single>> {
        // a single infer can return 0, 1 or more targets
        // we run multiple infers in a list
        // it is invalid for multiple infers to return anything other than 0
        // that is there should be only one infer which wins, it either returns 1 target or many targets
        // an infer can also say if we want to infer more after giving some result
        // the first infer which directly reads nabs.json simply asks us to break if it finds any target
        // basically, if you want to make sure nobody infers after you, you return break and its guaranteed that your infer would work
        let mut inferred_targets = Vec::new();
        for inf in &self.infers {
            let inf_res = inf
                .from_raw_target(raw)
                .context("failed in building graph of targets")?;
            if let InferredTarget::Nothing = inf_res.it {
                // nothing, just want the else part
            } else {
                inferred_targets.push(inf_res.it);
            }

            match inf_res.what_next {
                Next::Break => {
                    break;
                }
                Next::Continue => {}
            };
        }

        // TODO: if i want people to use this application
        // will need to return concrete error types here so that we can handle this at top level and show a good message
        if inferred_targets.len() > 1 {
            bail!(
                "err: found a target where multiple build systems were inferred. this is not allowed for automatic inference. You can add entries manually for each target in nabs.json for the package, package_path={:} inferred_targets={:?}",
                raw.name,
                inferred_targets
            );
        }
        if inferred_targets.len() == 0 {
            bail!(
                "err: could not infer any target for package={}, add it manually in nabs.json",
                raw.name
            );
        }

        let t = std::mem::replace(&mut inferred_targets[0], InferredTarget::Nothing);
        match t {
            InferredTarget::Nothing => {
                panic!(
                    "inferred_targets is a list with only `Nothing` inside, this is impossible, package={}",
                    raw.name
                );
            }
            InferredTarget::One(s) => Ok(vec![s]),
            InferredTarget::Many(m) => Ok(m),
        }
    }
}

#[cfg(test)]
mod test {
    // this is going to be slightly complicated, ill need to create test structs inferrers also
    // these would actually be pretty simple structs
    // given an instantiated struct, they just be a list of what the dependencies are, we pass the map in instantiation

    use std::collections::HashMap;

    use crate::{graph::Target, infer::InferRunner};

    use super::{Infer, InferResult, InferredTarget, Next, RawTarget, Single};

    struct Dep {
        ps: Vec<RawTarget>,
        flavors: Vec<String>,
    }

    struct MockInfer {
        n_by_deps: HashMap<String, Dep>,
        what_next: Next,
    }

    impl Infer for MockInfer {
        fn from_raw_target(&self, t: &super::RawTarget) -> anyhow::Result<super::InferResult> {
            let deps = self.n_by_deps.get(&t.name);
            match deps {
                None => Ok(InferResult {
                    it: InferredTarget::Nothing,
                    what_next: Next::Continue,
                }),
                Some(deps) => {
                    if deps.flavors.len() == 1 {
                        Ok(InferResult {
                            it: InferredTarget::One(Single {
                                target: Target::new(t.name.clone(), deps.flavors[0].clone()),
                                parents: deps.ps.clone(),
                            }),
                            what_next: Next::Continue,
                        })
                    } else {
                        let targets = deps
                            .flavors
                            .iter()
                            .map(|f| Single {
                                target: Target::new(t.name.clone(), f.clone()),
                                parents: deps.ps.clone(),
                            })
                            .collect();

                        Ok(InferResult {
                            it: InferredTarget::Many(targets),
                            what_next: Next::Break,
                        })
                    }
                }
            }
        }
    }

    #[test]
    fn test_runner() {
        let infs: Vec<Box<dyn Infer>> = vec![
            Box::new(get_infer_1()), Box::new(get_infer_2())
        ];
        let runner = InferRunner::new(infs);
        let start = vec![RawTarget {
            name: "qureapi".to_string(),
        }];
        let graph = runner.build_graph(start).unwrap();
        println!("{}", graph);

        // println!("{:?}", infs[1].from_raw_target(&RawTarget { name: "qsync_stream".to_string() }).unwrap());
        let start = vec![RawTarget {
            name: "image_manager".to_string(),
        }];
        let graph = runner.build_graph(start).unwrap();
        println!("{}", graph);
    }

    fn get_infer_2() -> MockInfer {
        let mut n_by_deps: HashMap<String, Dep> = HashMap::new();
        n_by_deps.insert(
            "image_manager".to_string(),
            Dep {
                ps: vec![RawTarget {
                    name: "qsync_stream".to_string(),
                }],
                flavors: vec!["python".to_string()],
            },
        );
        n_by_deps.insert(
            "qsync_stream".to_string(),
            Dep {
                ps: vec![],
                flavors: vec!["python".to_string(), "cargo".to_string()],
            },
        );
        MockInfer {
            n_by_deps,
            what_next: Next::Continue,
        }
    }

    fn get_infer_1() -> MockInfer {
        let mut n_by_deps = HashMap::new();
        n_by_deps = HashMap::from([
            // (
            //     "image_manager".to_string(),
            //     vec![RawTarget {
            //         name: "qsync_stream".to_string(),
            //     }],
            // ),
            // ("qsync_stream".to_string(), vec![]),
            (
                "qureapi".to_string(),
                vec![
                    RawTarget {
                        name: "qxr".to_string(),
                    },
                    RawTarget {
                        name: "qer".to_string(),
                    },
                    RawTarget {
                        name: "qer_reports".to_string(),
                    },
                ],
            ),
            (
                "cathode".to_string(),
                vec![
                    RawTarget {
                        name: "qxr".to_string(),
                    },
                    RawTarget {
                        name: "qxr_reports".to_string(),
                    },
                ],
            ),
            (
                "qxr".to_string(),
                vec![RawTarget {
                    name: "qure_dicom_utils".to_string(),
                }],
            ),
            (
                "qxr_reports".to_string(),
                vec![RawTarget {
                    name: "qxr".to_string(),
                }],
            ),
            (
                "qer".to_string(),
                vec![RawTarget {
                    name: "qure_dicom_utils".to_string(),
                }],
            ),
            (
                "qer_reports".to_string(),
                vec![RawTarget {
                    name: "qer".to_string(),
                }],
            ),
            ("qure_dicom_utils".to_string(), vec![]),
        ]);
        let n_by_deps = n_by_deps
            .into_iter()
            .map(|(k, v)| {
                (
                    k,
                    Dep {
                        ps: v,
                        flavors: vec!["cargo".to_string()],
                    },
                )
            })
            .collect();
        MockInfer {
            n_by_deps,
            what_next: Next::Continue,
        }
    }
}
