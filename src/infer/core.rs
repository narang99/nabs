use anyhow::Result;

use crate::types::{RawTarget, Target};


/// if the inferrer fails for some parent during parsing, they should return this for that particular parent
/// Useful to keep this information for showing diagnostics in the end
#[derive(Debug)]
pub struct FailedParent {
    pub name: String,
    pub reason: String,
}

#[derive(Debug)]
pub struct Single {
    pub target: Target,
    pub parents: Vec<RawTarget>,
    pub failed_parents: Vec<FailedParent>,
}

#[derive(Debug)]
pub enum InferredTarget {
    One(Single),
    Many(Vec<Single>),
    Nothing,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Next {
    Continue,
    Break,
}

#[derive(Debug)]
pub struct InferResult {
    pub inferred_target: InferredTarget,
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
