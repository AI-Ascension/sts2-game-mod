// SPDX-License-Identifier: MIT

use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::Finding;
use crate::files::relative_text;

pub(crate) fn findings(root: &Path, files: &[PathBuf]) -> Vec<Finding> {
    let mut findings = Vec::new();
    for path in files
        .iter()
        .filter(|path| path.extension().is_some_and(|value| value == "md"))
    {
        let relative = relative_text(root, path);
        match fs::read_to_string(path) {
            Ok(text) => check_links(path, &relative, &text, &mut findings),
            Err(error) => findings.push(Finding::error(
                "DOC002",
                &relative,
                format!("cannot read Markdown: {error}"),
            )),
        }
    }
    findings
}

fn check_links(path: &Path, relative: &str, text: &str, findings: &mut Vec<Finding>) {
    for target in link_targets(text) {
        let local = target.split('#').next().unwrap_or_default();
        if local.is_empty()
            || local.starts_with("http://")
            || local.starts_with("https://")
            || local.starts_with("mailto:")
        {
            continue;
        }
        let resolved = path.parent().unwrap_or_else(|| Path::new("")).join(local);
        if !resolved.exists() {
            findings.push(Finding::error(
                "DOC002",
                relative,
                format!("local link target does not exist: {local}"),
            ));
        }
    }
    if text.contains("](") && link_targets(text).next().is_none() {
        findings.push(Finding::error(
            "DOC002",
            relative,
            "could not parse a Markdown link",
        ));
    }
}

fn link_targets(text: &str) -> impl Iterator<Item = &str> {
    text.match_indices("](").filter_map(|(start, _)| {
        let remainder = &text[start + 2..];
        let end = remainder.find(')')?;
        Some(remainder[..end].trim().trim_matches(['<', '>']))
    })
}

#[cfg(test)]
mod tests {
    use super::link_targets;

    #[test]
    fn extracts_link_targets() {
        let links: Vec<_> =
            link_targets("[guide](docs/guide.md) [web](https://example.test)").collect();
        assert_eq!(links, ["docs/guide.md", "https://example.test"]);
    }
}
