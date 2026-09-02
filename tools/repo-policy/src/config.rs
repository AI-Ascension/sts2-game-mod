// SPDX-License-Identifier: MIT

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const SUPPORTED_POLICY_VERSION: usize = 1;

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
        let mut ignored_directories = BTreeSet::new();
        let mut ignored_path_prefixes = BTreeSet::new();
        let mut exemptions = BTreeMap::new();
        let mut limits = BTreeMap::new();
        let mut version = None;

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
                    ignored_directories = parse_array(value, key)?.into_iter().collect();
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
                "exemptions" => {
                    exemptions.insert(parse_string(key)?, parse_string(value)?);
                }
                _ => {}
            }
        }

        if version != Some(SUPPORTED_POLICY_VERSION) {
            return Err(format!(
                "policy_version must be {SUPPORTED_POLICY_VERSION}, found {version:?}"
            ));
        }
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
mod tests {
    use super::{Policy, SizeCategory};

    #[test]
    fn parses_required_paths_and_limits() -> Result<(), String> {
        let text = r#"
policy_version = 1
[project]
required_files = ["README.md"]
ignored_directories = ["target"]
ignored_path_prefixes = []
[limits]
rust_production_preferred = 10
rust_production_max = 20
rust_test_preferred = 10
rust_test_max = 20
csharp_production_preferred = 10
csharp_production_max = 20
csharp_test_preferred = 10
csharp_test_max = 20
workflow_preferred = 10
workflow_max = 20
markdown_preferred = 10
markdown_max = 20
[exemptions]
"docs/generated.md" = "A deliberately retained generated fixture."
"#;
        let policy = Policy::parse(text)?;
        assert_eq!(policy.required_files, ["README.md"]);
        assert_eq!(policy.budget(SizeCategory::Markdown).maximum, 20);
        Ok(())
    }
}
