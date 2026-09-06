// SPDX-License-Identifier: MIT

#![allow(clippy::expect_used)]

use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{Value, json};
use sts2_game_mod_loading::{ADDON_ID, apply, inspect, prepare};

struct Fixture(PathBuf);

impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "sts2-mod-loading-test-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("test clock")
                .as_nanos()
        ));
        fs::create_dir(&path).expect("fixture root");
        fs::create_dir(path.join("mods")).expect("fixture mods");
        fs::create_dir(path.join("backups")).expect("fixture backups");
        fs::write(
            path.join("mods/AIAscensionSTS2GameMod.json"),
            json!({"id":ADDON_ID,
            "has_dll":true,"has_pck":false})
            .to_string(),
        )
        .expect("synthetic manifest");
        fs::write(
            path.join("mods/AIAscensionSTS2GameMod.dll"),
            b"authored synthetic addon marker",
        )
        .expect("synthetic addon file");
        fs::write(
            path.join("settings.save"),
            b"{\"audio\":{\"music\":0.25},\"mod_settings\":null}\n",
        )
        .expect("fixture settings");
        Self(path)
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).expect("remove owned test fixture");
    }
}

#[test]
fn enables_intended_addon_and_preserves_unrelated_values_and_disabled_entries() {
    let original = json!({"audio":{"music":0.25},"display":{"width":1920},
        "mod_settings":{"mods_enabled":false,"mod_list":[
            {"id":"Other","source":"steam_workshop","is_enabled":false}]}});
    let fixture = Fixture::new();
    let settings = fixture.0.join("settings.save");
    fs::write(&settings, original.to_string()).expect("fixture settings");
    let plan = inspect(&settings, &fixture.0.join("mods")).expect("review plan");
    assert!(plan.changed);
    let backup = apply(
        &settings,
        &fixture.0.join("mods"),
        &fixture.0.join("backups"),
        &plan.before_sha256,
    )
    .expect("apply")
    .expect("backup label");
    let after: Value = serde_json::from_slice(&fs::read(&settings).expect("settings readback"))
        .expect("JSON readback");
    assert_eq!(after["audio"], original["audio"]);
    assert_eq!(after["display"], original["display"]);
    assert_eq!(
        after["mod_settings"]["mod_list"][0],
        original["mod_settings"]["mod_list"][0]
    );
    assert_eq!(after["mod_settings"]["mod_list"][1]["id"], ADDON_ID);
    assert_eq!(after["mod_settings"]["mods_enabled"], true);
    assert_eq!(
        fs::read(
            fixture
                .0
                .join("backups")
                .join(backup)
                .join("settings.before.json")
        )
        .expect("backup"),
        original.to_string().as_bytes()
    );
    let repeated = inspect(&settings, &fixture.0.join("mods")).expect("repeat plan");
    assert!(!repeated.changed);
    assert!(
        apply(
            &settings,
            &fixture.0.join("mods"),
            &fixture.0.join("backups"),
            &repeated.before_sha256
        )
        .expect("repeat apply")
        .is_none()
    );
    assert_eq!(
        fs::read_dir(fixture.0.join("backups"))
            .expect("backups")
            .count(),
        1
    );
}

#[test]
fn rejects_unknown_shapes_duplicates_and_implicit_other_addon_activation() {
    for input in [
        json!([]),
        json!({"mod_settings":{"mods_enabled":true,"mod_list":[],"future":true}}),
        json!({"mod_settings":{"mods_enabled":"true","mod_list":[]}}),
        json!({"mod_settings":{"mods_enabled":false,"mod_list":[
            {"id":"Other","source":"mods_directory","is_enabled":true}]}}),
        json!({"mod_settings":{"mods_enabled":true,"mod_list":[
            {"id":ADDON_ID,"source":"mods_directory","is_enabled":false},
            {"id":ADDON_ID,"source":"mods_directory","is_enabled":true}]}}),
    ] {
        assert!(prepare(input.to_string().as_bytes()).is_err());
    }
    assert!(prepare(br#"{"mod_settings":null,"mod_settings":{}}"#).is_err());
    assert!(prepare(br#"{"audio":{"x":1,"x":2}}"#).is_err());
    assert!(prepare(&vec![b' '; 1024 * 1024 + 1]).is_err());
}

#[test]
fn refuses_drift_and_unexpected_addons_without_touching_settings() {
    let fixture = Fixture::new();
    let settings = fixture.0.join("settings.save");
    let original = fs::read(&settings).expect("fixture bytes");
    assert!(
        apply(
            &settings,
            &fixture.0.join("mods"),
            &fixture.0.join("backups"),
            &"0".repeat(64)
        )
        .is_err()
    );
    assert_eq!(fs::read(&settings).expect("unchanged settings"), original);
    assert_eq!(
        fs::read_dir(fixture.0.join("backups"))
            .expect("backups")
            .count(),
        0
    );
    fs::write(fixture.0.join("mods/Other.dll"), b"synthetic other addon").expect("other addon");
    assert!(inspect(&settings, &fixture.0.join("mods")).is_err());
    assert_eq!(fs::read(&settings).expect("unchanged settings"), original);
}

#[test]
fn existing_lock_and_backup_failure_do_not_replace_settings() {
    let fixture = Fixture::new();
    let settings = fixture.0.join("settings.save");
    let original = fs::read(&settings).expect("fixture bytes");
    let plan = inspect(&settings, &fixture.0.join("mods")).expect("plan");
    fs::create_dir(fixture.0.join(".sts2-mod-loading.lock")).expect("existing lock");
    assert!(
        apply(
            &settings,
            &fixture.0.join("mods"),
            &fixture.0.join("backups"),
            &plan.before_sha256
        )
        .is_err()
    );
    fs::remove_dir(fixture.0.join(".sts2-mod-loading.lock")).expect("remove fixture lock");
    let invalid = fixture.0.join("not-a-directory");
    fs::write(&invalid, b"fixture").expect("invalid backup target");
    assert!(
        apply(
            &settings,
            &fixture.0.join("mods"),
            &invalid,
            &plan.before_sha256
        )
        .is_err()
    );
    assert_eq!(fs::read(&settings).expect("unchanged settings"), original);
    assert!(!fixture.0.join(".sts2-mod-loading.lock").exists());
}

#[test]
fn cli_requires_explicit_apply_and_digest_and_reports_only_the_owned_change() {
    let fixture = Fixture::new();
    let settings = fixture.0.join("settings.save");
    let original = fs::read(&settings).expect("fixture bytes");
    let invoke = |extra: &[&str]| {
        std::process::Command::new(env!("CARGO_BIN_EXE_sts2-game-mod-loading"))
            .arg("--settings-file")
            .arg(&settings)
            .arg("--mods-dir")
            .arg(fixture.0.join("mods"))
            .args(extra)
            .output()
            .expect("CLI invocation")
    };
    let output = invoke(&[]);
    assert!(output.status.success());
    let plan: Value = serde_json::from_slice(&output.stdout).expect("plan report");
    assert_eq!(plan["mode"], "plan");
    assert_eq!(fs::read(&settings).expect("read-only plan"), original);
    assert!(!invoke(&["--apply"]).status.success());
    assert_eq!(fs::read(&settings).expect("missing apply inputs"), original);
    let backups = fixture.0.join("backups");
    let output = invoke(&[
        "--apply",
        "--backup-dir",
        backups.to_str().expect("test path"),
        "--expected-sha256",
        plan["before_sha256"].as_str().expect("plan digest"),
    ]);
    assert!(output.status.success());
    let applied: Value = serde_json::from_slice(&output.stdout).expect("apply report");
    assert_eq!(applied["mode"], "applied");
    assert_eq!(applied["addon_id"], ADDON_ID);
    assert!(
        !String::from_utf8(output.stdout)
            .expect("report UTF-8")
            .contains("audio")
    );
}

#[cfg(unix)]
#[test]
fn rejects_linked_settings_and_preserves_file_permissions() {
    use std::os::unix::fs::{MetadataExt, PermissionsExt, symlink};
    let fixture = Fixture::new();
    let settings = fixture.0.join("settings.save");
    let link = fixture.0.join("linked.save");
    symlink(&settings, &link).expect("fixture symlink");
    assert!(inspect(&link, &fixture.0.join("mods")).is_err());
    fs::set_permissions(&settings, fs::Permissions::from_mode(0o640)).expect("fixture mode");
    let before = fs::metadata(&settings).expect("original ownership");
    let plan = inspect(&settings, &fixture.0.join("mods")).expect("plan");
    apply(
        &settings,
        &fixture.0.join("mods"),
        &fixture.0.join("backups"),
        &plan.before_sha256,
    )
    .expect("apply");
    assert_eq!(
        fs::metadata(&settings).expect("mode").permissions().mode() & 0o777,
        0o640
    );
    let after = fs::metadata(&settings).expect("preserved ownership");
    assert_eq!((before.uid(), before.gid()), (after.uid(), after.gid()));
}
