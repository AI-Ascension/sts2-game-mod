// SPDX-License-Identifier: MIT

use super::{Policy, SizeCategory};

const LIMITS: &str = "rust_production_preferred = 10\nrust_production_max = 20\n\
rust_test_preferred = 10\nrust_test_max = 20\ncsharp_production_preferred = 10\n\
csharp_production_max = 20\ncsharp_test_preferred = 10\ncsharp_test_max = 20\n\
workflow_preferred = 10\nworkflow_max = 20\nmarkdown_preferred = 10\nmarkdown_max = 20";

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

#[test]
fn rejects_unknown_and_duplicate_severity_rules() {
    let base = r#"
policy_version = 2
[project]
required_files = []
ignored_directories = []
ignored_path_prefixes = []
[severity]
mandatory = ["*"]
advisory = ["SIZE001"]
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
"#;
    assert!(
        Policy::parse(&base.replace("advisory = [\"SIZE001\"]", "advisory = [\"RUST999\"]"))
            .is_err()
    );
    assert!(
        Policy::parse(&base.replace("advisory = [\"SIZE001\"]", "advisory = [\"RUST001\"]"))
            .is_err()
    );
    assert!(
        Policy::parse(&base.replace("mandatory = [\"*\"]", "mandatory = [\"*\", \"*\"]")).is_err()
    );
    assert!(
        Policy::parse(&base.replace(
            "advisory = [\"SIZE001\"]",
            "advisory = [\"SIZE001\", \"SIZE001\"]"
        ))
        .is_err()
    );
}

#[test]
fn rejects_broad_standards_ignore_but_accepts_exact_managed_path() {
    let base = r#"
policy_version = 2
[project]
required_files = []
ignored_directories = []
ignored_path_prefixes = ["standards/tools/standards-sync"]
[severity]
mandatory = ["*"]
advisory = ["SIZE001"]
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
"#;
    assert!(Policy::parse(base).is_ok());
    assert!(Policy::parse(&base.replace("standards/tools/standards-sync", "standards")).is_err());
}

#[test]
fn rejects_path_traversal_in_required_files() {
    let text = format!(
        "policy_version = 1\n[project]\nrequired_files = [\"../outside\"]\n\
         ignored_directories = []\nignored_path_prefixes = []\n\
         [limits]\n{LIMITS}\n[exemptions]\n"
    );
    assert!(Policy::parse(&text).is_err());
}

#[test]
fn rejects_unsafe_and_duplicate_ignored_directories() {
    let base = r#"
policy_version = 1
[project]
required_files = []
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
"#;
    assert!(Policy::parse(&base.replace("[\"target\"]", "[\"target\", \"target\"]")).is_err());
    assert!(Policy::parse(&base.replace("[\"target\"]", "[\"standards\"]")).is_err());
    assert!(Policy::parse(&base.replace("[\"target\"]", "[\"nested/target\"]")).is_err());
}
