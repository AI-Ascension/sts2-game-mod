# Steam Workshop package tooling

This directory owns the deterministic, operator-invoked staging path for the first-party
`sts2-game-mod` Workshop item. It does not contain Steam credentials, an App ID, a Workshop item ID,
the Steamworks SDK, the proprietary STS2 host assembly, or generated release output.

## Package boundary

`package-item.sh` accepts an already-built mod payload and an operator-supplied Steam consumer App ID,
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

## Upload boundary

Valve documents the `ISteamUGC` create/update flow and the separate `steamcmd.exe` VDF flow at
<https://partner.steamgames.com/doc/features/workshop/implementation>. `steamcmd` is for operator
testing and staging only; credentials must be entered outside this repository. No pull-request
workflow uploads content, and no workflow receives Steam credentials.

The VDF can be passed to `steamcmd workshop_build_item` by an authorized maintainer. A future
in-game publisher may use `ISteamUGC::CreateItem`, `StartItemUpdate`, `SetItemContent`,
`SetItemPreview`, and `SubmitItemUpdate`, but that API binding is not fabricated by this target while
the Steamworks SDK is absent.

## Test

Run the fixture-only self-test from this directory:

```text
bash tools/workshop/test-package-item.sh
```

The test uses synthetic files and a synthetic preview only. It does not contact Steam, use a game
profile, or build/load executable code.
