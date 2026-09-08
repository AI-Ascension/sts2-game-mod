// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;

use crate::common::fail;

pub(crate) fn parse(bytes: &[u8]) -> Result<Vec<String>, String> {
    let text = String::from_utf8(bytes.to_owned())
        .map_err(|error| format!("Git source path inventory is not UTF-8: {error}"))?;
    let paths: Vec<String> = text
        .lines()
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect();
    if paths.windows(2).any(|pair| pair[0] >= pair[1]) {
        return fail("Git source path inventory is not sorted and unique");
    }
    Ok(paths)
}

pub(crate) fn compare(
    source: &[String],
    allowed: &[String],
    excluded: &[String],
) -> Result<(), String> {
    let source_set: BTreeSet<&str> = source.iter().map(String::as_str).collect();
    let mut policy_set: BTreeSet<&str> = allowed.iter().map(String::as_str).collect();
    policy_set.extend(excluded.iter().map(String::as_str));
    if source_set == policy_set {
        return Ok(());
    }
    let missing: Vec<&str> = source_set
        .difference(&policy_set)
        .copied()
        .take(8)
        .collect();
    let extra: Vec<&str> = policy_set
        .difference(&source_set)
        .copied()
        .take(8)
        .collect();
    let mut details = Vec::new();
    if !missing.is_empty() {
        details.push(format!("unlisted={}", missing.join(",")));
    }
    if !extra.is_empty() {
        details.push(format!("policy-only={}", extra.join(",")));
    }
    fail(format!(
        "source tree does not match the reviewed path policy ({})",
        details.join("; ")
    ))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::{compare, parse};

    #[test]
    fn accepts_sorted_unique_inventory_and_exact_set() {
        let source = parse(b"a\nb\n").expect("inventory");
        compare(&source, &["a".into()], &["b".into()]).expect("matching set");
    }

    #[test]
    fn rejects_unsorted_duplicate_inventory() {
        assert!(parse(b"b\na\n").is_err());
        assert!(parse(b"a\na\n").is_err());
    }

    #[test]
    fn reports_bounded_missing_and_extra_details() {
        let source = parse(b"a\nb\n").expect("inventory");
        let error = compare(&source, &["a".into()], &["c".into()]).expect_err("mismatch");
        assert!(error.contains("unlisted=b"));
        assert!(error.contains("policy-only=c"));
    }
}
