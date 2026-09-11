// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use super::{HostData, Inputs};
use crate::common::{
    ENTRYPOINT, MANAGED_NAME, Record, digest, fail, parse_object, read_bounded, regular_file_path,
};

pub(crate) fn fixture_inputs(
    payload: &Path,
    native: &str,
    package: &str,
) -> Result<Inputs, String> {
    let names = vec![MANAGED_NAME, ENTRYPOINT, native];
    let entries: BTreeSet<String> = fs::read_dir(payload)
        .map_err(|error| format!("cannot inspect fixture payload: {error}"))?
        .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
        .collect::<Result<_, _>>()
        .map_err(|error| format!("cannot inspect fixture payload: {error}"))?;
    if entries != names.iter().map(|name| (*name).to_owned()).collect() {
        return fail("fixture payload must contain exactly three runtime files");
    }
    for name in &names {
        let path = regular_file_path(&payload.join(name), "fixture payload")?;
        digest(&path, "fixture payload")?;
    }
    let loader = parse_object(
        &read_bounded(&payload.join(ENTRYPOINT), "fixture loader manifest", 65_536)?,
        "fixture loader manifest",
    )?;
    if loader.get("version").and_then(Value::as_str) != Some(package) {
        return fail("fixture loader manifest version does not match package version");
    }
    Ok(Inputs {
        managed: payload.join(MANAGED_NAME),
        native: payload.join(native),
        toolchain: fixture_toolchain(),
        hosts: Vec::new(),
        host_values: std::collections::BTreeMap::new(),
    })
}

fn fixture_toolchain() -> Map<String, Value> {
    Map::from_iter([
        ("dotnet_executable".into(), "fixture".into()),
        ("dotnet_path_style".into(), "fixture".into()),
        ("dotnet_version".into(), "fixture".into()),
        ("rustc_version".into(), "fixture".into()),
        ("cargo_version".into(), "fixture".into()),
    ])
}

pub(crate) fn host_records(host: &Path) -> Result<HostData, String> {
    let mut records = Vec::new();
    let mut values = std::collections::BTreeMap::new();
    for name in ["sts2.dll", "GodotSharp.dll"] {
        let (size, sha256) = digest(&host.join(name), &format!("host reference {name}"))?;
        let mut object = Map::new();
        object.insert("name".into(), name.into());
        object.insert("size_bytes".into(), size.into());
        object.insert("sha256".into(), sha256.clone().into());
        records.push(Value::Object(object));
        values.insert(name.into(), (size, sha256));
    }
    Ok(HostData { records, values })
}

pub(crate) fn payload_records(payload: &Path, native: &str) -> Result<Vec<Record>, String> {
    [MANAGED_NAME, ENTRYPOINT, native]
        .iter()
        .map(|name| {
            let (size_bytes, sha256) = digest(&payload.join(name), "receipt payload")?;
            Ok(Record {
                path: (*name).into(),
                size_bytes,
                sha256,
            })
        })
        .collect()
}
