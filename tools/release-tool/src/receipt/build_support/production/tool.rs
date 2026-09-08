// SPDX-License-Identifier: MIT

use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::super::Tool;
use crate::common::{fail, regular_file_path, safe_text};

pub(super) fn resolve_dotnet(value: &str) -> Result<Tool, String> {
    let candidate = if value.contains('/') || value.contains('\\') {
        if crate::common::windows_path(value) {
            let output = Command::new("wslpath")
                .args(["-u", "--", value])
                .output()
                .map_err(|error| {
                    format!("could not convert dotnet executable with wslpath: {error}")
                })?;
            let converted = String::from_utf8(output.stdout)
                .map_err(|_| "could not convert dotnet executable with wslpath".to_owned())?
                .trim()
                .to_owned();
            if !output.status.success() || converted.is_empty() || converted.contains('\n') {
                return fail("could not convert dotnet executable with wslpath");
            }
            PathBuf::from(converted)
        } else {
            PathBuf::from(value)
        }
    } else {
        let output = Command::new("which")
            .arg(value)
            .output()
            .map_err(|error| format!("selected dotnet executable is unavailable: {error}"))?;
        if !output.status.success() {
            return fail("selected dotnet executable is unavailable");
        }
        PathBuf::from(String::from_utf8_lossy(&output.stdout).trim())
    };
    let resolved = fs::canonicalize(&candidate)
        .map_err(|error| format!("selected dotnet executable cannot be resolved: {error}"))?;
    regular_file_path(&resolved, "dotnet executable")?;
    if resolved
        .metadata()
        .map_err(|error| error.to_string())?
        .permissions()
        .mode()
        & 0o111
        == 0
    {
        return fail("dotnet executable is not executable");
    }
    let style = if resolved
        .extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("exe"))
    {
        "windows"
    } else {
        "posix"
    };
    Ok(Tool {
        path: resolved,
        style,
    })
}

pub(super) fn dotnet_path(tool: &Tool, path: &Path, label: &str) -> Result<String, String> {
    if tool.style == "posix" {
        return Ok(path.display().to_string());
    }
    let output = Command::new("wslpath")
        .args(["-w", "--"])
        .arg(path)
        .output()
        .map_err(|error| format!("could not convert {label} for Windows dotnet.exe: {error}"))?;
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if !output.status.success() || value.is_empty() || value.contains('\n') {
        return fail(format!("could not convert {label} for Windows dotnet.exe"));
    }
    Ok(value)
}

pub(super) fn command_output(command: &mut Command, label: &str) -> Result<String, String> {
    let output = command
        .output()
        .map_err(|error| format!("{label} could not start: {error}"))?;
    if !output.status.success() {
        return fail(format!("{label} failed"));
    }
    let value = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    if value.is_empty() || value.contains('\n') {
        return fail(format!("{label} returned an invalid toolchain identity"));
    }
    safe_text(&value, label)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::resolve_dotnet;
    use std::fs;
    use std::os::unix::fs::{PermissionsExt, symlink};

    #[test]
    fn resolves_symlinked_dotnet_exe() {
        let root = std::env::temp_dir().join(format!(
            "sts2-release-tool-dotnet-{}",
            crate::common::temp_suffix()
        ));
        fs::create_dir(&root).expect("fixture root");
        let executable = root.join("dotnet.exe");
        fs::write(&executable, b"#!/bin/sh\nexit 0\n").expect("fixture executable");
        fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
            .expect("fixture permissions");
        let link = root.join("dotnet-link");
        symlink(&executable, &link).expect("fixture link");
        let direct = resolve_dotnet(executable.to_str().expect("path")).expect("direct tool");
        let linked = resolve_dotnet(link.to_str().expect("link path")).expect("linked tool");
        assert_eq!(direct.path, linked.path);
        assert_eq!(direct.style, "windows");
        assert_eq!(linked.style, "windows");
        fs::remove_dir_all(root).expect("fixture cleanup");
    }
}
