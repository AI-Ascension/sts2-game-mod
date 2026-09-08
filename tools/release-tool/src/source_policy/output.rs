// SPDX-License-Identifier: MIT

use std::path::Path;

use serde_json::Value;

use crate::common::{absolute, fail, write_new};

pub(crate) fn write_lists(
    allowed_path: &str,
    excluded_path: &str,
    required_path: &str,
    allowed: &[String],
    excluded: &[String],
    required: &[String],
) -> Result<(), String> {
    let allowed_path = absolute(allowed_path, "allowed source path inventory")?;
    let excluded_path = absolute(excluded_path, "excluded source path inventory")?;
    let required_path = absolute(required_path, "required source path inventory")?;
    if allowed_path == excluded_path
        || allowed_path == required_path
        || excluded_path == required_path
    {
        return fail("source path inventory outputs must be distinct");
    }
    write_list(&allowed_path, "allowed source path inventory", allowed)?;
    write_list(&excluded_path, "excluded source path inventory", excluded)?;
    write_list(&required_path, "required source path inventory", required)
}

fn write_list(path: &Path, label: &str, values: &[String]) -> Result<(), String> {
    let mut data = values.join("\n");
    data.push('\n');
    write_new(path, data.as_bytes(), label, 0o644)
}

pub(crate) fn excluded_json(paths: &[String]) -> Result<String, String> {
    serde_json::to_string(&Value::Array(
        paths.iter().cloned().map(Value::String).collect(),
    ))
    .map_err(|error| format!("cannot encode excluded source paths: {error}"))
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used, clippy::unwrap_used)]

    use super::excluded_json;

    #[test]
    fn emits_compact_json() {
        assert_eq!(
            excluded_json(&["a.txt".into(), "nested/x.cs".into()]).expect("JSON"),
            r#"["a.txt","nested/x.cs"]"#
        );
    }
}
