/// core types used by nabs
/// A note on handling paths in any build system ish program
/// There are many kinds of path-like strings that we are using in nabs consistently
/// Each of these have slightly different semantics
/// The first path-like string is how nabs uniquely identifies packages in the monorepo
/// These are basically posix paths relative to the repository root (kinda like bazel but without the `//`, this can change later though)
/// nabs target names can never be absolute paths, or contain `..`/`.`, or have empty components `libs//python`
/// These strings are modeled using `TargetName`
/// The other strings are strings used by individual build systems to denote relative paths in their manifest files (like dependencies[].path in Cargo.toml)
/// These strings can be arbitrary formats which a build system uses
/// These are modeled using `BuildSystemPath`. The type needs to be flexible enough to allow all build system definitions. The build system inferrer is responsible for using the correct representation
/// The last path type is the actual `Path`, how OS models it
/// A simple complication in `Path` is that they us `OsStr` instead of `String`. Build systems already encode relative paths in their manifest files
/// so it is reasonable to assume that `nabs` will consider all `Path` which can't be converted to `String` to simply be invalid
/// That node in graph should be invalidated in this case (not handled right now)
/// Finally `Repository` is the trait which allows us to play with the repository (it could be FS or just a mock implementation)
/// This trait provides many primitives to inter-convert all our path representations
use std::{
    collections::HashMap, fmt::Display, io::ErrorKind, path::{Path, PathBuf}
};

use anyhow::{Context, Result, anyhow, bail};
use log::info;

use crate::paths::{normalize_path, posix_to_win};

/// `TargetName` is the format nabs uses to uniquely identify a package in the monorepo
/// the format is simply a posix based string. It can only be a relative path from the root of the monorepo
/// correct formats: packages/python/lib
/// wrong: packages/../python/lib (.. or . not allowed), /packages/hello (absolute paths not allowed), hey//hello (empty components not allowed), packages\\python (windows paths not allowed)
#[derive(Debug, Hash, PartialEq, Eq, Clone)]
pub struct TargetName(String);

impl Display for TargetName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl TargetName {
    /// creates a new `TargetName`
    /// the function will return `Error` if the provided name is not in the correct format
    pub fn new(name: String) -> Result<Self> {
        Self::validate(&name)?;
        Ok(TargetName(name))
    }

    pub fn to_string_ref<'a>(&'a self) -> &'a String {
        &self.0
    }

    fn validate(name: &str) -> Result<()> {
        // validate that name is a valid relative posix path
        // but does not contain . or .. components
        if name.is_empty() {
            bail!("target name cannot be empty".to_string());
        }

        // check for absolute paths
        if name.starts_with('/') {
            bail!("target name cannot be absolute path: name={}", name);
        }

        // split into path components and validate each
        for component in name.split('/') {
            if component.is_empty() {
                bail!(
                    "detected target name with multiple slashes (like this: //). This is not allowed. name={}",
                    name
                );
            }
            if component == "." || component == ".." {
                bail!("'.' and '..' not allowed in target name, name={}", name);
            }
        }

        Ok(())
    }
}

/// a target is the main entity used in a build graph to uniquely identify a package in the monorepo
/// flavor: this can be something like `cargo`, `poetry`, etc.
/// a single package can expose multiple detected build systems, these are differentiated with `flavor`
/// Each inferrer has a single value it sets for `flavor`
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct Target {
    pub name: TargetName,
    pub flavor: String,
}

impl Target {
    pub fn new(name: TargetName, flavor: String) -> Self {
        Target { name, flavor }
    }

    pub fn from_string_name(name: String, flavor: String) -> Result<Self> {
        Ok(Self::new(TargetName::new(name)?, flavor))
    }

    pub fn from_raw_target(rt: &RawTarget, flavor: String) -> Result<Self> {
        Ok(Target::from_string_name(
            rt.name.to_string_ref().clone(),
            flavor,
        )?)
    }

    pub fn name_as_string_ref<'a>(&'a self) -> &'a String {
        &self.name.to_string_ref()
    }
}

impl Display for Target {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}", self.name, self.flavor)
    }
}

/// a `RawTarget` directly maps to a single package directory
/// It is the base unit for interacting with the actual repository
/// The difference with `Target` is simple. We use `RawTarget` at `infer` level, a directory which is a package is represented by `RawTarget`
/// individual inferrers and repository traits use this as argument and return `Target`s, the actual used in graph computation
#[derive(Hash, Eq, PartialEq, Debug, Clone)]
pub struct RawTarget {
    pub name: TargetName,
}

impl RawTarget {
    /// can fail if the name is an invalid `TargetName`
    pub fn new(name: TargetName) -> Self {
        Self { name }
    }

    pub fn from_string_name(name: String) -> Result<Self> {
        Ok(Self::new(TargetName::new(name.clone()).context(
            anyhow!("failed in creating target_name={}", name),
        )?))
    }
}

impl Display for RawTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}

/// Generally build system's manifest files themselves try to handle files in a way which can be ported across OS's
/// These generally end up being simply posix paths
/// The other format I can think of is simply using the host path format
/// This enum is used to differentiate between the two
pub enum PathFormat {
    Posix,
    Host,
}

/// the representation used by individual build systems for providing relative paths in their manifest files
/// as an example, bazel would always use posix relative paths
/// this struct is used as the basic unit of translation to our own format from other build-system formats
/// A `BuildSystemPath` would generally be always convertible to `String`, because we have read it from a string file content in the first place
/// It is expected that an inferrer would use the correct format to represent the path of the individual build system it is parsing
/// Conversion to `Target`, etc. is left to `Repository` trait
pub struct BuildSystemPath {
    pub raw: String,
    pub format: PathFormat,
}

impl BuildSystemPath {
    pub fn new(raw: String, format: PathFormat) -> Self {
        Self { raw, format }
    }

    /// get the actual host path that we point to
    pub fn get_host_path(&self) -> PathBuf {
        if cfg!(windows) {
            let name = match self.format {
                // PathFormat::Posix => self.raw.replace("/", "\\"),
                PathFormat::Posix => posix_to_win(&self.raw),
                _ => self.raw.clone(),
            };
            PathBuf::from(name)
        } else {
            PathBuf::from(self.raw.clone())
        }
    }

    pub fn is_absolute(&self) -> bool {
        PathBuf::from(&self.raw).is_absolute()
    }
}

/// the main trait for interacting with the monorepo (it can be FS based, or simply a mock one)
/// note: the fact that it models a file system backed repository is not abstracted out, I'm adding this here cuz all methods work with paths or talk about reading files, etc
/// This trait is also responsible for providing ways to convert all our different path string formats
/// I'm keeping the API very concrete and explicit, its very confusing right now anyways, not touching the whole `From` business for now
///
pub trait Repository {
    /// given a path provide the content corresponding to that "path"
    /// for a FS repository, this would simply involve reading the file at the path
    /// if the path is not found, return `None`
    /// NOTE: this might change to `Result` type later
    fn get_content(&self, path: &Path) -> Option<String>;

    /// return the root of the monorepo
    fn workspace_root(&self) -> &Path;

    fn get_nabs_packages(&self) -> Vec<PathBuf> {
        let nabs_pkgs: Vec<PathBuf> = ignore::Walk::new(self.workspace_root()).into_iter().filter_map(|v| {
            match v {
                Err(e) => {
                    eprintln!("warning: nabs could not read path, skipping analysis for this path and its children. cause={}", e);
                    None
                },
                Ok(entry) => {
                    let p = entry.path();
                    if p.is_file() {
                        match p.file_name() {
                            None => None,
                            Some(v) => {
                                if v == "nabs.json" {
                                    PathBuf::from(p).parent().map(|p| PathBuf::from(p.strip_prefix(self.workspace_root()).unwrap()))
                                } else {
                                    None
                                }
                            },
                        }
                        // Some(p)
                    } else {
                        None
                    }

                },
            }
        }).collect();
        nabs_pkgs
    }

    /// given a path relative to a RawTarget, construct a new RawTarget
    /// with the correct target name
    fn resolve_rel_path(
        &self,
        rel_path: &BuildSystemPath,
        rel_to: &RawTarget,
    ) -> Result<RawTarget> {
        // rel_path is a relative path from rel_to
        // rel_to.name is relative to the workspace
        // simply normalizing (rel_to / rel_path) will give us rel_path relative to workspace
        // this function fails if rel_path is a path outside the workspace (normalization fails)
        let base = self.target_name_to_path(&rel_to.name);
        let rel_to_base = rel_path.get_host_path();
        let path = base.join(rel_to_base);
        let target_name = normalize_path(&path).context(anyhow!(
            "the path is mostly outside your workspace, path={:?}",
            path
        ))?;
        let target_name = target_name.to_str().ok_or(anyhow!(
            "failed in converting path to String: {:?}",
            target_name
        ))?;
        Ok(RawTarget::from_string_name(String::from(target_name))?)
    }

    /// a standard target-name like packages/python/qsync_stream
    /// convert it to a path in the host system
    fn target_name_to_path(&self, t: &TargetName) -> PathBuf {
        if cfg!(windows) {
            PathBuf::from(posix_to_win(t.to_string_ref()))
        } else {
            PathBuf::from(t.to_string_ref().clone())
        }
    }
}

#[derive(Debug, Clone)]
pub struct Monorepo {
    workspace_path: PathBuf,
}

impl Monorepo {
    /// this would create a new Monorepo instance
    /// it tries to find the workspace root by looking up all parents recursively and trying to find workspace.json file
    /// fails if it doesn't find the file
    pub fn new() -> Result<Self> {
        let cwd = std::env::current_dir().context("Failed to get current working directory")?;

        let mut search_path = cwd.as_path();
        loop {
            let workspace_file = search_path.join("workspace.json");
            if workspace_file.exists() {
                info!("workspace-path={}", search_path.to_string_lossy());
                return Ok(Monorepo {
                    workspace_path: search_path.to_path_buf(),
                });
            }

            match search_path.parent() {
                Some(parent) => search_path = parent,
                None => break,
            }
        }
        bail!("Could not find workspace.json in current directory or any parent directory")
    }
}

impl Repository for Monorepo {
    fn get_content(&self, path: &Path) -> Option<String> {
        match std::fs::read_to_string(path) {
            Ok(s) => Some(s),
            Err(e) => {
                match e.kind() {
                    ErrorKind::NotFound => None,
                    _ => panic!("{:?}", e),
                }
            }
        }
    }

    fn workspace_root(&self) -> &Path {
        &self.workspace_path
    }
}

#[derive(Debug, Clone)]
pub struct MockRepo {
    fake: HashMap<String, String>,
    workspace_path: PathBuf,
}

impl MockRepo {
    #[allow(unused)]
    pub fn new(path_by_content: HashMap<String, String>, workspace_path: PathBuf) -> MockRepo {
        MockRepo {
            fake: path_by_content,
            workspace_path,
        }
    }
}

impl Repository for MockRepo {
    fn get_content(&self, path: &Path) -> Option<String> {
        self.fake.get(path.to_str().unwrap()).map(|c| c.clone())
    }
    fn workspace_root(&self) -> &Path {
        &self.workspace_path
    }
}

#[cfg(test)]
mod test {
    use std::{collections::HashMap, path::PathBuf};

    use anyhow::Result;

    use super::{BuildSystemPath, MockRepo, PathFormat, Repository, TargetName};
    use crate::types::RawTarget;

    #[test]
    fn test_resolve_rel_path() {
        let repo = MockRepo::new(HashMap::new(), PathBuf::from("hey/test"));
        assert_eq!(
            resolve("../qsync_stream", "packages/python/image_manager", &repo)
                .unwrap()
                .to_string_ref(),
            "packages/python/qsync_stream"
        );
        assert!(
            resolve(
                "../../../../qsync_stream",
                "packages/python/image_manager",
                &repo
            )
            .is_err()
        );
        assert_eq!(
            resolve(
                "libs/qsync_stream/../qsync",
                "packages/python/image_manager",
                &repo
            )
            .unwrap()
            .to_string_ref(),
            "packages/python/image_manager/libs/qsync"
        );
    }

    fn resolve(rel_path: &str, rel_to: &str, repo: &impl Repository) -> Result<TargetName> {
        repo.resolve_rel_path(
            &BuildSystemPath::new(rel_path.to_string(), PathFormat::Posix),
            &RawTarget::from_string_name(rel_to.to_string()).unwrap(),
        )
        .map(|v| v.name)
    }
}
