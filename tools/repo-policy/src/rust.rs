// SPDX-License-Identifier: MIT

use std::fs;
use std::path::Path;

use crate::diagnostic::Finding;

pub(crate) fn findings(root: &Path) -> Vec<Finding> {
    if !root.join("Cargo.toml").is_file() {
        return Vec::new();
    }
    let mut findings = Vec::new();
    for required in ["Cargo.lock", "rust-toolchain.toml"] {
        if !root.join(required).is_file() {
            findings.push(Finding::error(
                "RUST001",
                required,
                "required when a Cargo workspace exists",
            ));
        }
    }
    let manifest = read(root, "Cargo.toml", &mut findings);
    let toolchain = read(root, "rust-toolchain.toml", &mut findings);
    if let Some(manifest) = manifest {
        require_text(&manifest, "[workspace]", "Cargo.toml", &mut findings);
        require_text(
            &manifest,
            "[workspace.package]",
            "Cargo.toml",
            &mut findings,
        );
        require_text(&manifest, "[workspace.lints", "Cargo.toml", &mut findings);
        for value in [
            "edition = \"2024\"",
            "rust-version = \"1.97.1\"",
            "license = \"MIT\"",
        ] {
            require_text(&manifest, value, "Cargo.toml", &mut findings);
        }
    }
    if let Some(toolchain) = toolchain {
        require_text(
            &toolchain,
            "channel = \"1.97.1\"",
            "rust-toolchain.toml",
            &mut findings,
        );
    }
    findings
}

fn read(root: &Path, relative: &str, findings: &mut Vec<Finding>) -> Option<String> {
    match fs::read_to_string(root.join(relative)) {
        Ok(text) => Some(text),
        Err(error) => {
            findings.push(Finding::error(
                "RUST001",
                relative,
                format!("cannot read Rust configuration: {error}"),
            ));
            None
        }
    }
}

fn require_text(text: &str, expected: &str, path: &str, findings: &mut Vec<Finding>) {
    if !text.contains(expected) {
        findings.push(Finding::error(
            "RUST001",
            path,
            format!("Rust configuration is missing {expected}"),
        ));
    }
}
