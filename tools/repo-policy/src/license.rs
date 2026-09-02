// SPDX-License-Identifier: MIT

use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::Finding;
use crate::files::relative_text;

pub(crate) fn findings(root: &Path, files: &[PathBuf]) -> Vec<Finding> {
    let mut findings = Vec::new();
    check_root_license(root, &mut findings);
    for path in files {
        let relative = relative_text(root, path);
        let extension = path.extension().and_then(|value| value.to_str());
        if extension == Some("rs") || extension == Some("cs") {
            check_source_header(path, &relative, &mut findings);
        }
        if path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml") {
            check_manifest(path, &relative, &mut findings);
        }
    }
    findings
}

fn check_root_license(root: &Path, findings: &mut Vec<Finding>) {
    match fs::read_to_string(root.join("LICENSE")) {
        Ok(text) if text.contains("MIT License") => {}
        Ok(_) => findings.push(Finding::error(
            "LIC001",
            "LICENSE",
            "root license must declare MIT License",
        )),
        Err(error) => findings.push(Finding::error(
            "LIC001",
            "LICENSE",
            format!("cannot read root license: {error}"),
        )),
    }
}

fn check_source_header(path: &Path, relative: &str, findings: &mut Vec<Finding>) {
    match fs::read_to_string(path) {
        Ok(text)
            if text
                .lines()
                .take(5)
                .any(|line| line.contains("SPDX-License-Identifier: MIT")) => {}
        Ok(_) => findings.push(Finding::error(
            "LIC002",
            relative,
            "Rust and managed source needs an SPDX MIT header",
        )),
        Err(error) => findings.push(Finding::error(
            "LIC002",
            relative,
            format!("cannot read source: {error}"),
        )),
    }
}

fn check_manifest(path: &Path, relative: &str, findings: &mut Vec<Finding>) {
    let Ok(text) = fs::read_to_string(path) else {
        findings.push(Finding::error(
            "LIC003",
            relative,
            "cannot read Cargo manifest",
        ));
        return;
    };
    let acceptable = path.file_name().and_then(|value| value.to_str()) == Some("Cargo.toml")
        && (text.contains("license = \"MIT\"")
            || text.contains("license.workspace = true")
            || text.contains("license = { workspace = true"));
    if !acceptable {
        findings.push(Finding::error(
            "LIC003",
            relative,
            "Cargo manifest must declare or inherit the MIT license",
        ));
    }
}
