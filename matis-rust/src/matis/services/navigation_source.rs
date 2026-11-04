use crate::domain::navigation::{NavigationReader, SourceOptions};
use crate::paths::absolute_path;
use anyhow::Result;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    fs::{File, OpenOptions},
    io::{self, Read},
    path::{Component, Path},
};

pub const MAX_FILE_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_EXCERPT_BYTES: usize = 16 * 1024;
/// Read a bounded local excerpt only after matching the indexed file's SHA-256.
/// Use `options` for one-based line paging within `node`'s span; return a status without
/// source for external/pathless, unverified, changed, unsafe, or otherwise unavailable files.
/// Propagate reader errors; local filesystem failures become statuses. Do not write files.
pub fn source_excerpt(reader: &mut dyn NavigationReader, node: &Value, options: &SourceOptions) -> Result<Value> {
    let Some(path) = node["path"].as_str().filter(|p| !p.is_empty()) else {
        return Ok(json!({"status": "unavailable"}));
    };
    if node["level"] == "external" {
        return Ok(json!({"status": "unavailable"}));
    }
    let files = reader.file(path)?;
    if files.len() != 1 || files[0]["sha256"].as_str().is_none_or(str::is_empty) {
        return Ok(source_status("unverified", path));
    }
    let (status, data) = local_bytes(&reader.snapshot().root, path);
    if status != "ok" {
        return Ok(source_status(status, path));
    }
    let hash = format!("{:x}", Sha256::digest(&data));
    if files[0]["sha256"] != hash {
        return Ok(source_status("changed", path));
    }
    Ok(verified_excerpt(node, &data, options))
}
/// Return a status/path record, adding file and excerpt byte caps for `too_large`.
pub fn source_status(status: &str, path: &str) -> Value {
    let mut value = json!({"status": status, "path": path});
    if status == "too_large" {
        value["maxFileBytes"] = json!(MAX_FILE_BYTES);
        value["maxExcerptBytes"] = json!(MAX_EXCERPT_BYTES);
    }
    value
}
/// Borrow UTF-8 lines from `data`, splitting only on LF and removing trailing CR.
/// Omit a trailing empty row; return an error for invalid UTF-8 without replacing bytes.
pub fn source_lines(data: &[u8]) -> std::result::Result<Vec<&str>, std::str::Utf8Error> {
    Ok(std::str::from_utf8(data)?.split_terminator('\n').map(|line| line.strip_suffix('\r').unwrap_or(line)).collect())
}
/// Format already hash-verified `data` as a span/line-bounded excerpt using `options`.
/// Return statuses for invalid UTF-8, absent symbol spans, or excerpts above 16 KiB.
/// Do not verify the hash or enforce the whole-file byte cap here.
pub fn verified_excerpt(node: &Value, data: &[u8], options: &SourceOptions) -> Value {
    let path = node["path"].as_str().unwrap_or("");
    let Ok(lines) = source_lines(data) else {
        return source_status("unsupported_encoding", path);
    };
    if node["level"] != "file" && (node["startLine"].is_null() || node["endLine"].is_null()) {
        return source_status("span_unavailable", path);
    }
    let excerpt = excerpt_lines(node, &lines, options);
    let bytes: usize = excerpt["lines"].as_array().unwrap().iter().map(|v| v.as_str().unwrap().len()).sum();
    if bytes > MAX_EXCERPT_BYTES { source_status("too_large", path) } else { excerpt }
}
/// Return a one-based line page bounded by the node span, available `lines`, and `options`.
/// File nodes use the whole file. Clamp start to the span's first line and include
/// continuation metadata; assume valid indexed spans and positive line limits.
/// Do not enforce byte limits or read files.
pub fn excerpt_lines(node: &Value, lines: &[&str], options: &SourceOptions) -> Value {
    let (first, last) = if node["level"] == "file" {
        (1, lines.len())
    } else {
        (
            node["startLine"].as_u64().filter(|n| *n != 0).unwrap_or(1) as usize,
            node["endLine"].as_u64().filter(|n| *n != 0).map_or(lines.len(), |n| n as usize),
        )
    };
    let start = requested_start(first, options.start_line.as_ref());
    let index = start.as_u64().and_then(|value| usize::try_from(value).ok());
    let end = last.min(lines.len());
    let stop = index.map_or(end, |start| end.min(start.saturating_add(options.max_lines.saturating_sub(1))));
    let excerpt = index.filter(|start| *start <= stop).map_or_else(Vec::new, |start| lines[start - 1..stop].to_vec());
    json!({"status": "ok", "path": node["path"], "startLine": start, "endLine": stop,
        "lines": excerpt, "truncated": stop < last, "nextStartLine": if stop < last { Some(stop + 1) } else { None }})
}
fn requested_start(first: usize, requested: Option<&serde_json::Number>) -> serde_json::Number {
    let Some(number) = requested else {
        return first.into();
    };
    if number.to_string().starts_with('-') || number.as_u64().is_some_and(|value| value < first as u64) {
        return first.into();
    }
    number.clone()
}
/// Read at most 8 MiB from a regular file beneath resolved `root` using a relative path.
/// Reject absolute paths, parent components, and escapes; return status/bytes without writing.
/// Unsafe, missing, unreadable, or oversized files yield empty bytes rather than errors.
pub fn local_bytes(root: &str, relative: &str) -> (&'static str, Vec<u8>) {
    let relative = Path::new(relative);
    if relative.is_absolute() || relative.components().any(|c| c == Component::ParentDir) {
        return ("unsafe", vec![]);
    }
    let result = (|| -> io::Result<(&'static str, Vec<u8>)> {
        let root = absolute_path(Path::new(root))
            .map_err(|error| error.downcast::<io::Error>().unwrap_or_else(io::Error::other))?;
        let target = absolute_path(&root.join(relative))
            .map_err(|error| error.downcast::<io::Error>().unwrap_or_else(io::Error::other))?;
        if target == root || !target.starts_with(&root) {
            return Ok(("unsafe", vec![]));
        }
        contained_bytes(&root, &target)
    })();
    match result {
        Ok(value) => value,
        Err(e) if e.kind() == io::ErrorKind::NotFound => ("missing", vec![]),
        Err(e) if e.raw_os_error() == Some(libc::ELOOP) => ("unsafe", vec![]),
        Err(_) => ("unreadable", vec![]),
    }
}
/// Read a regular `target` after the caller verifies canonical containment within `root`.
/// Pin directory descriptors and reject symlink swaps; return `ok` with at most 8 MiB,
/// or `unreadable`/`too_large` with empty bytes. Do not write files.
///
/// # Errors
/// Propagate path, open, metadata, and read errors for the caller to classify.
#[cfg(unix)]
pub fn contained_bytes(root: &Path, target: &Path) -> io::Result<(&'static str, Vec<u8>)> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::{ffi::OsStrExt, fs::OpenOptionsExt},
        },
    };
    let mut descriptor = OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW).open(root)?;
    let parts: Vec<_> = target.strip_prefix(root).map_err(io::Error::other)?.components().collect();
    for (index, part) in parts.iter().enumerate() {
        let name = CString::new(part.as_os_str().as_bytes()).map_err(io::Error::other)?;
        let last = index + 1 == parts.len();
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | if last { libc::O_NONBLOCK } else { libc::O_DIRECTORY };
        // Each open is relative to a pinned directory; symlink swaps cannot escape it.
        let raw = unsafe { libc::openat(descriptor.as_raw_fd(), name.as_ptr(), flags) };
        if raw < 0 {
            return Err(io::Error::last_os_error());
        }
        descriptor = unsafe { File::from_raw_fd(raw) };
    }
    let metadata = descriptor.metadata()?;
    if !metadata.is_file() {
        return Ok(("unreadable", vec![]));
    }
    if metadata.len() > MAX_FILE_BYTES as u64 {
        return Ok(("too_large", vec![]));
    }
    let mut data = vec![];
    descriptor.take((MAX_FILE_BYTES + 1) as u64).read_to_end(&mut data)?;
    Ok(if data.len() > MAX_FILE_BYTES { ("too_large", vec![]) } else { ("ok", data) })
}
