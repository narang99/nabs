mod core;
mod icargo;
mod py_requirements;

use core::{Infer, InferredTarget, Next, Single};
use std::rc::Rc;

use anyhow::{Context, Result, bail};
use icargo::CargoInfer;
use py_requirements::{DEFAULT_REQ_FILE_NAME, PyRequirementsInfer};

use crate::graph::TargetGraph;
use crate::types::{RawTarget, Repository, Target};

pub struct InferRunner {
    infers: Vec<Box<dyn Infer>>,
}

impl InferRunner {
    pub fn new(infers: Vec<Box<dyn Infer>>) -> Self {
        InferRunner { infers }
    }

    pub fn default(repo: &Rc<dyn Repository>) -> Self {
        InferRunner {
            infers: vec![
                Box::new(CargoInfer::new(Rc::clone(repo))),
                Box::new(PyRequirementsInfer::new(
                    Rc::clone(repo),
                    DEFAULT_REQ_FILE_NAME.to_string(),
                )),
            ],
        }
    }

    // given a set of raw targets to start from
    // this would build a graph where it would link parents successively
    // a raw target can actually be associated with multiple targets
    // what do we do in that case?
    // an example is returning {name: a, flavor: cargo}, {name: a, flavor: poetry}
    // in this case, flavor should match of the returned parent dep
    pub fn build_graph<I>(&self, start: I) -> Result<(TargetGraph, Vec<Target>)>
    where
        I: IntoIterator<Item = RawTarget>,
    {
        let mut g = TargetGraph::new();
        let mut our_targets = Vec::new();
        for s in start {
            our_targets.extend(self.build_graph_rec(&mut g, &s)?);
        }
        Ok((g, our_targets))
    }

    // we are given a raw target
    // we get the parents
    // add them to graph
    // then add us
    // we also need to add an edge for us and them
    // so we need to return the nodes that resulted from us being inserted
    // there can be multiple nodes
    fn build_graph_rec(&self, g: &mut TargetGraph, raw: &RawTarget) -> Result<Vec<Target>> {
        // if our inference fails, we return fast
        let our_inferred_targets = self.run_inf(raw)?;
        for our in our_inferred_targets.iter() {
            // for one of our targets, we need to build graph of parents
            if g.contains_node(&our.target) {
                continue;
            }
            g.add_node(our.target.clone());
            for p in &our.parents {
                // for a parent's failure in inference, currently only logging it
                // the cli would ignore failures in parent graph building
                // this at-least gives us a partial graph, terminated at the point of failure

                match self.build_graph_rec(g, p) {
                    Err(e) => {
                        println!(
                            "warning: failed in creating graph for package={}. nabs will skip adding this target in analysis",
                            p.name
                        );
                        println!("reason:\n{:?}", e);
                    }
                    Ok(parent_targets) => {
                        for pt in parent_targets {
                            g.add_edge(&pt, &our.target).expect(
                                &format!("unexpected corruption, failed in adding edge for {:?} and {:?} even though they should be in the graph", p, our.target)
                            );
                        }
                    }
                };
            }
        }
        Ok(our_inferred_targets.into_iter().map(|i| i.target).collect())
    }

    pub fn run_inf(&self, raw: &RawTarget) -> Result<Vec<Single>> {
        // a single infer can return 0, 1 or more targets
        // we run multiple infers in a list
        // it is invalid for multiple infers to return anything other than 0
        // that is there should be only one infer which wins, it either returns 1 target or many targets
        // an infer can also say if we want to infer more after giving some result
        // the first infer which directly reads nabs.json simply asks us to break if it finds any target
        // basically, if you want to make sure nobody infers after you, you return break and its guaranteed that your infer would work
        // let mut inferred_targets = Vec::new();
        let mut inferred_targets = self.raw_run_inferrers(raw)?;
        self.validate_inferred_targets(raw, &inferred_targets)?;

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

    fn raw_run_inferrers(&self, raw: &RawTarget) -> Result<Vec<InferredTarget>> {
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
            if let InferredTarget::Nothing = inf_res.inferred_target {
                // nothing, just want the else part
            } else {
                inferred_targets.push(inf_res.inferred_target);
            }

            match inf_res.what_next {
                Next::Break => {
                    break;
                }
                Next::Continue => {}
            };
        }
        Ok(inferred_targets)
    }

    fn validate_inferred_targets(
        &self,
        raw: &RawTarget,
        inferred_targets: &Vec<InferredTarget>,
    ) -> Result<()> {
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
        Ok(())
    }
}

#[cfg(test)]
mod test {
    // this is going to be slightly complicated, ill need to create test structs inferrers also
    // these would actually be pretty simple structs
    // given an instantiated struct, they just be a list of what the dependencies are, we pass the map in instantiation

    use std::collections::HashMap;

    use crate::{
        graph::TargetGraph,
        infer::InferRunner,
        types::{RawTarget, Target},
    };

    use super::core::{Infer, InferResult, InferredTarget, Next, Single};

    struct Dep {
        ps: Vec<RawTarget>,
        flavors: Vec<String>,
    }

    struct MockInfer {
        n_by_deps: HashMap<String, Dep>,
    }

    impl Infer for MockInfer {
        fn from_raw_target(
            &self,
            t: &super::RawTarget,
        ) -> anyhow::Result<super::core::InferResult> {
            let deps = self.n_by_deps.get(t.name.to_string_ref());
            match deps {
                None => Ok(InferResult {
                    inferred_target: InferredTarget::Nothing,
                    what_next: Next::Continue,
                }),
                Some(deps) => {
                    if deps.flavors.len() == 1 {
                        Ok(InferResult {
                            inferred_target: InferredTarget::One(Single {
                                target: Target::new(t.name.clone(), deps.flavors[0].clone()),
                                parents: deps.ps.clone(),
                                failed_parents: vec![],
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
                                failed_parents: vec![],
                            })
                            .collect();

                        Ok(InferResult {
                            inferred_target: InferredTarget::Many(targets),
                            what_next: Next::Break,
                        })
                    }
                }
            }
        }
    }

    #[test]
    fn test_runner() {
        let infs: Vec<Box<dyn Infer>> = vec![Box::new(get_infer_1()), Box::new(get_infer_2())];
        let runner = InferRunner::new(infs);
        let start = vec![RawTarget::from_string_name("qureapi".to_string()).unwrap()];
        let (graph, _) = runner.build_graph(start).unwrap();
        compare(
            &graph,
            "qure_dicom_utils",
            "cargo",
            vec![("qer", "cargo"), ("qxr", "cargo")],
        );
        compare(
            &graph,
            "qer",
            "cargo",
            vec![("qer_reports", "cargo"), ("qureapi", "cargo")],
        );
        compare(&graph, "qxr", "cargo", vec![("qureapi", "cargo")]);
        compare(&graph, "qureapi", "cargo", vec![]);
        compare(&graph, "qer_reports", "cargo", vec![("qureapi", "cargo")]);

        let start = vec![RawTarget::from_string_name("image_manager".to_string()).unwrap()];
        let (graph, _) = runner.build_graph(start).unwrap();
        compare(
            &graph,
            "qsync_stream",
            "cargo",
            vec![("image_manager", "python")],
        );
        compare(
            &graph,
            "qsync_stream",
            "python",
            vec![("image_manager", "python")],
        );
        compare(&graph, "image_manager", "python", vec![]);
    }

    fn compare(graph: &TargetGraph, name: &str, flavor: &str, want: Vec<(&str, &str)>) {
        let ns = graph
            .neighbors(&Target::from_string_name(name.to_string(), flavor.to_string()).unwrap())
            .unwrap();
        let got: Vec<(&String, &String)> = ns
            .iter()
            .map(|t| (t.name_as_string_ref(), &t.flavor))
            .collect();
        assert_eq!(got.len(), want.len());
        for v in &want {
            assert!(want.contains(&v));
        }
    }

    fn get_infer_2() -> MockInfer {
        let mut n_by_deps: HashMap<String, Dep> = HashMap::new();
        n_by_deps.insert(
            "image_manager".to_string(),
            Dep {
                ps: vec![RawTarget::from_string_name("qsync_stream".to_string()).unwrap()],
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
        MockInfer { n_by_deps }
    }

    fn get_infer_1() -> MockInfer {
        let n_by_deps = HashMap::from([
            (
                "qureapi".to_string(),
                vec![
                    RawTarget::from_string_name("qxr".to_string()).unwrap(),
                    RawTarget::from_string_name("qer".to_string()).unwrap(),
                    RawTarget::from_string_name("qer_reports".to_string()).unwrap(),
                ],
            ),
            (
                "cathode".to_string(),
                vec![
                    RawTarget::from_string_name("qxr".to_string()).unwrap(),
                    RawTarget::from_string_name("qxr_reports".to_string()).unwrap(),
                ],
            ),
            (
                "qxr".to_string(),
                vec![RawTarget::from_string_name("qure_dicom_utils".to_string()).unwrap()],
            ),
            (
                "qxr_reports".to_string(),
                vec![RawTarget::from_string_name("qxr".to_string()).unwrap()],
            ),
            (
                "qer".to_string(),
                vec![RawTarget::from_string_name("qure_dicom_utils".to_string()).unwrap()],
            ),
            (
                "qer_reports".to_string(),
                vec![RawTarget::from_string_name("qer".to_string()).unwrap()],
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
        MockInfer { n_by_deps }
    }
}
