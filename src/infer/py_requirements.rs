use crate::types::{BuildSystemPath, PathFormat, RawTarget, Repository, Target};

use super::core::{FailedParent, Infer, InferResult, InferredTarget, Next, Single};

pub const FLAVOR: &str = "python_requirements";
pub const DEFAULT_REQ_FILE_NAME: &str = "requirements.txt";

pub struct PyRequirementsInfer {
    repo: Box<dyn Repository>,
    req_file_name: String,
}

impl PyRequirementsInfer {
    pub fn new(repo: Box<dyn Repository>, req_file_name: String) -> Self {
        Self {
            repo,
            req_file_name,
        }
    }
}

impl Infer for PyRequirementsInfer {
    fn from_raw_target(&self, t: &RawTarget) -> anyhow::Result<super::core::InferResult> {
        let content = self.repo.get_content(
            &self
                .repo
                .target_name_to_path(&t.name)
                .join(&self.req_file_name),
        );

        match content {
            None => Ok(InferResult {
                inferred_target: InferredTarget::Nothing,
                what_next: Next::Continue,
            }),
            Some(content) => {
                let parents = get_file_names(&content);
                let mut failed = Vec::new();
                let mut success = Vec::new();
                for p in &parents {
                    let p = BuildSystemPath::new(p.to_string(), PathFormat::Posix);
                    if p.is_absolute() {
                        failed.push(FailedParent {
                            name: p.raw.clone(),
                            reason: "absolute paths are not allowed".to_string(),
                        });
                    } else {
                        let res = self.repo.resolve_rel_path(&p, t);
                        match res {
                            Ok(raw_target) => success.push(raw_target),
                            Err(e) => failed.push(FailedParent {
                                name: p.raw.clone(),
                                reason: format!("{}", e),
                            }),
                        };
                    }
                }
                Ok(InferResult {
                    inferred_target: InferredTarget::One(Single {
                        target: Target::from_raw_target(t, FLAVOR.to_string())?,
                        parents: success,
                        failed_parents: failed,
                    }),
                    what_next: Next::Continue,
                })
            }
        }
    }
}

fn get_file_names(content: &str) -> Vec<String> {
    let mut paths = Vec::new();

    for line in content.lines() {
        let line = line.trim();

        // Check for lines starting with ./
        if line.starts_with("./") || line.starts_with("../") {
            paths.push(line.to_string());
        }

        // Check for lines with @ file:// pattern
        if let Some(start_at_the_rate_index) = line.find("@") {
            let line = &line[start_at_the_rate_index..];
            if let Some(start_file_index) = line.find("file://") {
                let file_path = &line[start_file_index + 7..];
                paths.push(file_path.to_string());
            }
        }
    }
    paths
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use crate::{
        infer::{
            core::{Infer, InferredTarget, Next},
            py_requirements::DEFAULT_REQ_FILE_NAME,
        },
        types::{MockRepo, RawTarget},
    };

    use super::PyRequirementsInfer;

    #[test]
    fn test_infer() {
        let us_name = "libs/qsync_stream";
        let req_str = r#"
            ./../serde
            ./../toml
            requests==1.2.3
            hello @ file://../anyhow
            he  @ file:///yours/truly
            ./../../../invalid_path
            ../../../invalid_path
        "#;
        let repo = MockRepo::new(
            HashMap::from([(
                format!("{}/{}", us_name, DEFAULT_REQ_FILE_NAME),
                req_str.to_string(),
            )]),
            PathBuf::new(),
        );
        let inf = PyRequirementsInfer::new(Box::new(repo), DEFAULT_REQ_FILE_NAME.to_string());
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

            let failed_parents: Vec<String> = single
                .failed_parents
                .iter()
                .map(|p| p.name.to_string())
                .collect();
            compare_vec(
                &failed_parents,
                &vec![
                    "/yours/truly".to_string(),
                    "./../../../invalid_path".to_string(),
                    "../../../invalid_path".to_string(),
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
