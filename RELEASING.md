# Release policy and procedure

This target has no release artifact yet. Preparing a build, publishing a package, installing a
mod, and verifying a release are separate states. None is authorized by ordinary foundation work.

## Authority and versioning

Only an explicitly authorized maintainer may publish or deploy. Agents may prepare and inspect a
candidate, but must not create or move tags, upload artifacts, install a mod, mutate a profile, or
deploy without explicit authorization.

Repository, HTTP, ABI, managed host, game compatibility, and package versions are separate facts.
Do not infer one from another. A host ABI adaptation can require a compatibility release even when
the local HTTP contract is unchanged.

## Release readiness

A release may claim only the highest executed evidence level in
[docs/COMPATIBILITY.md](docs/COMPATIBILITY.md). Before publication, the exact commit, review,
policy, formatting, lint, tests, contract fixtures, security checks, packaging contents, and
host/runtime evidence must be recorded.

No release may contain:

- proprietary game assemblies or assets;
- personal saves, profiles, credentials, or machine-specific paths;
- source checkout metadata, target directories, debug output, or unrelated files; or
- an unreviewed dependency or fixture with unknown licensing.

The managed loader and each native artifact must be paired for the exact supported
operating-system and architecture. Source archives include the manifest, applicable notices,
license, and user documentation; the Workshop runtime item is the separately defined five-file
package and keeps host assemblies outside both distributions.

## Prepare and verify

1. Record the intended source revision and release classification.
2. Update the changelog, compatibility matrix, migration notes, and all authoritative versions.
3. Run the policy, format, lint, test, conformance, security, and package checks from
   [docs/TESTING.md](docs/TESTING.md).
4. Build from the exact approved source and record checksums.
5. Inspect the unpacked bytes and run install/start smoke tests only in an authorized disposable
   environment.
6. Obtain maintainer publication approval and preserve the artifact-to-commit mapping.

The host-loader smoke test must use an operator-supplied exact host installation and disposable
data. CI must not download or redistribute proprietary host files.

## Reproducible source bundles

The checked-in source bundle tool is the portable Windows/Linux release preparation path:

```text
SOURCE_DATE_EPOCH=0 bash tools/release/test-source-bundle.sh
SOURCE_DATE_EPOCH=0 bash tools/release/build-source-bundle.sh 0.4.0 windows-x86_64 /tmp/release/windows HEAD
SOURCE_DATE_EPOCH=0 bash tools/release/build-source-bundle.sh 0.4.0 linux-x86_64 /tmp/release/linux HEAD
```

It archives tracked files from the resolved commit, rejects tracked host binaries, saves, profiles,
and build output, and writes `RELEASE-MANIFEST.json`, `SHA256SUMS`, and an external archive checksum.
The two platform labels make the compatibility target explicit; they are source bundles and do not
include the managed loader, native companion, or proprietary host inputs.

Before distribution, extract each archive in a clean directory and run the embedded checksum check.
For an update, retain the prior verified archive and manifest, stop the target disposable game
profile, install the new staged five-file payload, and verify its manifest and hashes before launch.
`tools/release/install-runtime-addon.sh` performs this file-scoped install for either supported
platform, preserves unrelated files, records prior bytes in a caller-owned backup directory, and
provides an explicit rollback operation. If the new payload fails verification or load smoke, stop
it, restore the prior verified payload, rerun its hash check, and record the source commit and both
package checksums. This is an operator procedure; no host installation or rollback is performed by
CI.

## Workshop platform boundary

The Workshop contract supports separate Windows x86-64 and Linux x86-64 packages.
`tools/workshop/package-platform-item.sh` requires the platform explicitly and stages the managed
assembly, loader manifest, platform-native library, `sts2-workshop-manifest.json`, and `SHA256SUMS`.
The Windows native filename is `AIAscensionSTS2GameModNative.dll`; the Linux native filename is
`libAIAscensionSTS2GameModNative.so`. The managed validator applies the exact matching allowlist and
platform before the native loader runs. `tools/workshop/package-item.sh` remains a Windows wrapper.

Do not overwrite one Workshop item with a native payload for another platform. Use a distinct
first-party published-file ID for each platform until a shared multi-platform item has a separately
reviewed contract and exact host/platform evidence. A package is staged and installable only after
the payload, manifest, and checksum gates pass; publication and host runtime evidence remain
separate gates.

## Workshop publication

Workshop staging is a release-preparation action, not an ordinary CI action:

1. Build the managed/native addon from the exact approved source and stage its three runtime payload files.
2. Run tools/workshop/package-platform-item.sh with the exact platform, consumer App ID, assigned published file ID, exact
   game/package versions, source revision, and an operator-owned preview image.
3. Inspect the content directory, manifest, SHA256SUMS, VDF, and checksums. The VDF must remain
   beside the content directory and must not be uploaded as item content.
4. For a new item, use published file ID 0 only to create the item. Record the assigned ID, rebuild
   the package with that ID, and rerun all gates before treating the package as a release candidate.
5. An authorized maintainer may use the generated VDF with SteamCMD for testing/publication or
   later use an approved ISteamUGC publisher. Credentials remain outside the repository.
6. Verify the published item, installed bytes, exact manifest policy, game discovery, load smoke,
   and cleanup separately. Record the evidence level and exact Steam/STS2 versions.

The current repository has no Steamworks SDK or committed App ID/item ID. Package staging, managed
platform validation, and fixture install/update/rollback are implemented; Steam configuration,
upload, subscription, callback, and host-runtime evidence remain unverified. The public consumer app is Steam app `2868840` (Slay the
Spire 2); the intended first-party published-file ID and publisher entitlement must be supplied by
the authorized owner before any create/update or subscription test. Synthetic IDs in fixture tests
are not publication destinations.

## Failure and post-release checks

After publication, verify checksums, package allowlists, documented startup, the HTTP index, and
the claimed host matrix from freshly downloaded bytes. Record this separately from publication.

Never rewrite an immutable tag or silently replace an artifact. Withdraw or mark a defective
release where supported, preserve diagnostic evidence without private data, and issue a corrective
version through the same gates.
