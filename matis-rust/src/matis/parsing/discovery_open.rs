//! Resolve aliases, then open each component relative to pinned descriptors.
use std::{
    ffi::CString,
    os::{
        fd::{AsRawFd, FromRawFd},
        unix::{ffi::OsStrExt, fs::OpenOptionsExt},
    },
};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read},
    path::{Component, Path, PathBuf},
};

pub(super) struct SourceRoot {
    path: PathBuf,
    descriptor: File,
}

fn open_child(parent: &File, name: &Path, directory: bool) -> io::Result<File> {
    let name = CString::new(name.as_os_str().as_bytes()).map_err(io::Error::other)?;
    let flags = libc::O_RDONLY
        | libc::O_NOFOLLOW
        | libc::O_CLOEXEC
        | if directory { libc::O_DIRECTORY } else { libc::O_NONBLOCK };
    // Never follow a component that changed into a symlink after resolution.
    let raw = unsafe { libc::openat(parent.as_raw_fd(), name.as_ptr(), flags) };
    if raw < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(unsafe { File::from_raw_fd(raw) })
}

fn open_relative(root: &File, relative: &Path, directory: bool) -> io::Result<File> {
    let mut descriptor = root.try_clone()?;
    let mut parts = relative.components().peekable();
    while let Some(part) = parts.next() {
        let Component::Normal(name) = part else {
            return Err(io::Error::other("Unsafe source path"));
        };
        descriptor = open_child(&descriptor, Path::new(name), directory || parts.peek().is_some())?;
    }
    Ok(descriptor)
}

impl SourceRoot {
    /// Pin a descriptor for an absolute, resolved root directory until this value is dropped.
    ///
    /// # Errors
    /// Returns an I/O error for non-absolute paths, unsafe components, symlinks, or failed opens.
    pub(super) fn new(root: &Path) -> io::Result<Self> {
        let filesystem = OpenOptions::new().read(true).custom_flags(libc::O_DIRECTORY).open("/")?;
        let descriptor = open_relative(&filesystem, root.strip_prefix("/").map_err(io::Error::other)?, true)?;
        Ok(Self { path: root.to_owned(), descriptor })
    }

    /// Resolve `path` and read regular-file bytes relative to the pinned root descriptor.
    /// Return `None` if the resolved target is outside the root; reject symlink swaps on open.
    ///
    /// # Errors
    /// Returns an I/O error if resolution, secure opening, file validation, or reading fails.
    pub(super) fn read(&self, path: &Path) -> io::Result<Option<Vec<u8>>> {
        let target = fs::canonicalize(path)?;
        let Ok(relative) = target.strip_prefix(&self.path) else {
            return Ok(None);
        };
        self.read_resolved(relative).map(Some)
    }

    fn read_resolved(&self, relative: &Path) -> io::Result<Vec<u8>> {
        let mut file = open_relative(&self.descriptor, relative, false)?;
        if !file.metadata()?.is_file() {
            return Err(io::Error::other("Source is not a regular file"));
        }
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn resolved_file_and_parent_swaps_never_read_external_bytes() -> io::Result<()> {
        let root = tempfile::tempdir()?;
        let outside = tempfile::tempdir()?;
        fs::create_dir(root.path().join("src"))?;
        fs::write(root.path().join("src/main.py"), "internal")?;
        fs::write(outside.path().join("main.py"), "external")?;
        let pinned = SourceRoot::new(root.path())?;
        let parent = open_relative(&pinned.descriptor, Path::new("src"), true)?;
        fs::rename(root.path().join("src"), root.path().join("old"))?;
        symlink(outside.path(), root.path().join("src"))?;
        assert!(pinned.read_resolved(Path::new("src/main.py")).is_err());
        assert!(pinned.read(&root.path().join("src/main.py"))?.is_none());
        let mut file = open_child(&parent, Path::new("main.py"), false)?;
        let mut text = String::new();
        file.read_to_string(&mut text)?;
        assert_eq!(text, "internal");
        fs::remove_file(root.path().join("old/main.py"))?;
        symlink(outside.path().join("main.py"), root.path().join("old/main.py"))?;
        assert!(open_child(&parent, Path::new("main.py"), false).is_err());
        assert!(pinned.read_resolved(Path::new("old/main.py")).is_err());
        symlink("loop.py", root.path().join("loop.py"))?;
        assert_eq!(pinned.read(&root.path().join("loop.py")).unwrap_err().raw_os_error(), Some(libc::ELOOP));
        Ok(())
    }

    #[test]
    fn root_descriptor_survives_root_replacement() -> io::Result<()> {
        let parent = tempfile::tempdir()?;
        let root = parent.path().join("repo");
        fs::create_dir(&root)?;
        fs::write(root.join("main.py"), "internal")?;
        let pinned = SourceRoot::new(&root)?;
        fs::rename(&root, parent.path().join("old"))?;
        fs::create_dir(&root)?;
        fs::write(root.join("main.py"), "replacement")?;
        assert_eq!(pinned.read(&root.join("main.py"))?, Some(b"internal".to_vec()));
        Ok(())
    }
}
