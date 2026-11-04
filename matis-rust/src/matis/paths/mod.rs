//! Native equivalents of Python's shared path expansion and resolution operations.
mod home;

use anyhow::Result;
use std::{
    collections::HashSet,
    env,
    path::{Component, Path, PathBuf},
};

pub use home::expand_home;

/// Expand home markers and resolve `path` against cwd, following existing symlinks before `..`.
/// Permit nonexistent components without creating files or directories.
///
/// # Errors
/// Propagate home lookup, cwd, or symlink-read failures; reject symlink loops with `ELOOP`.
pub fn absolute_path(path: &Path) -> Result<PathBuf> {
    let expanded = expand_home(path)?;
    let absolute = if expanded.is_absolute() { expanded } else { env::current_dir()?.join(expanded) };
    resolve_components(&absolute, &mut HashSet::new())
}

fn resolve_components(path: &Path, links: &mut HashSet<PathBuf>) -> Result<PathBuf> {
    let mut resolved = PathBuf::new();
    for part in path.components() {
        match part {
            Component::ParentDir => {
                resolved.pop();
            }
            Component::CurDir => {}
            other => {
                resolved.push(other);
                resolved = resolve_link(resolved, links)?;
            }
        }
    }
    Ok(resolved)
}

fn resolve_link(path: PathBuf, links: &mut HashSet<PathBuf>) -> Result<PathBuf> {
    if !path.symlink_metadata().is_ok_and(|metadata| metadata.file_type().is_symlink()) {
        return Ok(path);
    }
    if !links.insert(path.clone()) {
        return Err(std::io::Error::from_raw_os_error(libc::ELOOP).into());
    }
    let target = std::fs::read_link(&path)?;
    let absolute = if target.is_absolute() { target } else { path.parent().unwrap_or(Path::new("/")).join(target) };
    let resolved = resolve_components(&absolute, links);
    links.remove(&path);
    resolved
}
