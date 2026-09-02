# ADR 0011: First-party Steam Workshop package boundary

## Status

Accepted for the first-party package and validation slice. Steam publication, subscription,
download callbacks, and host discovery remain unverified until an authorized Steam/STS2 runtime
test is performed.

## Context

The managed runtime addon is now a real game-facing package with a paired managed assembly and
native companion. Steam Workshop represents an item as a folder of files and downloads subscribed
items through the Steam client. The target has no Steamworks SDK, App ID, published file ID, or
safe reason to load arbitrary third-party executable content.

The official implementation flow is documented at:
<https://partner.steamgames.com/doc/features/workshop/implementation>.

## Decision

sts2-game-mod owns the Workshop package contract, package staging, and runtime compatibility gate.
The first-party item is an executable package because it distributes the existing managed/native
mod. Runtime acceptance is restricted to an explicitly configured first-party App ID and published
file ID, an exact package identity, the exact supported game/platform/loader contract, and the
allowlisted files:

- AIAscensionSTS2Poc.dll;
- AIAscensionSTS2Poc.json; and
- ai_ascension_sts2_poc.dll.

The item also contains sts2-workshop-manifest.json and SHA256SUMS. These metadata files are
required package material but are not executable payload. The manifest is
sts2-workshop-manifest-v1; it records package and compatibility identities, sorted file roles,
sizes, SHA-256 digests, a deterministic content digest, and source revision.

The Rust sts2-game-mod Workshop module owns pure manifest shape validation and Steam install-state
decisions. The managed loader owns actual directory inspection, reparse-point rejection, file
hashing, content-digest verification, and the final handoff gate. Steam callback translation remains
a future adapter seam; no Steam ABI is fabricated while the SDK is absent.

The target-local tools/workshop/package-item.sh accepts an already-built payload, creates the
deterministic manifest/checksum inventory, and emits a Steam Workshop VDF beside the content
directory. A published file ID of 0 is allowed only for item creation. A release candidate must
be rebuilt with the assigned ID and must pass the runtime trust policy.

## Security invariants

- Steam title, author, tags, subscription state, and local folder location are not trust evidence.
- Missing, pending, downloading, updating, corrupt, incompatible, partially installed, malformed,
  unexpected, or changed-during-validation content fails closed.
- Relative, absolute, traversal, case-collision, symlink, and reparse-point payload paths are
  rejected.
- Pull-request CI never uploads to Steam and receives no Steam credentials.
- Host assemblies, saves, profiles, credentials, Steamworks binaries, and generated release output
  remain outside the repository and Workshop staging input.

## Evidence boundary

Rust and managed fixture probes prove manifest, package, path, compatibility, digest, and failure
behavior with synthetic content. They do not prove Steam App Admin configuration, ISteamUGC
initialization, callback delivery, subscription/download behavior, game discovery, or compatibility
with an exact host. Those claims require a separately authorized disposable runtime test.
