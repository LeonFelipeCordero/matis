use anyhow::{Result, ensure};
use std::{
    ffi::{CStr, CString},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

/// Expand leading `~` using `HOME` or the current user's passwd entry, and `~user` via passwd.
/// Leave other paths unchanged; do not resolve symlinks or create filesystem entries.
/// Return an error for unknown users, invalid username bytes, or failed home lookup.
pub fn expand_home(path: &Path) -> Result<PathBuf> {
    let bytes = path.as_os_str().as_bytes();
    if !bytes.starts_with(b"~") {
        return Ok(path.to_path_buf());
    }
    let end = bytes.iter().position(|byte| *byte == b'/').unwrap_or(bytes.len());
    let username = &bytes[1..end];
    let home = if username.is_empty() { current_home()? } else { passwd_home(Some(&CString::new(username)?))? };
    let suffix = &bytes[end..];
    let start = suffix.iter().position(|byte| *byte != b'/').unwrap_or(suffix.len());
    Ok(home.join(std::ffi::OsStr::from_bytes(&suffix[start..])))
}
fn current_home() -> Result<PathBuf> {
    match std::env::var_os("HOME") {
        Some(home) => Ok(home.into()),
        None => passwd_home(None),
    }
}
fn passwd_home(user: Option<&CStr>) -> Result<PathBuf> {
    let mut capacity = 4096;
    loop {
        let mut buffer = vec![0u8; capacity];
        let mut entry = std::mem::MaybeUninit::<libc::passwd>::uninit();
        let mut result = std::ptr::null_mut();
        // Reentrant passwd functions keep all returned strings in this owned buffer.
        let status = unsafe {
            match user {
                Some(user) => libc::getpwnam_r(
                    user.as_ptr(),
                    entry.as_mut_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ),
                None => libc::getpwuid_r(
                    libc::getuid(),
                    entry.as_mut_ptr(),
                    buffer.as_mut_ptr().cast(),
                    buffer.len(),
                    &mut result,
                ),
            }
        };
        if status == libc::ERANGE {
            capacity = capacity.checked_mul(2).ok_or_else(|| anyhow::anyhow!("Could not determine home directory."))?;
            continue;
        }
        ensure!(status == 0 && !result.is_null(), "Could not determine home directory.");
        let directory = unsafe { CStr::from_ptr((*result).pw_dir) };
        return Ok(PathBuf::from(std::ffi::OsStr::from_bytes(directory.to_bytes())));
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn named_user_homes_and_unknown_users() {
        let home = passwd_home(None).unwrap();
        let name = unsafe { CStr::from_ptr((*libc::getpwuid(libc::getuid())).pw_name) }.to_str().unwrap();
        assert_eq!(expand_home(Path::new(&format!("~{name}"))).unwrap(), home);
        assert_eq!(expand_home(Path::new(&format!("~{name}/sub"))).unwrap(), home.join("sub"));
        assert_eq!(expand_home(Path::new(&format!("~{name}//sub"))).unwrap(), home.join("sub"));
        assert_eq!(
            expand_home(Path::new("~matis_nonexistent_user_4bcf8e7a")).unwrap_err().to_string(),
            "Could not determine home directory."
        );
    }
}
