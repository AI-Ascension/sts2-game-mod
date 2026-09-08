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

It resolves the requested commit, applies the checked-in
`tools/release/source-distribution-policy-v1.json` exact tracked-path allowlist, and fails closed if
the tree has an unreviewed path. The policy removes diagnostic and gameplay fixture sources from the
production source archive while retaining the production build closure. It also rejects tracked host
binaries, saves, profiles, and build output. `RELEASE-MANIFEST.json` records the original full Git
tree, policy identity and hash, excluded paths, included path count and content digest; `SHA256SUMS`
and an external archive checksum cover the resulting bytes. The two platform labels make the
compatibility target explicit; these source bundles are separate from the built managed loader,
native companion, and proprietary host inputs.

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

### Reproducible Windows native builds

The `x86_64-pc-windows-gnu` target configuration disables PE linker timestamps.
Without `--no-insert-timestamp`, identical native source produces different DLL checksums
at different build times. Preserve this target flag when building a release; environment
`RUSTFLAGS` or `CARGO_ENCODED_RUSTFLAGS` overrides must not remove it. Use the pinned
toolchain and locked dependencies, build the same commit into two independent Cargo target
directories, and compare the resulting DLL bytes. Record this separately from package
checksum validation and native loading evidence.

The canonical native release helper is
`experiments/managed-rust-interop/build-native-release.sh`. It accepts
`windows-x86_64` or `linux-x86_64`, resolves Cargo's effective target directory, and passes
remap flags for the checkout and `CARGO_HOME` through `CARGO_ENCODED_RUSTFLAGS`. The Windows
path explicitly carries `-C link-arg=-Wl,--no-insert-timestamp` because encoded environment
flags replace target-table flags. The helper rejects non-empty ambient `RUSTFLAGS` and
`CARGO_ENCODED_RUSTFLAGS` rather than silently dropping an unreviewed override, and fails if
either producer path remains in the native artifact. `package-runtime-addon.sh` invokes the
helper for either platform package; the helper changes to the pinned checkout root internally.

Canonical native checks are:

~~~text
experiments/managed-rust-interop/build-native-release.sh windows-x86_64
experiments/managed-rust-interop/build-native-release.sh linux-x86_64
~~~

The helper output path is the artifact to copy into a package. It does not publish, upload,
install, or launch anything.

Release managed addons map their source directory to a stable logical path and do not
embed a portable-PDB location. This keeps the operator checkout path out of the DLL and
allows a second build in a different checkout to reproduce it. Debug builds retain the
normal debugging settings. Compare managed DLLs using the same SDK and exact host reference
assemblies; the Windows and Linux host references are distinct compatibility inputs.

When building from a source archive without `.git` metadata, pass the exact source revision
explicitly so the managed assembly carries the same informational version as a Git checkout:

~~~text
dotnet build experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj \
  --configuration Release -p:STS2GameDataDir=/path/to/host-data \
  -p:SourceRevisionId=<exact-commit>
~~~

Use the same pinned .NET SDK and exact `sts2.dll` and `GodotSharp.dll` reference assemblies for
the archive and checkout builds before comparing their bytes. An archive build without
`SourceRevisionId` is a different managed artifact even when its source files match.

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

The current repository has no embedded Steamworks SDK or committed App ID/item ID. Package staging,
managed platform validation, fixture install/update/rollback, and the guarded Linux x86-64
empty-item `ISteamUGC::CreateItem` helper are implemented; real Steam configuration, upload,
subscription, callback settlement, and host-runtime evidence remain unverified. The public
consumer app is Steam app `2868840` (Slay the Spire 2); the intended first-party published-file ID
and publisher entitlement must be supplied by the authorized owner before any create/update or
subscription test. Synthetic IDs in fixture tests are not publication destinations.

## Failure and post-release checks

After publication, verify checksums, package allowlists, documented startup, the HTTP index, and
the claimed host matrix from freshly downloaded bytes. Record this separately from publication.

Never rewrite an immutable tag or silently replace an artifact. Withdraw or mark a defective
release where supported, preserve diagnostic evidence without private data, and issue a corrective
version through the same gates.
