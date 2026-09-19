// SPDX-License-Identifier: MIT

//! The directory the checkpoint-restore fixtures may create scratch space in.

#![cfg(target_os = "linux")]

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Resolves the directory that owns build output, honoring `CARGO_TARGET_DIR`.
///
/// Scratch space for this suite must not require the default `<workspace>/target` to exist: build
/// output is routinely redirected to a shared or relocated directory, and requiring the default
/// location failed with an `ENOENT` that named neither the path nor the cause, and which had
/// nothing to do with the behavior under test.
///
/// A relative `CARGO_TARGET_DIR` resolves against the current directory, which is the rule Cargo
/// itself applies.
pub(crate) fn scratch_root(target_dir: Option<&OsStr>, current_dir: &Path) -> PathBuf {
    match target_dir {
        Some(dir) => {
            let dir = Path::new(dir);
            if dir.is_absolute() {
                dir.to_path_buf()
            } else {
                current_dir.join(dir)
            }
        }
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target"),
    }
}

#[cfg(test)]
mod resolution {
    use std::ffi::OsStr;
    use std::path::Path;

    use super::scratch_root;

    #[test]
    fn an_absolute_target_directory_is_used_as_given() {
        let resolved = scratch_root(
            Some(OsStr::new("/tmp/shared-cargo-output")),
            Path::new("/work/package"),
        );
        assert_eq!(resolved, Path::new("/tmp/shared-cargo-output"));
    }

    #[test]
    fn a_relative_target_directory_resolves_against_the_current_directory() {
        let resolved = scratch_root(Some(OsStr::new("alternate output")), Path::new("/work/pkg"));
        assert_eq!(resolved, Path::new("/work/pkg/alternate output"));
    }

    #[test]
    fn an_absent_target_directory_defaults_to_the_workspace_target() {
        let resolved = scratch_root(None, Path::new("/unused"));
        assert_eq!(
            resolved,
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target")
        );
    }
}
