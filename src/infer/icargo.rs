use std::collections::HashMap;

use anyhow::Result;
use serde::Deserialize;

use super::core::{Infer, InferResult, InferredTarget, Next, Single};
use crate::types::{BuildSystemPath, PathFormat, RawTarget, Repository, Target};

pub const CARGO_FLAVOR: &str = "cargo";

#[derive(Debug, Deserialize)]
struct FullDep {
    #[serde(default)]
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum Dependency {
    Object(FullDep),
    #[allow(dead_code)]
    Simple(String),
}

#[derive(Debug, Deserialize)]
struct CargoToml {
    #[serde(default)]
    dependencies: HashMap<String, Dependency>,
    #[serde(default)]
    #[serde(rename = "dev-dependencies")]
    dev_dependencies: HashMap<String, Dependency>,
}

pub struct CargoInfer {
    repo: Box<dyn Repository>,
}

fn get_parents(
    our_target: &RawTarget,
    cargo_toml: CargoToml,
    repo: &Box<dyn Repository>,
) -> Result<Vec<RawTarget>> {
    let mut parents = Vec::new();
    parents.extend(deps_to_raw_targets(
        our_target,
        cargo_toml.dependencies,
        repo,
    )?);
    parents.extend(deps_to_raw_targets(
        our_target,
        cargo_toml.dev_dependencies,
        repo,
    )?);
    Ok(parents)
}

fn deps_to_raw_targets(
    our_target: &RawTarget,
    deps: HashMap<String, Dependency>,
    repo: &Box<dyn Repository>,
) -> Result<Vec<RawTarget>> {
    let mut res = Vec::new();
    for (_, dep) in deps.into_iter() {
        let maybe_path = match dep {
            Dependency::Simple(_) => None,
            Dependency::Object(o) => o.path,
        };
        if let Some(build_sys_path_str) = maybe_path {
            let path = BuildSystemPath::new(build_sys_path_str, PathFormat::Posix);
            let parent_raw_target = repo.resolve_rel_path(&path, our_target)?;
            res.push(parent_raw_target);
        }
    }
    Ok(res)
}

impl Infer for CargoInfer {
    fn from_raw_target(&self, t: &RawTarget) -> anyhow::Result<super::core::InferResult> {
        let content = self
            .repo
            .get_content(&self.repo.target_name_to_path(&t.name).join("Cargo.toml"));

        match content {
            None => Ok(InferResult {
                inferred_target: InferredTarget::Nothing,
                what_next: Next::Continue,
            }),
            Some(content) => {
                let cargo_toml: CargoToml = toml::from_str(&content)?;
                let parents = get_parents(t, cargo_toml, &self.repo)?;
                let target = Target::from_raw_target(&t, CARGO_FLAVOR.to_string())?;

                Ok(InferResult {
                    inferred_target: InferredTarget::One(Single {
                        target,
                        parents,
                        failed_parents: vec![],
                    }),
                    what_next: Next::Continue,
                })
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use crate::{
        infer::core::{Infer, InferredTarget, Next},
        types::{MockRepo, RawTarget},
    };

    use super::CargoInfer;

    #[test]
    fn test_infer() {
        let us_name = "libs/qsync_stream";
        let toml_str = r#"
            [package]
            name = "test_package"
            version = "0.1.0"

            [dependencies]
            serde = { version = "1.0", path = "../serde" }
            toml = { path = "../toml" }
            hey = "1.2"
            lol = {version = "1"}

            [dev-dependencies]
            anyhow = { path = "../anyhow" }
        "#;
        let repo = MockRepo::new(
            HashMap::from([(format!("{}/Cargo.toml", us_name), toml_str.to_string())]),
            PathBuf::new(),
        );
        let inf = CargoInfer {
            repo: Box::new(repo),
        };
        let infer_result = inf
            .from_raw_target(&RawTarget::from_string_name(us_name.to_string()).unwrap())
            .unwrap();

        assert_eq!(infer_result.what_next, Next::Continue);
        if let InferredTarget::One(single) = infer_result.inferred_target {
            // good, this is what we expect
            assert_eq!(single.target.name_as_string_ref(), us_name);
            let parents: Vec<&String> = single
                .parents
                .iter()
                .map(|p| p.name.to_string_ref())
                .collect();

            compare_vec(
                &parents,
                &vec![
                    &"libs/serde".to_string(),
                    &"libs/toml".to_string(),
                    &"libs/anyhow".to_string(),
                ],
            );
        } else {
            panic!("expected inferred_target to be One variant");
        }
    }

    fn compare_vec<T: Eq>(want: &Vec<T>, got: &Vec<T>) {
        assert_eq!(want.len(), got.len());
        for v in want {
            assert!(got.contains(v));
        }
    }
}
