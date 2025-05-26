use std::path::{Component, Path, PathBuf};

use anyhow::{Result, bail};

pub fn posix_to_win(p: &str) -> String {
    p.replace("/", "\\")
}

/// copied from here: https://github.com/rust-lang/cargo/blob/fede83ccf973457de319ba6fa0e36ead454d2e20/src/cargo/util/paths.rs#L61
/// basically remove '.' and '..'
/// canonicalize does the same thing, but it checks for file existence to resolve symlinks
/// we don't need it, so using this
/// fails in cases like `hello/../..` when its not possible to go back
pub fn normalize_path(path: &Path) -> Result<PathBuf> {
    let mut components = path.components().peekable();
    let mut ret = if let Some(c @ Component::Prefix(..)) = components.peek().cloned() {
        components.next();
        PathBuf::from(c.as_os_str())
    } else {
        PathBuf::new()
    };

    for component in components {
        match component {
            Component::Prefix(..) => unreachable!(),
            Component::RootDir => {
                ret.push(component.as_os_str());
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if let None = ret.parent() {
                    bail!("failed in normalizing path, path={:?}", path);
                }
                ret.pop();
            }
            Component::Normal(c) => {
                ret.push(c);
            }
        }
    }
    Ok(ret)
}
