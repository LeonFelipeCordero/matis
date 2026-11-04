//! Source guards and output-path validation.
use super::{
    config::MutationConfig,
    digest,
    formatting::{command_error, text},
};
use crate::parsing::discovery::{detect_language, repository_paths};
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;
use std::{
    fs::{self, File},
    path::{Component, Path, PathBuf},
};

/// Resolve each target's working directory in configuration order.
///
/// `root` must be an absolute, resolved project directory. Reject paths that
/// escape it or are not directories; propagate path-resolution errors. This
/// reads filesystem metadata but does not create directories.
pub fn working_directories(root: &Path, config: &MutationConfig) -> Result<Vec<PathBuf>> {
    config
        .targets
        .iter()
        .map(|(name, target)| {
            let path = resolve(&root.join(&target.cwd))?;
            if !path.starts_with(root) || !path.is_dir() {
                bail!("Invalid working directory for target {name}: {}", path.display());
            }
            Ok(path)
        })
        .collect()
}

/// Make a path absolute, collapse dot components, and resolve existing symlinks.
///
/// Allow missing components so callers can resolve output paths before creation.
/// Read filesystem metadata only; propagate errors other than `NotFound`.
pub fn resolve(path: &Path) -> Result<PathBuf> {
    let absolute = std::path::absolute(path)?;
    let mut resolved = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                resolved.pop();
            }
            _ => {
                resolved.push(component);
            }
        }
        match resolved.canonicalize() {
            Ok(path) => resolved = path,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(command_error(error, &resolved.to_string_lossy())),
        }
    }
    Ok(resolved)
}

fn validate_output(root: &Path, manifests: &[Value], output: &Path) -> Result<()> {
    let mut protected = vec![resolve(&root.join("matis.json"))?];
    // Candidate selection and empty rule sets must not remove source protection.
    let sources = repository_paths(root)?.into_iter().filter(|path| detect_language(path).is_some());
    for path in sources.chain(manifests.iter().map(|manifest| root.join(text(manifest, "path")))) {
        protected.push(resolve(&path)?);
    }
    if (output.try_exists()? && detect_language(output).is_some()) || protected.iter().any(|path| path == output) {
        bail!("Mutation results cannot overwrite source files or matis.json");
    }
    Ok(())
}

/// Protect project sources and configuration, then create or reuse a log directory.
///
/// `root` and `output` must already be resolved absolute paths. Protect recognized
/// repository sources, manifest paths, and `matis.json`; append `.logs` to the
/// output path and return that directory. Do not create the results file itself.
/// Reject protected output paths and symlink log directories; propagate discovery,
/// resolution, and directory-creation failures. Parent directories must exist.
pub fn prepare_output(root: &Path, manifests: &[Value], output: &Path) -> Result<PathBuf> {
    validate_output(root, manifests, output)?;
    let mut logs = output.as_os_str().to_owned();
    logs.push(".logs");
    let logs = PathBuf::from(logs);
    if logs.is_symlink() {
        bail!("Mutation log directory must not be a symlink");
    }
    match fs::create_dir(&logs) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists && logs.is_dir() => {}
        Err(error) => return Err(command_error(error, &logs.to_string_lossy())),
    }
    Ok(logs)
}

/// Atomically replace a path with a new empty file and return its open handle.
///
/// Create a temporary file in the same directory, then replace the directory
/// entry instead of truncating a potentially hardlinked inode. Reject a symlink
/// destination and propagate temporary-file or replacement errors.
pub fn fresh_file(path: &Path) -> Result<File> {
    if path.is_symlink() {
        bail!("Refusing to overwrite a symlink: {}", path.display());
    }
    let temporary = tempfile::NamedTempFile::new_in(path.parent().unwrap_or(Path::new(".")))?;
    temporary.persist(path).map_err(|error| command_error(error.error, &path.to_string_lossy()))
}

pub struct Source {
    pub path: PathBuf,
    original: Vec<u8>,
    pub modified: Vec<u8>,
    restored: bool,
}
impl Source {
    /// Write the saved original bytes back to the source path.
    ///
    /// Propagate write failures so the campaign stops rather than continuing with
    /// modified source. Disable the drop fallback before attempting the write;
    /// a failed restoration is not silently retried.
    pub fn restore(&mut self) -> Result<()> {
        // A failed restoration aborts the campaign; do not silently retry in Drop.
        self.restored = true;
        fs::write(&self.path, &self.original).map_err(|error| command_error(error, &self.path.to_string_lossy()))?;
        Ok(())
    }
}
impl Drop for Source {
    /// Restore original bytes unless explicitly attempted; ignore fallback write errors.
    fn drop(&mut self) {
        if !self.restored {
            let _ = fs::write(&self.path, &self.original);
        }
    }
}
/// Read and verify a manifest's source and prepare replacement bytes in memory.
///
/// `root` must be resolved. Reject absolute, parent-traversing, or escaping paths,
/// invalid base64, invalid byte spans, and source hash/original-byte mismatches.
/// Propagate file-resolution and read errors. No mutation is written here; the
/// returned guard restores original bytes on drop unless `restore` was attempted.
pub fn source(root: &Path, manifest: &Value) -> Result<Source> {
    let name = text(manifest, "path");
    let relative = Path::new(name);
    let requested = root.join(relative);
    let path = requested.canonicalize().map_err(|error| command_error(error, &requested.to_string_lossy()))?;
    if relative.is_absolute()
        || relative.components().any(|part| part == Component::ParentDir)
        || !path.starts_with(root)
    {
        bail!("Unsafe mutation path: {name}");
    }
    let original = fs::read(&path).map_err(|error| command_error(error, &path.to_string_lossy()))?;
    let expected = STANDARD.decode(text(manifest, "originalBytes"))?;
    let start = manifest["span"]["startByte"].as_u64().and_then(|n| usize::try_from(n).ok());
    let end = manifest["span"]["endByte"].as_u64().and_then(|n| usize::try_from(n).ok());
    let valid = start.zip(end).filter(|&(start, end)| start < end && end <= original.len());
    let Some((start, end)) = valid else {
        bail!("Stale mutation source: {name}");
    };
    if digest(&original) != text(manifest, "sourceHash") || original[start..end] != expected {
        bail!("Stale mutation source: {name}");
    }
    let replacement = STANDARD.decode(text(manifest, "replacementBytes"))?;
    let modified = [&original[..start], replacement.as_slice(), &original[end..]].concat();
    Ok(Source { path, original, modified, restored: false })
}
