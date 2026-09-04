// SPDX-License-Identifier: MIT

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::diagnostic::Finding;
use crate::files::relative_text;

pub(crate) fn findings(root: &Path, files: &[PathBuf]) -> Vec<Finding> {
    let mut result = Vec::new();
    let mut ids = BTreeMap::new();
    for path in files {
        let relative = relative_text(root, path);
        if !relative.starts_with("docs/decisions/")
            || path.extension().is_none_or(|ext| ext != "md")
        {
            continue;
        }
        let Ok(text) = fs::read_to_string(path) else {
            // Markdown's DOC002 check already reports read failures.
            continue;
        };
        if let Err(message) = check(&relative, &text, &mut ids) {
            result.push(Finding::error("DOC003", &relative, message));
        }
    }
    result
}

fn check(path: &str, text: &str, ids: &mut BTreeMap<String, String>) -> Result<(), String> {
    let name = path.rsplit('/').next().unwrap_or_default();
    if name == "README.md" {
        return Ok(());
    }
    let heading = text.lines().next().unwrap_or_default();
    if heading.starts_with("# Moved: ") {
        if text.lines().count() > 6
            || text.lines().skip(1).any(|line| line.starts_with('#'))
            || !text.contains("](")
        {
            return Err("decision redirect must be a thin link without a normative body".into());
        }
        return Ok(());
    }
    let Some(id) = heading
        .strip_prefix("# ADR ")
        .and_then(|value| value.split_once(": "))
        .map(|(id, _)| id)
    else {
        return Err("decision must begin with # ADR NNNN: or # Moved:".into());
    };
    if id.len() != 4
        || !id.bytes().all(|byte| byte.is_ascii_digit())
        || !name.starts_with(&format!("{id}-"))
    {
        return Err("decision identifier must be four digits matching its filename".into());
    }
    if let Some(previous) = ids.insert(id.to_owned(), path.to_owned()) {
        return Err(format!(
            "duplicate decision identifier {id}, also used by {previous}"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::check;
    use std::collections::BTreeMap;

    #[test]
    fn decisions_require_unique_matching_identifiers() {
        let mut ids = BTreeMap::new();
        assert!(
            check(
                "docs/decisions/0011-settings.md",
                "# ADR 0011: Settings",
                &mut ids
            )
            .is_ok()
        );
        assert!(
            check(
                "docs/decisions/0011-other.md",
                "# ADR 0011: Other",
                &mut ids
            )
            .is_err()
        );
        assert!(
            check(
                "docs/decisions/0014-other.md",
                "# ADR 0011: Other",
                &mut ids
            )
            .is_err()
        );
        assert!(check("docs/decisions/11-other.md", "# ADR 11: Other", &mut ids).is_err());
        assert!(check("docs/decisions/0015-other.md", "# Other", &mut ids).is_err());
    }

    #[test]
    fn redirects_and_registry_do_not_claim_identifiers() {
        let mut ids = BTreeMap::new();
        assert!(check("docs/decisions/README.md", "# Registry", &mut ids).is_ok());
        assert!(
            check(
                "docs/decisions/0011-old.md",
                "# Moved: Old\n\n[ADR 0014](0014-new.md)",
                &mut ids
            )
            .is_ok()
        );
        assert!(
            check(
                "docs/decisions/0011-bad.md",
                "# Moved: Old\n## Decision\n[ADR 0014](0014-new.md)",
                &mut ids
            )
            .is_err()
        );
        assert!(ids.is_empty());
    }
}
