// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path};

const LEGACY_POLICY_VERSION: usize = 1;
const CURRENT_POLICY_VERSION: usize = 2;
const MANAGED_STANDARDS_PATH: &str = "standards/tools/standards-sync";
const KNOWN_RULES: &[&str] = &[
    "BOUND001", "CFG001", "DOC001", "DOC002", "DOC003", "EXC001", "LANG001", "LIC001", "LIC002",
    "LIC003", "RUST001", "RUST002", "RUST003", "RUST004", "RUST005", "SIZE001", "WF001", "WF002",
    "WF003", "WF004", "WF005",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SizeCategory {
    RustProduction,
    RustTest,
    CsharpProduction,
    CsharpTest,
    Workflow,
    Markdown,
}

impl SizeCategory {
    fn key(self) -> &'static str {
        match self {
            Self::RustProduction => "rust_production",
            Self::RustTest => "rust_test",
            Self::CsharpProduction => "csharp_production",
            Self::CsharpTest => "csharp_test",
            Self::Workflow => "workflow",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct Budget {
    pub(crate) preferred: usize,
    pub(crate) maximum: usize,
}

#[derive(Debug)]
pub(crate) struct Policy {
    pub(crate) policy_version: usize,
    pub(crate) advisory_rules: BTreeSet<String>,
    pub(crate) required_files: Vec<String>,
    pub(crate) ignored_directories: BTreeSet<String>,
    pub(crate) ignored_path_prefixes: BTreeSet<String>,
    pub(crate) exemptions: BTreeMap<String, String>,
    limits: BTreeMap<String, Budget>,
}

impl Policy {
    pub(crate) fn load(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path)
            .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        Self::parse(&text)
    }

    pub(crate) fn budget(&self, category: SizeCategory) -> Budget {
        self.limits.get(category.key()).copied().unwrap_or(Budget {
            preferred: usize::MAX,
            maximum: usize::MAX,
        })
    }

    fn parse(text: &str) -> Result<Self, String> {
        let mut section = String::new();
        let mut required_files = Vec::new();
        let mut ignored_directories = Vec::new();
        let mut ignored_path_prefixes = BTreeSet::new();
        let mut exemptions = BTreeMap::new();
        let mut limits = BTreeMap::new();
        let mut version = None;
        let mut mandatory_rules = None;
        let mut advisory_rules = None;

        for raw_line in text.lines() {
            let line = raw_line
                .split_once('#')
                .map_or(raw_line, |(content, _)| content)
                .trim();
            if line.is_empty() {
                continue;
            }
            if let Some(name) = line
                .strip_prefix('[')
                .and_then(|value| value.strip_suffix(']'))
            {
                section = name.to_owned();
                continue;
            }
            let Some((key, raw_value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = raw_value.trim();
            match section.as_str() {
                "" if key == "policy_version" => version = Some(parse_number(value, key)?),
                "project" if key == "required_files" => {
                    required_files = parse_array(value, key)?;
                }
                "project" if key == "ignored_directories" => {
                    ignored_directories = parse_array(value, key)?;
                }
                "project" if key == "ignored_path_prefixes" => {
                    ignored_path_prefixes = parse_array(value, key)?.into_iter().collect();
                }
                "limits" => {
                    if key.ends_with("_preferred") {
                        limits
                            .entry(key.trim_end_matches("_preferred").to_owned())
                            .or_insert(Budget {
                                preferred: 0,
                                maximum: 0,
                            })
                            .preferred = parse_number(value, key)?;
                    } else if key.ends_with("_max") {
                        limits
                            .entry(key.trim_end_matches("_max").to_owned())
                            .or_insert(Budget {
                                preferred: 0,
                                maximum: 0,
                            })
                            .maximum = parse_number(value, key)?;
                    }
                }
                "severity" if key == "mandatory" => {
                    mandatory_rules = Some(parse_array(value, "severity.mandatory")?);
                }
                "severity" if key == "advisory" => {
                    advisory_rules = Some(parse_array(value, "severity.advisory")?);
                }
                "exemptions" => {
                    exemptions.insert(parse_string(key)?, parse_string(value)?);
                }
                _ => {}
            }
        }

        let policy_version =
            version.ok_or_else(|| "policy_version must be an integer".to_owned())?;
        if !matches!(
            policy_version,
            LEGACY_POLICY_VERSION | CURRENT_POLICY_VERSION
        ) {
            return Err(format!(
                "policy_version must be {LEGACY_POLICY_VERSION} or {CURRENT_POLICY_VERSION}, found {policy_version}"
            ));
        }
        let ignored_path_prefixes = validate_ignored_path_prefixes(&ignored_path_prefixes)?;
        validate_exact_paths(&required_files, "required_files")?;
        let ignored_directories = validate_ignored_directories(&ignored_directories)?;
        for path in exemptions.keys() {
            validate_exact_path(path, "exemptions")?;
        }
        let advisory_rules = if policy_version == LEGACY_POLICY_VERSION {
            BTreeSet::new()
        } else {
            let mandatory =
                mandatory_rules.ok_or_else(|| "severity.mandatory is missing".to_owned())?;
            validate_rule_list(&mandatory, "severity.mandatory", true)?;
            if !mandatory.iter().any(|rule| rule == "*") {
                return Err(
                    "severity.mandatory must include \"*\" as the default classification"
                        .to_owned(),
                );
            }
            let advisory_values =
                advisory_rules.ok_or_else(|| "severity.advisory is missing".to_owned())?;
            validate_rule_list(&advisory_values, "severity.advisory", false)?;
            let advisory: BTreeSet<String> = advisory_values.into_iter().collect();
            if advisory.is_empty() {
                return Err("severity.advisory must name at least one rule".to_owned());
            }
            if advisory
                .iter()
                .any(|rule| mandatory.iter().any(|item| item == rule))
            {
                return Err("severity rules cannot be both mandatory and advisory".to_owned());
            }
            advisory
        };
        for category in [
            SizeCategory::RustProduction,
            SizeCategory::RustTest,
            SizeCategory::CsharpProduction,
            SizeCategory::CsharpTest,
            SizeCategory::Workflow,
            SizeCategory::Markdown,
        ] {
            let key = category.key();
            let budget = limits
                .get(key)
                .ok_or_else(|| format!("missing limits for {key}"))?;
            if budget.preferred == 0 || budget.maximum == 0 || budget.preferred > budget.maximum {
                return Err(format!("invalid limits for {key}"));
            }
        }
        Ok(Self {
            policy_version,
            advisory_rules,
            required_files,
            ignored_directories,
            ignored_path_prefixes,
            exemptions,
            limits,
        })
    }
}

fn parse_array(value: &str, key: &str) -> Result<Vec<String>, String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| format!("{key} must be an inline array"))?;
    inner
        .split(',')
        .filter(|item| !item.trim().is_empty())
        .map(|item| parse_string(item.trim()))
        .collect()
}

fn validate_rule_list(values: &[String], key: &str, allow_default: bool) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for rule in values {
        if !seen.insert(rule.as_str()) {
            return Err(format!("{key} contains duplicate rule {rule}"));
        }
        if rule == "*" {
            if !allow_default {
                return Err(format!("{key} cannot contain the default wildcard"));
            }
        } else if !KNOWN_RULES.contains(&rule.as_str()) {
            return Err(format!("{key} contains unknown rule {rule}"));
        } else if !allow_default && rule != "SIZE001" {
            return Err(format!("{key} cannot demote mandatory rule {rule}"));
        }
    }
    Ok(())
}

fn validate_exact_paths(values: &[String], key: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        validate_exact_path(value, key)?;
        if !seen.insert(value) {
            return Err(format!("{key} contains duplicate path {value}"));
        }
    }
    Ok(())
}

fn validate_ignored_directories(values: &[String]) -> Result<BTreeSet<String>, String> {
    let mut directories = BTreeSet::new();
    for directory in values {
        let path = Path::new(directory);
        let safe = !directory.is_empty()
            && !directory.contains('\\')
            && !directory.contains('/')
            && !directory.contains('*')
            && !directory.contains('?')
            && path.components().count() == 1
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)))
            && !directory.eq_ignore_ascii_case("standards");
        if !safe {
            return Err(format!(
                "ignored_directories must contain one safe directory name and cannot hide standards: {directory}"
            ));
        }
        if !directories.insert(directory.clone()) {
            return Err(format!(
                "ignored_directories contains duplicate directory {directory}"
            ));
        }
    }
    Ok(directories)
}

fn validate_exact_path(value: &str, key: &str) -> Result<(), String> {
    let path = Path::new(value);
    if value.is_empty()
        || value.contains('\\')
        || value.starts_with('/')
        || value.ends_with('/')
        || value.contains('*')
        || value.contains('?')
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(format!(
            "{key} must contain exact repository-relative paths: {value}"
        ));
    }
    Ok(())
}

fn validate_ignored_path_prefixes(values: &BTreeSet<String>) -> Result<BTreeSet<String>, String> {
    let mut prefixes = BTreeSet::new();
    for prefix in values {
        let path = Path::new(prefix);
        let safe = !prefix.is_empty()
            && !prefix.contains('\\')
            && !prefix.starts_with('/')
            && !prefix.ends_with('/')
            && !prefix.contains('*')
            && !prefix.contains('?')
            && path
                .components()
                .all(|component| matches!(component, Component::Normal(_)));
        if !safe {
            return Err(format!(
                "ignored_path_prefixes must contain exact repository-relative paths: {prefix}"
            ));
        }
        if prefix == "standards"
            || prefix == "standards/tools"
            || MANAGED_STANDARDS_PATH.starts_with(&format!("{prefix}/"))
        {
            return Err(format!(
                "ignored_path_prefixes cannot hide the standards root; use the exact managed path {MANAGED_STANDARDS_PATH}"
            ));
        }
        if !prefixes.insert(prefix.clone()) {
            return Err(format!(
                "ignored_path_prefixes contains duplicate path {prefix}"
            ));
        }
    }
    Ok(prefixes)
}

fn parse_string(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.len() < 2 || !value.starts_with('"') || !value.ends_with('"') {
        return Err(format!("expected a double-quoted string, found {value}"));
    }
    Ok(value[1..value.len() - 1].to_owned())
}

fn parse_number(value: &str, key: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|error| format!("{key} must be a positive integer: {error}"))
        .and_then(|number| {
            (number > 0)
                .then_some(number)
                .ok_or_else(|| format!("{key} must be positive"))
        })
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
