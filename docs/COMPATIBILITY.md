# Compatibility policy and matrix

## Separate dimensions

Compatibility is tracked independently for the owner-local HTTP contract, managed loader metadata,
native ABI, STS2 host version, operating system and architecture, Rust/.NET runtime, configuration,
and package contents. A build result never silently broadens another dimension.

The initial game baseline is recorded in
[decision 0002](decisions/0002-initial-game-compatibility-baseline.md). It is a source-derived
planning target, not runtime proof from this target.

## Evidence levels

1. Build-only: source compiles with the declared toolchain and any authorized host reference.
2. Load smoke: a packaged managed entry point is discovered and starts in the exact game.
3. Focused runtime: a disposable profile exercises the affected host path.
4. Full conformance: all required host-dependent and owner-local suites pass.

Every claim records exact game version or commit, host assembly hash when used, operating system,
architecture, managed/runtime versions, source revision, artifact hash, date, inputs, outputs,
cleanup, and evidence level. Absence of evidence remains unverified.

## Current matrix

| Managed/native target | Game host | Platform | Evidence | Result |
| --- | --- | --- | --- | --- |
| Runtime addon `AIAscensionSTS2GameMod` (renamed from `AIAscensionSTS2Poc`; evidence predates rename) | STS2 v0.107.1, commit `59260271` | Windows x86-64 | Load smoke | Confirmed for the recorded pre-rename package; [dated runtime evidence](evidence/runtime-addon-load-smoke-20260902.md) |
| Runtime-v1 listener and host probe | STS2 v0.107.1, commit `59260271` | Windows x86-64 | Focused runtime | Confirmed; [dated host evidence](evidence/runtime-v1-host-live-20260902.md) |
| Runtime-v2 fake boundary | No host; `sts2-protocol` commit `8d4b2f574cf860a71f2a5e4ce3308ac069cb1527` | Offline Rust toolchain | Deterministic source/build/test | Confirmed for the in-memory fake seam only; live host mutation and settlement unverified |
| Gameplay host behavior | STS2 v0.107.1, commit `59260271` | Windows x86-64 | Not executed | Unverified; no gameplay mutation is implemented |

The project planning baseline names a host assembly identity, but this repository does not retain
that proprietary file. The recorded load-smoke uses the operator's installed host assembly without
storing or distributing it. No support claim is made for beta builds, earlier versions, Linux,
macOS, or another architecture until an exact matrix row and evidence exist.

Runtime-v2 is pinned locally to schema digest
`f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2` with provenance generator
`hand-authored`. No concrete host gameplay API exists in this repository. The copied artifact,
Rust contract, and fake lifecycle tests provide source/build/test evidence only; live host mutation
and live host settlement are unverified.

## Contract compatibility

The checked-in `protocol-artifact/poc-v1/` copy is consumed by exact protocol version, schema
digest, and provenance. It is an offline release-like input, not a package or runtime compatibility
claim. The game-mod POC mapping remains source/test-confirmed only. The separate runtime-v1 probe
has confirmed its HTTP contract, main-thread callback, and host-visible witness in the exact recorded
host; gameplay compatibility remains unverified.

HTTP and ABI changes are classified as internal, additive-compatible, safety correction,
deprecated-compatible, or breaking. A route, field, status, error, ordering, ABI symbol, calling
convention, or host callback change needs a requirement, fixture, migration note, and version
decision. Game-host adaptation and HTTP compatibility are versioned independently.

The host boundary must preserve main-thread affinity, explicit acceptance versus completion, bounded
queue behavior, and sanitized errors. A client disconnect or timeout does not revoke host work
already accepted.

## Promotion and deprecation

Do not promote build-only evidence to load or runtime support. A deprecation names a replacement,
warning, first deprecated release, earliest removal release, and compatibility tests. A breaking
change needs explicit approval and coordinated updates to the adapter, schema, package, and
consumers.

## Runtime profile row

| Managed/native target | Runtime profile | Evidence | Result |
| --- | --- | --- | --- |
| `AIAscensionSTS2GameMod` plus native runtime listener (runtime evidence predates rename) | `sts2-protocol/runtime-v1` | Rust/managed gates plus authorized disposable-host request/action trace | Focused runtime confirmed for the recorded pre-rename package on STS2 v0.107.1 Windows x86-64; gameplay and broader compatibility unverified |

The profile's `show_runtime_probe` action proves only a host-visible status-overlay witness when
reproduced in an authorized disposable host. It is not a support claim for gameplay mutation,
another host version, another platform, or a valued profile.

## Workshop package profile

The first-party Workshop package has its own compatibility dimensions: consumer App ID, published
file ID, package version, game version, platform, loader contract, file-role allowlist, payload
sizes, per-file digests, and content digest. The Rust contract and managed validator have
source/build and synthetic-fixture evidence for these dimensions.

The supported package is executable because it distributes the current managed/native mod, but the
loader accepts only the exact first-party allowlist under an explicitly configured App ID and item
ID. A Workshop title, author, tag, subscription, or folder path is not a trust or compatibility
proof. The current target has no committed App ID or item ID; operators supply them while staging
and must rebuild a release candidate after Steam assigns a new item ID.

No row is promoted to Steam runtime support until an authorized disposable test records Steam API
initialization, item creation/update or subscription, install callback/poll behavior,
GetItemInstallInfo usage, game discovery, load smoke, and cleanup for an exact host/platform
matrix.
