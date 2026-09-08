# ADR 0015: First-party Steam Workshop package boundary

## Status

Accepted for the first-party package, guarded create helper, and validation slice. Steam
publication, subscription, download callbacks, and host discovery remain unverified until an
authorized Steam/STS2 runtime test is performed.

## Context

The managed runtime addon is now a real game-facing package with a paired managed assembly and
native companion. Steam Workshop represents an item as a folder of files and downloads subscribed
items through the Steam client. The target does not retain an embedded Steamworks SDK, App ID, or
published file ID, and has no safe reason to load arbitrary third-party executable content.

The official implementation flow is documented at:
<https://partner.steamgames.com/doc/features/workshop/implementation>.

## Decision

sts2-game-mod owns the Workshop package contract, package staging, and runtime compatibility gate.
The first-party item is an executable package because it distributes the existing managed/native
mod. Runtime acceptance is restricted to an explicitly configured first-party App ID and published
file ID, an exact package identity, the exact supported game/platform/loader contract, and the
platform-specific allowlisted files:

- AIAscensionSTS2GameMod.dll;
- AIAscensionSTS2GameMod.json; and
- AIAscensionSTS2GameModNative.dll on Windows x86-64; or
- libAIAscensionSTS2GameModNative.so on Linux x86-64.

The operator's expected platform selects exactly one native library allowlist. Unknown platforms,
the other platform's library, and a manifest whose platform differs from that policy are rejected.
This platform validation is component-tested with synthetic packages; native Linux Workshop
discovery, loading, update, and rollback require separate runtime evidence.

The item also contains sts2-workshop-manifest.json and SHA256SUMS. These metadata files are
required package material but are not executable payload. The manifest is
sts2-workshop-manifest-v1; it records package and compatibility identities, sorted file roles,
sizes, SHA-256 digests, a deterministic content digest, and source revision.

The Rust sts2-game-mod Workshop module owns pure manifest shape validation and Steam install-state
decisions. The managed loader owns actual directory inspection, reparse-point rejection, file and
SHA256SUMS inventory hashing, content-digest verification, and the final handoff gate. The versioned
Linux x86-64 `tools/workshop/lifecycle/ugc-create-item` helper owns a guarded empty-item
`ISteamUGC::CreateItem` operation using an operator-supplied native Steam API library. It verifies
the library digest, account/environment preconditions, and the pinned Linux callback ABI before
calling the flat API exports, and records unknown outcomes when transport or callback settlement is
uncertain. The helper does not embed the Steamworks SDK or credentials; upload and update remain
guarded SteamCMD VDF operations.

The target-local tools/workshop/package-item.sh accepts an already-built payload, creates the
deterministic manifest/checksum inventory, and emits a Steam Workshop VDF beside the content
directory. A published file ID of 0 is allowed only for item creation. A release candidate must
be rebuilt with the assigned ID and must pass the runtime trust policy.

## Security invariants

- Steam title, author, tags, subscription state, and local folder location are not trust evidence.
- Missing, pending, downloading, updating, corrupt, incompatible, partially installed, malformed,
  unexpected, or digest-mismatching content fails closed at validation time.
- Relative, absolute, traversal, case-collision, symlink, and reparse-point payload paths are
  rejected.
- Pull-request CI never uploads to Steam and receives no Steam credentials.
- Host assemblies, saves, profiles, credentials, Steamworks binaries, and generated release output
  remain outside the repository and Workshop staging input.

The package must remain quiescent in an owner-controlled directory through native loading. Validation
does not hold all payload handles through that later load, so concurrent replacement is not prevented.
Manifest IDs and hashes establish compatibility and internal byte consistency, not publisher
authentication. The managed assembly is already executing when this gate checks its package;
Steam installation trust and managed assembly admission are separate, not implemented by this gate.

## Evidence boundary

Rust and managed fixture probes prove manifest, package, path, compatibility, digest, and failure
behavior with synthetic content. They do not prove Steam App Admin configuration, ISteamUGC
initialization, callback delivery, subscription/download behavior, game discovery, or compatibility
with an exact host. Those claims require a separately authorized disposable runtime test.
