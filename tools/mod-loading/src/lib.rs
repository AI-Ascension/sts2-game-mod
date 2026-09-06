// SPDX-License-Identifier: MIT

//! Offline preparation of the intended addon's native mod-loading settings.
//! The launch owner must keep the selected game stopped throughout an apply operation.

use serde_json::{Value, json};

mod storage;
mod unique_json;
pub use storage::{apply, inspect};

/// The only addon this tool can enable.
pub const ADDON_ID: &str = "AIAscensionSTS2GameMod";

/// A reviewed candidate, without profile paths or game content in its public report.
#[derive(Debug)]
pub struct Plan {
    before: Vec<u8>,
    after: Vec<u8>,
    /// Whether the mod-loading values need to change.
    pub changed: bool,
    /// Digest used to fence a subsequent apply operation.
    pub before_sha256: String,
    /// Digest of the proposed settings bytes.
    pub after_sha256: String,
}

/// Validate the known settings shape and preserve unrelated JSON values.
pub fn prepare(bytes: &[u8]) -> Result<Plan, String> {
    let mut root = unique_json::parse(bytes)?;
    let object = root
        .as_object_mut()
        .ok_or("settings must be a JSON object")?;
    let before_mods = object.get("mod_settings").cloned().unwrap_or(Value::Null);
    let mut mods = if before_mods.is_null() {
        json!({"mods_enabled":false,"mod_list":[]})
    } else {
        before_mods.clone()
    };
    let settings = mods
        .as_object_mut()
        .ok_or("unsupported mod settings shape")?;
    if settings.len() != 2
        || !settings.contains_key("mods_enabled")
        || !settings.contains_key("mod_list")
    {
        return Err("unsupported mod settings fields".into());
    }
    let agreed = settings["mods_enabled"]
        .as_bool()
        .ok_or("invalid mod-loading consent")?;
    let entries = settings
        .get_mut("mod_list")
        .and_then(Value::as_array_mut)
        .ok_or("invalid mod list")?;
    if entries.len() > 256 {
        return Err("mod list exceeds bound".into());
    }
    let mut identities = std::collections::HashSet::new();
    let mut intended = None;
    for (index, entry) in entries.iter().enumerate() {
        let item = entry.as_object().ok_or("invalid mod-list entry")?;
        if item.len() != 3 {
            return Err("unsupported mod-list entry fields".into());
        }
        let id = item
            .get("id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty() && id.len() <= 256)
            .ok_or("invalid mod identity")?;
        let source = item
            .get("source")
            .and_then(Value::as_str)
            .filter(|source| matches!(*source, "mods_directory" | "steam_workshop" | "none"))
            .ok_or("unsupported mod source")?;
        let enabled = item
            .get("is_enabled")
            .and_then(Value::as_bool)
            .ok_or("invalid mod enabled flag")?;
        if !identities.insert((id.to_owned(), source.to_owned())) {
            return Err("duplicate mod identity".into());
        }
        if id == ADDON_ID && source == "mods_directory" {
            intended = Some(index);
        } else if !agreed && enabled {
            return Err("global consent would also enable another listed addon".into());
        }
    }
    match intended {
        Some(index) => entries[index]["is_enabled"] = json!(true),
        None if entries.len() < 256 => {
            entries.push(json!({"id":ADDON_ID,"source":"mods_directory","is_enabled":true}))
        }
        None => return Err("mod list has no room for the intended addon".into()),
    }
    settings.insert("mods_enabled".into(), json!(true));
    let changed = before_mods != mods;
    object.insert("mod_settings".into(), mods);
    let after = if changed {
        let mut encoded = serde_json::to_vec_pretty(&root).map_err(|_| "cannot encode settings")?;
        encoded.push(b'\n');
        encoded
    } else {
        bytes.to_vec()
    };
    if after.len() > 1024 * 1024 {
        return Err("prepared settings exceed byte bound".into());
    }
    Ok(Plan {
        before_sha256: storage::digest(bytes),
        after_sha256: storage::digest(&after),
        before: bytes.to_vec(),
        after,
        changed,
    })
}
