// SPDX-License-Identifier: MIT

use std::error::Error;

use serde_json::json;
use sts2_game_mod::{
    AllowedWorkshopFile, WORKSHOP_LOADER_CONTRACT, WORKSHOP_MANIFEST_SCHEMA_VERSION,
    WORKSHOP_PACKAGE_ID, WorkshopCompatibilityError, WorkshopConsumer, WorkshopContentKind,
    WorkshopFile, WorkshopFileRole, WorkshopInstallState, WorkshopItemSnapshot, WorkshopManifest,
    WorkshopManifestError, WorkshopPackage, WorkshopPolicy, WorkshopReadiness, WorkshopReject,
    WorkshopWaitReason,
};

const APP_ID: u32 = 2_111_222;
const ITEM_ID: u64 = 9_111_222;
const GAME_VERSION: &str = "0.107.1";
const PLATFORM: &str = "windows-x86_64";

fn digest(byte: char) -> String {
    std::iter::repeat_n(byte, 64).collect()
}

fn manifest() -> WorkshopManifest {
    WorkshopManifest {
        schema_version: WORKSHOP_MANIFEST_SCHEMA_VERSION.to_owned(),
        package_id: WORKSHOP_PACKAGE_ID.to_owned(),
        package_version: "0.1.0".to_owned(),
        consumer_app_id: APP_ID,
        published_file_id: ITEM_ID,
        game_version: GAME_VERSION.to_owned(),
        platform: PLATFORM.to_owned(),
        loader_contract: WORKSHOP_LOADER_CONTRACT.to_owned(),
        content_kind: WorkshopContentKind::FirstPartyExecutable,
        entrypoint: "AIAscensionSTS2Poc.json".to_owned(),
        files: vec![
            WorkshopFile {
                path: "AIAscensionSTS2Poc.dll".to_owned(),
                role: WorkshopFileRole::ManagedAssembly,
                size_bytes: 12,
                sha256: digest('a'),
            },
            WorkshopFile {
                path: "AIAscensionSTS2Poc.json".to_owned(),
                role: WorkshopFileRole::LoaderManifest,
                size_bytes: 12,
                sha256: digest('b'),
            },
            WorkshopFile {
                path: "ai_ascension_sts2_poc.dll".to_owned(),
                role: WorkshopFileRole::NativeLibrary,
                size_bytes: 12,
                sha256: digest('c'),
            },
        ],
        content_digest: digest('d'),
        source_revision: "commit-123".to_owned(),
    }
}

fn policy() -> WorkshopPolicy {
    WorkshopPolicy::first_party(
        APP_ID,
        Some(ITEM_ID),
        GAME_VERSION,
        PLATFORM,
        vec![
            AllowedWorkshopFile {
                path: "AIAscensionSTS2Poc.dll".to_owned(),
                role: WorkshopFileRole::ManagedAssembly,
            },
            AllowedWorkshopFile {
                path: "AIAscensionSTS2Poc.json".to_owned(),
                role: WorkshopFileRole::LoaderManifest,
            },
            AllowedWorkshopFile {
                path: "ai_ascension_sts2_poc.dll".to_owned(),
                role: WorkshopFileRole::NativeLibrary,
            },
        ],
    )
}

fn snapshot(manifest: Option<WorkshopManifest>) -> WorkshopItemSnapshot {
    WorkshopItemSnapshot {
        consumer_app_id: APP_ID,
        item_id: ITEM_ID,
        subscribed: true,
        download_app_id: Some(APP_ID),
        state: WorkshopInstallState::Installed,
        install_path: Some("/steam/workshop/content/2111222/9111222".to_owned()),
        manifest: manifest.and_then(|value| serde_json::to_vec(&value).ok()),
    }
}

#[test]
fn valid_manifest_round_trips_and_matches_first_party_policy() -> Result<(), Box<dyn Error>> {
    let value = manifest();
    value.validate()?;
    let encoded = serde_json::to_vec(&value)?;
    let decoded = WorkshopManifest::parse(&encoded)?;
    assert_eq!(decoded, value);
    assert_eq!(decoded.validate_against(&policy()), Ok(()));
    Ok(())
}

#[test]
fn malformed_unknown_and_oversized_manifests_fail_closed() -> Result<(), Box<dyn Error>> {
    assert_eq!(
        WorkshopManifest::parse(b"not-json"),
        Err(WorkshopManifestError::MalformedJson)
    );

    let mut unknown = serde_json::to_value(manifest())?;
    unknown["unexpected"] = json!(true);
    assert_eq!(
        WorkshopManifest::parse(&serde_json::to_vec(&unknown)?),
        Err(WorkshopManifestError::MalformedJson)
    );

    let oversized = vec![b' '; 64 * 1024 + 1];
    assert_eq!(
        WorkshopManifest::parse(&oversized),
        Err(WorkshopManifestError::ManifestTooLarge)
    );
    Ok(())
}

#[test]
fn unsafe_paths_duplicates_and_bad_file_shapes_are_rejected() {
    let mut traversal = manifest();
    traversal.files[0].path = "../AIAscensionSTS2Poc.dll".to_owned();
    assert_eq!(
        traversal.validate(),
        Err(WorkshopManifestError::InvalidFilePath)
    );

    let mut absolute = manifest();
    absolute.files[0].path = "/tmp/AIAscensionSTS2Poc.dll".to_owned();
    assert_eq!(
        absolute.validate(),
        Err(WorkshopManifestError::InvalidFilePath)
    );

    let mut duplicate = manifest();
    duplicate.files[1].path = "aIAscensionSTS2Poc.dll".to_owned();
    duplicate.files[1].role = WorkshopFileRole::ManagedAssembly;
    assert_eq!(
        duplicate.validate(),
        Err(WorkshopManifestError::DuplicatePath)
    );

    let mut unsorted = manifest();
    unsorted.files.swap(0, 2);
    assert_eq!(
        unsorted.validate(),
        Err(WorkshopManifestError::FileInventoryNotSorted)
    );

    let mut wrong_extension = manifest();
    wrong_extension.files[0].path = "AIAscensionSTS2Poc.json".to_owned();
    assert_eq!(
        wrong_extension.validate(),
        Err(WorkshopManifestError::InvalidFilePath)
    );

    let mut zero_size = manifest();
    zero_size.files[0].size_bytes = 0;
    assert_eq!(
        zero_size.validate(),
        Err(WorkshopManifestError::InvalidFileSize)
    );

    let mut bad_digest = manifest();
    bad_digest.files[0].sha256 = "ABC".to_owned();
    assert_eq!(
        bad_digest.validate(),
        Err(WorkshopManifestError::InvalidFileDigest)
    );
}

#[test]
fn compatibility_rejects_version_item_and_allowlist_drift() {
    let mut version = manifest();
    version.game_version = "0.107.2".to_owned();
    assert_eq!(
        version.validate_against(&policy()),
        Err(WorkshopCompatibilityError::PackageCompatibility)
    );

    let mut item = manifest();
    item.published_file_id += 1;
    assert_eq!(
        item.validate_against(&policy()),
        Err(WorkshopCompatibilityError::PublishedFileId)
    );

    let mut files = manifest();
    files.files.pop();
    assert_eq!(
        files.validate_against(&policy()),
        Err(WorkshopCompatibilityError::FileAllowlist)
    );
}

#[test]
fn consumer_waits_for_install_readiness() {
    let consumer = WorkshopConsumer::new(policy());
    for (state, expected) in [
        (
            WorkshopInstallState::Missing,
            WorkshopWaitReason::DownloadPending,
        ),
        (
            WorkshopInstallState::DownloadPending,
            WorkshopWaitReason::DownloadPending,
        ),
        (
            WorkshopInstallState::Downloading,
            WorkshopWaitReason::Downloading,
        ),
        (
            WorkshopInstallState::NeedsUpdate,
            WorkshopWaitReason::NeedsUpdate,
        ),
    ] {
        let mut current = snapshot(None);
        current.state = state;
        assert_eq!(consumer.inspect(current), WorkshopReadiness::Wait(expected));
    }

    let mut unsubscribed = snapshot(Some(manifest()));
    unsubscribed.subscribed = false;
    assert_eq!(
        consumer.inspect(unsubscribed),
        WorkshopReadiness::Wait(WorkshopWaitReason::NotSubscribed)
    );
}

#[test]
fn consumer_rejects_wrong_identity_missing_content_and_unsafe_paths() {
    let consumer = WorkshopConsumer::new(policy());

    let mut wrong_app = snapshot(Some(manifest()));
    wrong_app.consumer_app_id += 1;
    assert_eq!(
        consumer.inspect(wrong_app),
        WorkshopReadiness::Reject(WorkshopReject::ConsumerAppId)
    );

    let mut wrong_item = snapshot(Some(manifest()));
    wrong_item.item_id += 1;
    assert_eq!(
        consumer.inspect(wrong_item),
        WorkshopReadiness::Reject(WorkshopReject::ItemId)
    );

    let mut wrong_download_app = snapshot(Some(manifest()));
    wrong_download_app.download_app_id = Some(APP_ID + 1);
    assert_eq!(
        consumer.inspect(wrong_download_app),
        WorkshopReadiness::Reject(WorkshopReject::DownloadAppId)
    );

    let mut missing_path = snapshot(Some(manifest()));
    missing_path.install_path = None;
    assert_eq!(
        consumer.inspect(missing_path),
        WorkshopReadiness::Reject(WorkshopReject::MissingInstallPath)
    );

    let mut unsafe_path = snapshot(Some(manifest()));
    unsafe_path.install_path = Some("relative/../../workshop".to_owned());
    assert_eq!(
        consumer.inspect(unsafe_path),
        WorkshopReadiness::Reject(WorkshopReject::UnsafeInstallPath)
    );

    let mut missing_manifest = snapshot(None);
    assert_eq!(
        consumer.inspect(missing_manifest.clone()),
        WorkshopReadiness::Reject(WorkshopReject::MissingManifest)
    );
    missing_manifest.state = WorkshopInstallState::Corrupt;
    assert_eq!(
        consumer.inspect(missing_manifest),
        WorkshopReadiness::Reject(WorkshopReject::Corrupt)
    );
}

#[test]
fn consumer_returns_owned_ready_package_only_after_full_validation() {
    let consumer = WorkshopConsumer::new(policy());
    let result = consumer.inspect(snapshot(Some(manifest())));
    assert!(matches!(result, WorkshopReadiness::Ready(_)));
    if let WorkshopReadiness::Ready(package) = result {
        let WorkshopPackage {
            item_id,
            install_path,
            manifest: ready_manifest,
        } = *package;
        assert_eq!(item_id, ITEM_ID);
        assert_eq!(install_path, "/steam/workshop/content/2111222/9111222");
        assert_eq!(ready_manifest, manifest());
    }
}

#[test]
fn consumer_rejects_invalid_manifest_and_incompatible_package() -> Result<(), Box<dyn Error>> {
    let consumer = WorkshopConsumer::new(policy());
    let mut malformed = snapshot(None);
    malformed.manifest = Some(b"{}".to_vec());
    assert!(matches!(
        consumer.inspect(malformed),
        WorkshopReadiness::Reject(WorkshopReject::Manifest(_))
    ));

    let mut incompatible = manifest();
    incompatible.platform = "linux-x86_64".to_owned();
    let result = consumer.inspect(snapshot(Some(incompatible)));
    assert_eq!(
        result,
        WorkshopReadiness::Reject(WorkshopReject::Incompatible(
            WorkshopCompatibilityError::PackageCompatibility
        ))
    );
    Ok(())
}
