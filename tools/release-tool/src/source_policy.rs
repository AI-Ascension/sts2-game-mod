// SPDX-License-Identifier: MIT

mod contract;
mod inventory;
mod output;

use std::path::Path;

use crate::common::{fail, read_bounded, regular_file_path};

const POLICY_MAX_BYTES: u64 = 1_048_576;
const INVENTORY_MAX_BYTES: u64 = 16_777_216;

pub fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 5 {
        return fail(
            "usage: validate-source-policy <policy-file> <git-source-paths> \
             <allowed-output> <excluded-output> <required-output>",
        );
    }
    let policy_path = regular_file_path(Path::new(&args[0]), "source policy")?;
    let source_paths_path = regular_file_path(Path::new(&args[1]), "Git source path inventory")?;
    let policy = contract::parse(&read_bounded(
        &policy_path,
        "source policy",
        POLICY_MAX_BYTES,
    )?)?;
    let source_paths = inventory::parse(&read_bounded(
        &source_paths_path,
        "Git source path inventory",
        INVENTORY_MAX_BYTES,
    )?)?;
    inventory::compare(&source_paths, &policy.allowed, &policy.excluded)?;

    output::write_lists(
        &args[2],
        &args[3],
        &args[4],
        &policy.allowed,
        &policy.excluded,
        &policy.required,
    )?;
    println!("{}", output::excluded_json(&policy.excluded)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use std::fs;

    use serde_json::json;

    use super::run;

    fn fixture_root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!(
            "sts2-release-source-policy-{}",
            crate::common::temp_suffix()
        ));
        fs::create_dir(&root).expect("fixture root");
        root
    }

    #[test]
    fn command_writes_sorted_lists_and_compact_exclusions() {
        let root = fixture_root();
        let policy = root.join("policy.json");
        let inventory = root.join("source-paths");
        let allowed = root.join("allowed");
        let excluded = root.join("excluded");
        let required = root.join("required");
        let value = json!({
            "schema_version": "ai-ascension-source-distribution-policy-v1",
            "policy_id": "source-distribution-v1",
            "artifact_kind": "source_bundle",
            "artifact_scope": "production_source_only",
            "selection_mode": "exact_tracked_path_allowlist",
            "allow_only_regular_files": true,
            "allowed_paths": ["a.txt", "b.txt"],
            "excluded_paths": ["diagnostics/x.cs"],
            "required_paths": ["a.txt"],
            "review_note": "unrelated properties remain permitted"
        });
        fs::write(&policy, serde_json::to_vec(&value).expect("policy JSON")).expect("policy");
        fs::write(&inventory, b"a.txt\nb.txt\ndiagnostics/x.cs\n").expect("inventory");
        let args = vec![
            policy.display().to_string(),
            inventory.display().to_string(),
            allowed.display().to_string(),
            excluded.display().to_string(),
            required.display().to_string(),
        ];
        run(&args).expect("policy command");
        assert_eq!(
            fs::read_to_string(allowed).expect("allowed"),
            "a.txt\nb.txt\n"
        );
        assert_eq!(
            fs::read_to_string(excluded).expect("excluded"),
            "diagnostics/x.cs\n"
        );
        assert_eq!(fs::read_to_string(required).expect("required"), "a.txt\n");
        fs::remove_dir_all(root).expect("fixture cleanup");
    }

    #[test]
    fn command_writes_empty_lists_without_blank_paths() {
        let root = fixture_root();
        let policy = root.join("policy.json");
        let inventory = root.join("source-paths");
        let allowed = root.join("allowed");
        let excluded = root.join("excluded");
        let required = root.join("required");
        let value = json!({
            "schema_version": "ai-ascension-source-distribution-policy-v1",
            "policy_id": "source-distribution-v1",
            "artifact_kind": "source_bundle",
            "artifact_scope": "production_source_only",
            "selection_mode": "exact_tracked_path_allowlist",
            "allow_only_regular_files": true,
            "allowed_paths": ["a.txt"],
            "excluded_paths": [],
            "required_paths": []
        });
        fs::write(&policy, serde_json::to_vec(&value).expect("policy JSON")).expect("policy");
        fs::write(&inventory, b"a.txt\n").expect("inventory");
        let args = vec![
            policy.display().to_string(),
            inventory.display().to_string(),
            allowed.display().to_string(),
            excluded.display().to_string(),
            required.display().to_string(),
        ];
        run(&args).expect("empty policy command");
        assert_eq!(fs::read_to_string(&allowed).expect("allowed"), "a.txt\n");
        for path in [&excluded, &required] {
            assert!(fs::read(path).expect("output").is_empty());
        }
        fs::remove_dir_all(root).expect("fixture cleanup");
    }
}
