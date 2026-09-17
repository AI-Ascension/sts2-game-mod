// SPDX-License-Identifier: MIT

use super::*;

pub(super) fn optional_string_list(json: &str, name: &str) -> Result<Option<Vec<String>>, String> {
    let value: Value = serde_json::from_str(json).map_err(|_| "semantic".to_owned())?;
    let Some(field) = value.get(name) else {
        return Ok(None);
    };
    let Some(values) = field.as_array() else {
        return Err(format!("{name} is not an array"));
    };
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{name} contains a non-string"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

pub(super) struct SourceManifest {
    pub(super) snapshot: sts2_game_mod::ContentCatalogSnapshot,
}

impl sts2_game_mod::ContentCatalogSource for SourceManifest {
    fn read_catalog(
        &self,
    ) -> Result<sts2_game_mod::ContentCatalogSnapshot, sts2_game_mod::ContentSourceError> {
        Ok(self.snapshot.clone())
    }
}
