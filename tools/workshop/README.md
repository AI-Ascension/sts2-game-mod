# Steam Workshop package tooling

This directory owns the deterministic, operator-invoked staging path for the first-party
`sts2-game-mod` Workshop item. It does not contain Steam credentials, an App ID, a Workshop item ID,
the Steamworks SDK, the proprietary STS2 host assembly, or generated release output.

## Package boundary

`package-item.sh` accepts an already-built Windows mod payload and an operator-supplied Steam consumer App ID,
published file ID, game version, package version, source revision, and preview image. It accepts only
the three current first-party runtime files:

- `AIAscensionSTS2GameMod.dll` — managed loader assembly;
- `AIAscensionSTS2GameMod.json` — host loader manifest; and
- `AIAscensionSTS2GameModNative.dll` — Windows native companion.

It rejects symlinks, directories, unexpected files, missing/empty payload files, unsafe metadata,
and a pre-existing output directory. The output contains the allowlisted payload, a
`sts2-workshop-manifest.json` with per-file SHA-256 values and a deterministic content digest, and
`SHA256SUMS`. The Steam upload VDF is written beside the content directory so it cannot accidentally
become Workshop payload.

App/item IDs must fit unsigned 32/64-bit decimal fields without leading zeros; metadata tokens
and payload sizes obey the consumer bounds. Invalid input, output nested inside the payload,
and an existing upload VDF are rejected before creating the output directory. Source payloads
must remain quiescent during staging; this tool is not a transactional installer.

The manifest uses the owner-local `sts2-workshop-manifest-v1` contract and records
`sts2-managed-loader-v1`. A `published-file-id` of `0` is valid only for creating a new item. After
Steam assigns an ID, rebuild the package with that exact ID before treating it as a release
candidate. The runtime consumer separately applies its exact App ID, item ID, game version, platform,
loader-contract, and file-role policy.

## Platform boundary

`package-platform-item.sh` requires an explicit `windows-x86_64` or `linux-x86_64` platform and emits
the exact native filename for that target. Windows packages contain
`AIAscensionSTS2GameModNative.dll`; Linux packages contain
`libAIAscensionSTS2GameModNative.so`. Both packages contain the managed assembly, loader manifest,
and the same manifest/checksum contract. `package-item.sh` remains a Windows compatibility wrapper.
The managed loader chooses the same native filename from its runtime OS and validates the manifest
allowlist against the requested `STS2_WORKSHOP_PLATFORM` value before loading the library.

Keep native payloads for different platforms under separate first-party published-file IDs. Never
replace a Windows item in place with incompatible native bytes. A shared multi-platform item needs
a reviewed manifest/loader contract and exact per-platform runtime evidence before its allowlist can
admit any additional native file.

Build a native payload first with `package-runtime-addon.sh` (Windows is the default for existing
callers; pass `--platform linux-x86_64` for the Linux target), then pass the resulting directory to
`package-platform-item.sh`. The tool is a deterministic staging step, not an installer: it refuses
symlinks, unexpected files, missing/empty payload files, unsafe metadata, and pre-existing outputs.

## Upload boundary

Valve documents the `ISteamUGC` create/update flow and the separate `steamcmd.exe` VDF flow at
<https://partner.steamgames.com/doc/features/workshop/implementation>. `steamcmd` is for operator
testing and staging only; credentials must be entered outside this repository. No pull-request
workflow uploads content, and no workflow receives Steam credentials.

The VDF can be passed to `steamcmd workshop_build_item` by an authorized maintainer. The versioned
Linux x86-64 `lifecycle/ugc-create-item` helper also provides a guarded `ISteamUGC::CreateItem`
path for creating an empty item when an operator supplies the pinned public SDK ABI proof, exact
package, native library, and account environment. Upload and update continue through the guarded
SteamCMD VDF workflow; the helper does not embed the Steamworks SDK, credentials, proprietary STS2
files, or generated native binaries.

## Test

Run the fixture-only self-test from this directory:

```text
bash tools/workshop/test-package-item.sh
```

The test covers both platform allowlists with synthetic files and a synthetic preview. It does not
contact Steam, use a game profile, or build/load executable code.
