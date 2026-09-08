// SPDX-License-Identifier: MIT

use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;

use serde_json::{Map, Value};

use super::build_support::{self, Args, Inputs};
use super::contract::native_name;
use crate::common::{
    ENTRYPOINT, MANAGED_NAME, RECEIPT_SCHEMA, Record, absolute, copy_bytes, copy_checked, digest,
    fail, move_no_replace, record_value, temp_suffix, write_new,
};

pub fn run(arguments: &[String]) -> Result<(), String> {
    let args = build_support::parse_args(arguments)?;
    let repo = build_support::repository()?;
    let production = args.scope == "production";
    let (source_commit, source_tree) =
        build_support::checkout_identity(&repo, &args.source_commit, production)?;
    let (loader_bytes, loader_version) = build_support::source_loader(&repo, &source_commit)?;
    if loader_version != args.package_version {
        return fail("package version does not match selected loader manifest version");
    }
    let native = native_name(&args.platform)
        .ok_or_else(|| format!("unsupported platform: {}", args.platform))?;
    let host = args
        .host_data_dir
        .as_deref()
        .map(|value| build_support::input_dir(value, "host data directory", true))
        .transpose()?;
    let fixture = args
        .fixture_payload_dir
        .as_deref()
        .map(|value| build_support::input_dir(value, "fixture payload", true))
        .transpose()?;
    let output = absolute(&args.output_dir, "output directory")?;
    let evidence = match &args.evidence_dir {
        Some(value) => absolute(value, "evidence directory")?,
        None => output.parent().unwrap_or(Path::new(".")).join(format!(
            ".{}.evidence-{}-{}",
            output
                .file_name()
                .map_or_else(|| "receipt".into(), |name| name.to_string_lossy()),
            std::process::id(),
            temp_suffix(),
        )),
    };
    build_support::validate_roots(&output, &evidence, &repo, host.as_deref())?;
    let host_before = if production {
        build_support::host_records(host.as_deref().ok_or("production host is missing")?)?.values
    } else {
        std::collections::BTreeMap::new()
    };
    build_support::prepare(&output, &evidence)?;
    let logs = evidence.join("logs");
    let scratch = evidence.join(format!("scratch-{}", temp_suffix()));
    fs::create_dir(&scratch).map_err(|error| format!("cannot create receipt scratch: {error}"))?;
    build_support::set_private(&scratch)?;
    let stage = scratch.join("receipt-root");
    let stage_payload = stage.join("payload");
    fs::create_dir(&stage).map_err(|error| format!("cannot create receipt stage: {error}"))?;
    fs::create_dir(&stage_payload)
        .map_err(|error| format!("cannot create receipt payload stage: {error}"))?;
    build_support::set_private(&stage)?;
    build_support::set_private(&stage_payload)?;
    let claim_path = std::path::PathBuf::from(format!("{}.claim", output.display()));
    let claim = crate::common::claim(&claim_path)?;
    let mut committed = false;
    let result = (|| {
        let inputs = if production {
            let inputs = build_support::production_inputs(
                &repo,
                host.as_deref().ok_or("production host is missing")?,
                &args,
                &source_commit,
                &scratch,
                &logs,
            )?;
            if inputs.host_values != host_before {
                return fail("host reference changed during the build");
            }
            inputs
        } else {
            build_support::fixture_inputs(
                fixture.as_deref().ok_or("fixture payload is missing")?,
                native,
                &args.package_version,
            )?
        };
        let managed_digest = digest(&inputs.managed, "managed payload")?;
        copy_checked(
            &inputs.managed,
            &stage_payload.join(MANAGED_NAME),
            managed_digest.0,
            &managed_digest.1,
        )?;
        copy_bytes(
            &loader_bytes,
            &stage_payload.join(ENTRYPOINT),
            "loader payload",
        )?;
        let native_digest = digest(&inputs.native, "native payload")?;
        copy_checked(
            &inputs.native,
            &stage_payload.join(native),
            native_digest.0,
            &native_digest.1,
        )?;
        let records = build_support::payload_records(&stage_payload, native)?;
        let (final_commit, final_tree) =
            build_support::checkout_identity(&repo, &args.source_commit, production)?;
        if final_commit != source_commit || final_tree != source_tree {
            return fail("source checkout changed during the build");
        }
        let (final_loader, final_version) = build_support::source_loader(&repo, &source_commit)?;
        if final_loader != loader_bytes || final_version != args.package_version {
            return fail("selected loader manifest changed during the build");
        }
        if production {
            let final_hosts =
                build_support::host_records(host.as_deref().ok_or("production host is missing")?)?;
            if final_hosts.values != host_before {
                return fail("host reference changed before receipt publication");
            }
        }
        if env::var("STS2_RELEASE_TEST_INTERRUPT_BEFORE_PUBLISH").as_deref() == Ok("1") {
            return fail("interrupted before receipt publication");
        }
        let value = receipt_value(&args, &source_commit, &source_tree, &inputs, records);
        let mut bytes = serde_json::to_vec_pretty(&value).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        write_new(
            &stage.join("build-receipt.json"),
            &bytes,
            "build receipt",
            0o644,
        )?;
        let names: BTreeSet<String> = fs::read_dir(&stage)
            .map_err(|error| format!("cannot inspect receipt stage: {error}"))?
            .map(|entry| entry.map(|entry| entry.file_name().to_string_lossy().into_owned()))
            .collect::<Result<_, _>>()
            .map_err(|error| format!("cannot inspect receipt stage: {error}"))?;
        if names != BTreeSet::from(["build-receipt.json".into(), "payload".into()]) {
            return fail("receipt staging root contains an unexpected entry");
        }
        move_no_replace(&stage, &output, "receipt output")?;
        committed = true;
        println!("runtime receipt: {}", output.display());
        println!("source commit: {source_commit}");
        println!("private evidence: {}", evidence.display());
        Ok(())
    })();
    if committed {
        let _ = fs::remove_dir_all(&scratch);
    }
    crate::common::release_claim(&claim);
    result
}

fn receipt_value(
    args: &Args,
    commit: &str,
    tree: &str,
    inputs: &Inputs,
    payload: Vec<Record>,
) -> Value {
    let mut object = Map::new();
    object.insert("schema_version".into(), RECEIPT_SCHEMA.into());
    object.insert("scope".into(), args.scope.clone().into());
    object.insert("platform".into(), args.platform.clone().into());
    object.insert("source_commit".into(), commit.into());
    object.insert("source_tree".into(), tree.into());
    object.insert("game_version".into(), args.game_version.clone().into());
    object.insert(
        "package_version".into(),
        args.package_version.clone().into(),
    );
    object.insert(
        "loader_manifest_version".into(),
        args.package_version.clone().into(),
    );
    object.insert("toolchain".into(), Value::Object(inputs.toolchain.clone()));
    object.insert("host_references".into(), Value::Array(inputs.hosts.clone()));
    object.insert(
        "fixture_flags".into(),
        if args.scope == "fixture" {
            Value::Array(vec!["fixture_payload".into()])
        } else {
            Value::Array(Vec::new())
        },
    );
    object.insert(
        "payload".into(),
        Value::Array(payload.iter().map(record_value).collect()),
    );
    Value::Object(object)
}
