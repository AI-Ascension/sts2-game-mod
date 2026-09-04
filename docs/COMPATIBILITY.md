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
| Repeat-seed practice replay | STS2 v0.107.1, commit `59260271` | Windows x86-64 | Managed build against exact host; gameplay not executed | Build confirmed; settings UI, cleanup, restart, history suppression, and protected-mode behavior unverified; see [repeat-seed](repeat-seed.md) |
| Runtime-v2 host adapter candidate | STS2 v0.107.1, commit `59260271` | Windows x86-64 | Managed/native build and package | Confirmed at source/build level; [build evidence](evidence/runtime-v2-host-build-20260902.md); live execution and settlement unverified |
| Gameplay host behavior | STS2 v0.107.1, commit `59260271` | Windows x86-64 | Not executed | Unverified; the candidate has not been exercised in a live host |

The project planning baseline names a host assembly identity, but this repository does not retain
that proprietary file. The recorded load-smoke uses the operator's installed host assembly without
storing or distributing it. No support claim is made for beta builds, earlier versions, Linux,
macOS, or another architecture until an exact matrix row and evidence exist.

The repeat-seed row is intentionally narrower than general gameplay support. The implementation
only targets an active single-player Custom run and restarts it from the captured seed beginning.
It does not support later-floor checkpoints, standard/daily/multiplayer replay, or another host
version without a new compatibility test.

## Built-in runtime listener settings compatibility

The `AIAscensionSTS2Poc` addon owns its Runtime connection settings surface. ModConfig or another
settings-framework mod is not required, bundled, or referenced. The native Installed Mods checkbox
remains the sole enablement control for the addon itself.

The built-in settings contract is:

| Setting | Default | Compatibility behavior |
| --- | --- | --- |
| Runtime API | Enabled | Applies immediately; disabling stops the listener unless the explicit `STS2_RUNTIME_SESSION=1` launch override is active |
| Bind address | `127.0.0.1` | The UI offers loopback, all interfaces, the detected hostname, and detected local IPv4 addresses |
| Network port | `15526` | The UI accepts `1024` through `65535` and applies it immediately |

The settings tab also reports authentication and listener status without exposing the bearer token.
Apply persists the selected values and restarts only the bounded listener; Reset restores the defaults
and applies them immediately.
`STS2_RUNTIME_PORT` and `STS2_RUNTIME_BIND_ADDRESS` remain explicit environment overrides for
automation. The address and port controls are source/build and load-smoke confirmed in the exact
recorded host; clicking every control, hostname resolution, firewall behavior, and every supported
bind address remain unverified and do not broaden the host matrix.

The profile action rejects active runs and unfinished/failed run saves before switching the active
profile or changing progress. These guards have production-linked synthetic coverage only; an
exact-host build and authorized disposable-profile test remain required. Profile changes are
persistent, do not unlock achievements, and do not reduce ascension values already above 10.

Runtime-v2 is pinned locally to schema digest
`f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2` with provenance generator
`hand-authored`. The host-adapter candidate uses the common verified symbols documented in the
build evidence, including `CombatManager.IsInProgress`, `IsEnemyTurnStarted`,
`PlayerActionsDisabled`, `DebugOnlyGetState`, and `PlayerCmd.EndTurn`. The copied artifact, Rust
contract, fake lifecycle tests, and managed build provide source/build/test evidence only; live host
mutation and live host settlement are unverified.

## Contract compatibility

The target-local ABI descriptor validator rejects nonzero reserved bytes, as required by ABI
version 1. This is a source-tested validation correction; the descriptor layout, version, and
zero-filled descriptors are unchanged. It does not establish host compatibility.

The managed Runtime-v1 request validator enforces the copied schema's closed required shape,
null sentinels, identity bounds and maximum safe integer. It additionally rejects duplicate JSON
properties and requires the body epoch to match the transport context. Numeric values use the
typed unsigned-integer representation (fraction/exponent lexical forms are not accepted).
Malformed inputs fail before host effects. A session refuses additional probe actions once
`action_count` reaches 1024 rather than emitting observations outside the immutable schema.
These are source-tested contract enforcement corrections, not an artifact or native ABI revision.
A host effect invoked without its subsequently visible witness returns HTTP 503 owner-local
`runtime_probe_outcome_unknown`; it must not be interpreted as proof that the effect did not occur.

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

Runtime-v1 queue timeout safety correction preserves the artifact and ABI. Pending work is removed
before dispatch; already executing work is not revoked. HTTP 504 now carries an owner-local
`main_thread_timeout_before_dispatch` or `main_thread_outcome_unknown` transport error, rather than
a canonical rejected action response which could falsely imply no effect. Saturation returns HTTP
503 `runtime_queue_full`. These source-only tested failures do not extend the live-host matrix.

Do not promote build-only evidence to load or runtime support. A deprecation names a replacement,
warning, first deprecated release, earliest removal release, and compatibility tests. A breaking
change needs explicit approval and coordinated updates to the adapter, schema, package, and
consumers.

The native listener's absolute 10-second socket-I/O deadline is a safety correction, not a new
wire or ABI version. Clients must finish framing and consume responses within that budget;
timeouts may close the connection without a response. The callback lifetime and synchronous
calling convention are unchanged. Linux loopback regressions establish the new I/O behavior;
Windows host reconfiguration and unload remain separately unverified.

## Runtime profile row

| Managed/native target | Runtime profile | Evidence | Result |
| --- | --- | --- | --- |
| `AIAscensionSTS2GameMod` plus native runtime listener (runtime evidence predates rename) | `sts2-protocol/runtime-v1` | Rust/managed gates plus authorized disposable-host request/action trace | Focused runtime confirmed for the recorded pre-rename package on STS2 v0.107.1 Windows x86-64; gameplay and broader compatibility unverified |
| `AIAscensionSTS2GameMod` plus native runtime listener | `sts2-protocol/runtime-v2` | Rust/managed gates plus controlled disposable-host `end_turn` trace | Build/package candidate confirmed; host mutation, settlement, and cross-target runtime remain unverified |

The profile's `show_runtime_probe` action proves only a host-visible status-overlay witness when
reproduced in an authorized disposable host. It is not a support claim for gameplay mutation,
another host version, another platform, or a valued profile.

The separate `runtime-v3-gameplay` bridge is source-derived and uses the neutral protocol digest
`b37c80f583aeaf4f81ede2083bcfb4129196baf5eb092470e8738173c4b7226c`. Its fair-play projection,
typed catalog checks, host-thread adapter, separate co-op helpers, and postcondition verifier are
covered by source/build tests. Exact target assembly compatibility, host legality, full-run effect
settlement, and multiplayer behavior remain `unverified`.

The internal Runtime-v3 host-source interface receives scoped operation identities
and must supply independent completion evidence. The internal completion correction
leaves message shapes unchanged. Separately, the unmerged protocol revision tightens
schema shape validation, explicit nullable fields, and closed tagged variants.
Consumers must update together to the new digest above; old-digest requests fail closed.
Host implementations must implement the completion port;
an unavailable witness preserves an unknown outcome. Managed handler tests use
synthetic completion events and do not promote the licensed-host compatibility row.
The co-op helpers are not connected to the managed gameplay request path; their
source-only validation does not establish multiplayer mutation fencing.

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

## Review correction (2026-09-04)

The source review replaced the candidate's state-delta settlement inference. Neither a later turn
nor changed energy/pile counts proves completion of a particular queued operation. The current
adapter returns `unknown` after enqueue (including enqueue exceptions), retains its operation and
blocks further v2 mutations until independent operation-bound completion is available. It does
not emit a settlement witness from these host adapters. No such host completion binding has yet
been established; this is an integration blocker, not a successful gameplay result.

Runtime-v2 retains one identity fence and one outstanding-mutation exclusion. Exact semantic
retries ignore transport correlation and JSON formatting; run/combat/player replacement
invalidates generation. This bounded observation is not a complete game-state revision
or a game-rule parity claim.
