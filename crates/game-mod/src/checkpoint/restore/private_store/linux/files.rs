// SPDX-License-Identifier: MIT

use std::ffi::CString;
use std::fs::File;
use std::os::fd::{AsRawFd, FromRawFd, RawFd};
use std::os::unix::ffi::OsStrExt;
use std::path::{Component, Path};

use crate::checkpoint::restore::ExactRestoreError;

pub(super) fn open_private_directory(path: &Path) -> Result<File, ExactRestoreError> {
    if !path.is_absolute() {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let root_name = c_name(b"/")?;
    // SAFETY: The fixed root path is a valid NUL-terminated C string; returned descriptor is
    // checked and immediately owned by `File`.
    let root_fd = unsafe {
        libc::open(
            root_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC,
        )
    };
    let mut current = file_from_fd(root_fd)?;
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::RootDir => None,
            Component::Normal(value) => Some(value.to_owned()),
            Component::CurDir | Component::ParentDir | Component::Prefix(_) => {
                Some(std::ffi::OsString::new())
            }
        })
        .collect::<Vec<_>>();
    if components.is_empty() || components.iter().any(|part| part.is_empty()) {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    for (index, component) in components.iter().enumerate() {
        let name = CString::new(component.as_bytes())
            .map_err(|_| ExactRestoreError::StorageUnavailable)?;
        let last = index + 1 == components.len();
        let mut fd = unsafe {
            libc::openat(
                current.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if fd < 0 && last && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound
        {
            // SAFETY: The component name is NUL-free and the held parent descriptor is a
            // directory opened without following symlinks. mkdirat creates only this child.
            if unsafe { libc::mkdirat(current.as_raw_fd(), name.as_ptr(), 0o700) } != 0 {
                return Err(ExactRestoreError::StorageUnavailable);
            }
            fd = unsafe {
                libc::openat(
                    current.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
                )
            };
        }
        current = file_from_fd(fd)?;
    }
    let metadata = stat_file(&current)?;
    check_private_directory(&metadata)?;
    Ok(current)
}

pub(super) fn open_regular_file(
    root: RawFd,
    name: &str,
    create: bool,
) -> Result<File, ExactRestoreError> {
    if !safe_filename(name) {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let name = CString::new(name).map_err(|_| ExactRestoreError::StorageUnavailable)?;
    let mut flags = libc::O_RDWR | libc::O_CLOEXEC | libc::O_NOFOLLOW;
    if create {
        flags |= libc::O_CREAT;
    }
    // SAFETY: The name is a validated single path component and root is a retained private
    // directory descriptor. O_NOFOLLOW prevents opening a symlink replacement.
    let file = unsafe { libc::openat(root, name.as_ptr(), flags, 0o600) };
    let file = file_from_fd(file)?;
    let metadata = stat_file(&file)?;
    check_private_regular(&metadata)?;
    Ok(file)
}

pub(super) fn open_new_regular_file(root: RawFd, name: &str) -> Result<File, ExactRestoreError> {
    if !safe_filename(name) {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let name = CString::new(name).map_err(|_| ExactRestoreError::StorageUnavailable)?;
    // SAFETY: The name is a validated single component relative to a retained private directory.
    // O_EXCL prevents a pre-existing temporary entry from being reused.
    let file = unsafe {
        libc::openat(
            root,
            name.as_ptr(),
            libc::O_RDWR | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    let file = file_from_fd(file)?;
    let metadata = stat_file(&file)?;
    check_private_regular(&metadata)?;
    Ok(file)
}

pub(super) fn open_regular_file_optional(
    root: RawFd,
    name: &str,
) -> Result<Option<File>, ExactRestoreError> {
    if !safe_filename(name) {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let name = CString::new(name).map_err(|_| ExactRestoreError::StorageUnavailable)?;
    // SAFETY: The name is one validated path component; O_NOFOLLOW binds the lookup to the
    // retained directory and refuses symbolic links.
    let file = unsafe {
        libc::openat(
            root,
            name.as_ptr(),
            libc::O_RDWR | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if file < 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(ExactRestoreError::StorageUnavailable);
    }
    let file = file_from_fd(file)?;
    let metadata = stat_file(&file)?;
    check_private_regular(&metadata)?;
    Ok(Some(file))
}

pub(super) fn stat_file(file: &File) -> Result<libc::stat, ExactRestoreError> {
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: `metadata` is writable output and the fd is live for the call.
    if unsafe { libc::fstat(file.as_raw_fd(), metadata.as_mut_ptr()) } != 0 {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    // SAFETY: fstat initialized the value after a successful return.
    Ok(unsafe { metadata.assume_init() })
}

pub(super) fn check_private_directory(metadata: &libc::stat) -> Result<(), ExactRestoreError> {
    if metadata.st_mode & libc::S_IFMT != libc::S_IFDIR
        || metadata.st_uid != unsafe { libc::geteuid() }
        || metadata.st_mode & 0o077 != 0
        || metadata.st_mode & 0o700 != 0o700
    {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

pub(super) fn check_private_regular(metadata: &libc::stat) -> Result<(), ExactRestoreError> {
    if metadata.st_mode & libc::S_IFMT != libc::S_IFREG
        || metadata.st_uid != unsafe { libc::geteuid() }
        || metadata.st_nlink != 1
        || metadata.st_mode & 0o077 != 0
        || metadata.st_mode & 0o600 != 0o600
    {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

fn file_from_fd(fd: RawFd) -> Result<File, ExactRestoreError> {
    if fd < 0 {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    // SAFETY: A non-negative fd returned by open/openat is uniquely transferred to File.
    Ok(unsafe { File::from_raw_fd(fd) })
}

fn c_name(bytes: &[u8]) -> Result<CString, ExactRestoreError> {
    CString::new(bytes).map_err(|_| ExactRestoreError::StorageUnavailable)
}

pub(super) fn rename_at(root: RawFd, from: &str, to: &str) -> Result<(), ExactRestoreError> {
    let from = CString::new(from).map_err(|_| ExactRestoreError::StorageUnavailable)?;
    let to = CString::new(to).map_err(|_| ExactRestoreError::StorageUnavailable)?;
    // SAFETY: Both validated names are single components relative to the retained directory.
    if unsafe { libc::renameat(root, from.as_ptr(), root, to.as_ptr()) } != 0 {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

pub(super) fn unlink_at(root: RawFd, name: &str) -> Result<(), ExactRestoreError> {
    let name = CString::new(name).map_err(|_| ExactRestoreError::StorageUnavailable)?;
    // SAFETY: The candidate is one enumerated file name from the held private directory.
    if unsafe { libc::unlinkat(root, name.as_ptr(), 0) } != 0 {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

pub(super) fn sync_directory(root: RawFd) -> Result<(), ExactRestoreError> {
    // SAFETY: `root` is a live directory file descriptor owned by PrivateDirectory.
    if unsafe { libc::fsync(root) } != 0 {
        return Err(ExactRestoreError::StorageUnavailable);
    }
    Ok(())
}

pub(super) fn data_name(key: &str) -> String {
    format!("{key}.data")
}

pub(super) fn bits_name(key: &str) -> String {
    format!("{key}.bits")
}

pub(super) fn data_name_checked(key: &str) -> Result<String, ExactRestoreError> {
    valid_key(key)
        .then(|| data_name(key))
        .ok_or(ExactRestoreError::StorageUnavailable)
}

pub(super) fn bits_name_checked(key: &str) -> Result<String, ExactRestoreError> {
    valid_key(key)
        .then(|| bits_name(key))
        .ok_or(ExactRestoreError::StorageUnavailable)
}

pub(super) fn valid_key(key: &str) -> bool {
    key.len() <= 128
        && !key.is_empty()
        && key
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

pub(super) fn valid_operation_id(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(index, byte)| match index {
            8 | 13 | 18 | 23 => byte == b'-',
            _ => byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte),
        })
}

pub(super) fn safe_filename(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 160
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
}
