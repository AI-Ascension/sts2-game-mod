# Testing and evidence

For the opt-in exact-host LLM combat, repeat/replay evidence, launch settings and isolation
limits, see [LIVE_COMBAT_DEMO.md](LIVE_COMBAT_DEMO.md). Host-free CI does not replace this
live evidence. Changed-seed replay must stop before dispatch; repeated same-seed choices
must receive fresh host completion witnesses.

Workshop regression probes reject duplicate JSON properties, symlink install roots, oversized
manifests without proportional allocation, and producer metadata/payload bounds before staging.
Symlink checks explicitly report unverified when the runner cannot create a link. These synthetic
checks do not establish Windows junction behavior or safety against concurrent package replacement.

## Purpose

Tests must prove observable boundary invariants without requiring a game whenever possible.
Host-dependent evidence runs only in an authorized disposable environment and records its exact
inputs, outputs, cleanup, and evidence level.

## Foundation commands

Run from the target root:

~~~text
cargo metadata --locked --no-deps --format-version 1
(cd protocol-artifact/poc-v1 && sha256sum -c SHA256SUMS)
(cd protocol-artifact/runtime-v1 && sha256sum -c SHA256SUMS)
cargo test --locked --package sts2-game-mod --test poc
cargo test --locked --offline --package sts2-game-mod --test runtime_v2
(cd protocol-artifact/runtime-v2 && sha256sum -c SHA256SUMS)
cargo run --locked --offline --package repo-policy -- --strict
cargo fmt --all --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
~~~

The workspace now also contains the target-owned host, HTTP-adapter, composition, and copied
`poc-v1` mapping seams. The commands prove source-level structure, queue/ABI/adapter composition,
artifact identity, Runtime-v1 compatibility, and the Runtime-v2 deterministic fake lifecycle. The
managed host-adapter build is a separate compiler/package oracle; these ordinary commands still do
not launch the game or prove gameplay or Runtime-v2 host settlement.

## Runtime-v2 deterministic seam

CI runs the Runtime-v2 `sha256sum -c SHA256SUMS` check from its artifact directory alongside
the POC, Runtime-v1, and Runtime-v3 checksum gates. A checksum failure fails the Rust CI job.

The `runtime_v2_admission` regressions also invoke public action-only APIs directly with state
and reconciliation requests, verifying rejection before queue/receipt insertion. The fake host
rejects generation exhaustion before changing its observation or action count.

The focused `runtime_v2` test covers one admitted and settled `end_turn`, exactly-once fake
application, duplicate replay, conflicting operation identity, outside-combat and enemy-turn
rejection, stale generation and identity fencing, queue and receipt bounds, cancellation timing,
post-write disconnect reconciliation, and pre-dispatch timeout removal. `sha256sum -c` verifies the
copied release-like artifact from repository-relative paths. No Rust test invokes STS2 or any
persistent profile/save/provider path; the managed build resolves concrete host symbols without
executing them.

## Runtime-v2 host-adapter build

The managed candidate is built only with an operator-supplied exact host assembly outside the
repository:

~~~text
dotnet restore experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj \
  -p:STS2GameDataDir="<operator-supplied-host-data>"
dotnet build experiments/managed-rust-interop/game-loader/GameLoaderProbe.csproj --configuration Release \
  -p:STS2GameDataDir="<operator-supplied-host-data>" --no-restore
bash experiments/managed-rust-interop/package-runtime-addon.sh \
  "<operator-supplied-host-data>" /tmp/sts2-runtime-v2-addon
~~~

For the recorded v0.107.1 host, the candidate builds with zero warnings and errors and the native
crate passes six tests, including exact bearer-token matching at the native HTTP boundary. Exact
hashes and verified common host symbols are recorded in
[`runtime-v2-host-build-20260902.md`](evidence/runtime-v2-host-build-20260902.md). This is `L1`
build/package evidence, not a live host result.

## Planned layers

| Layer | Purpose | Environment |
| --- | --- | --- |
| unit | validation, ABI values, error mapping, queue policy | any CI host |
| component | dispatcher, lifecycle, configuration, HTTP, fake host | deterministic local |
| protocol | exact owner-local HTTP shapes and golden fixtures | local/CI |
| integration | real bounded sockets and packaged components | isolated CI/local |
| host | loader, callbacks, main-thread and settlement behavior | exact disposable game |
| release smoke | packaged bytes and install/start behavior | authorized clean host |

Wave 2 claims unit/component coverage for the initialized ports, composition, and fake POC mapping.
The runtime-v1 host report adds focused host and integration evidence for one exact disposable
profile; it is not full conformance or a release-support claim.

## Repeat-seed verification

Run the source-linked controller probe without proprietary assemblies:

~~~text
dotnet run --project experiments/managed-rust-interop/replay-tests/ReplayValidationProbe.csproj --configuration Release
~~~

It links the production controller against synthetic API-shape fakes and covers unknown/protected
mode rejection, duplicate queueing, opt-out before the frame or during saving, profile/run changes,
replacement pending saves, failed saves, one same-seed restart, and sanitized post-cleanup failure.
It does not establish real host API compatibility, Godot continuation scheduling, save atomicity,
or the immutability of captured character/act/modifier model objects.

The managed repeat-seed implementation is covered by the exact-host compile command in the host
evidence section below. Static and build checks establish API shape and fail-closed source paths;
they do not establish that a game UI click starts a replacement run. The feature-specific runtime
cases are:

| Case | Expected invariant | Current evidence |
| --- | --- | --- |
| Setting persistence | `allow_repeating_seeds` defaults to `false`, accepts the existing boolean spellings, and saves with the addon settings values | Source/build; runtime read-back unverified |
| Explicit confirmation | The action warns that current progress is discarded and no history entry is created before accepting the request | Source/build; UI runtime unverified |
| Custom single-player admission | Only an active one-player Custom run is accepted; standard, daily, multiplayer, and missing-state paths reject without cleanup | Source/build; exact-host runtime unverified |
| Same-seed restart | Host-owned cleanup and current-save deletion precede one `NGame.StartNewSingleplayerRun` with the captured seed and `GameMode.Custom` | Source/build; exact-host runtime unverified |
| No history mutation | The reset path does not call the run-ended/history pipeline | Source review; runtime history read-back unverified |
| No later-floor claim | The UI and documentation describe a restart from the seed beginning, not checkpoint restoration | Source/documentation |

The existing generic Runtime-v1/v2 duplicate-operation tests are not repeat-seed evidence. A future
checkpoint or history-browser feature must add a separate owner-local contract and deterministic
fixtures rather than extending this seed-only action implicitly.

## Required future behavior

When implemented, tests must cover bounded request and response handling, status and error
mapping, malformed and unknown input, stable ordering, queue capacity and FIFO behavior, accepted
work settlement, cancellation timing, client disconnect, shutdown, callback failures, host object
ownership, native ABI mismatch, and panic containment.

Use deterministic clocks, schedulers, IDs, fakes, and barriers. Do not use arbitrary sleeps,
blanket retries, real user profiles, real saves, or hidden network discovery.

## Host evidence

A host test records the exact STS2 version or commit, host assembly hash without storing the
assembly, OS, architecture, .NET/Rust runtime, source revision, artifact checksum, disposable
profile identity, setup, request/action sequence, expected and observed results, cleanup, and
evidence level. Build-only and generic ABI evidence remain distinct from load smoke and runtime.

The runtime addon package requires an operator-supplied `sts2.dll` and `GodotSharp.dll`. Build and
stage it from WSL with:

~~~text
bash experiments/managed-rust-interop/package-runtime-addon.sh \
  "/mnt/c/Program Files (x86)/Steam/steamapps/common/Slay the Spire 2/data_sts2_windows_x86_64" \
  /tmp/sts2-runtime-addon
~~~

The script produces only the managed addon DLL, its unique Rust companion, and the manifest. A
manual host load-smoke may copy those three files into an authorized game's `mods/` directory and
launch the exact executable with a bounded `--quit-after` value. Include the standalone `--debug`
argument when validating the optional visible debug banner; without it, the load-smoke should
produce no in-game overlay. The observed marker, overlay state, and all host inputs must be
recorded in a separate evidence report; this is not part of ordinary CI. The completed report is
[`docs/evidence/runtime-v1-host-live-20260902.md`](evidence/runtime-v1-host-live-20260902.md).

## Settings-specific verification

The AI-Ascension addon owns its settings tab and does not require a ModConfig or other settings
framework mod. Unless the controls are separately exercised in an authorized disposable host, the
UI and live listener behavior remain source/build and load-smoke evidence only. Deterministic cases
must cover:

| Case | Required observation |
| --- | --- |
| Standalone registration | The addon installs its AI-Ascension tab without a framework dependency and without changing other mods' settings tabs. |
| Runtime settings contract | The tab exposes Runtime API, Bind address, Network port, Apply now, and Reset; defaults are enabled, `127.0.0.1`, and `15526`. |
| Runtime validation | Ports outside `1024` through `65535` and invalid address values are rejected without overwriting the last saved settings. |
| Live reconfiguration | Apply persists the values, stops only the native listener, starts it with the selected endpoint, and updates the listener status without restarting the game. Reset performs the same live update using defaults. |
| Environment overrides | `STS2_RUNTIME_PORT` and `STS2_RUNTIME_BIND_ADDRESS` retain precedence for automation; the bearer token remains outside the settings UI. |
| Launch unlock | Only the explicit CLI flag requests launch unlock; no persistent one-shot toggle exists. Each attempt ends on success, failure, or readiness timeout; a manual retry must be explicit. |
| Profile/run isolation | Active runs and pending, canceled, or failed run saves reject before profile switching or progress mutation; checks repeat on each deferred frame. |
| Manual retry and concurrency | Manual and launch requests share one readiness/main-thread attempt; concurrent requests do not double-save, while a failed, timed-out, or completed attempt can be retried without overlapping work. |
| Bounded diagnostics | Registration/read/write, listener, and profile failures produce bounded, sanitized categories only; logs contain no credentials, setting values, saves, private paths, or raw host exception details. |

The managed loader requires operator-supplied exact `sts2.dll` and `GodotSharp.dll` host assemblies;
they must remain outside the repository and package. Profile mutation and live listener
reconfiguration remain `unverified` unless separately exercised with disposable data in an
authorized host test that records the exact host tuple, setup, observations, and cleanup.

The production-linked synthetic unlock probe runs without proprietary assemblies or real profiles:

~~~text
dotnet run --project experiments/managed-rust-interop/settings-tests/SettingsValidationProbe.csproj --configuration Release
~~~

It covers invalid profile requests, active-run and save-task rejection, a run starting between
frames, selected-profile switching, concurrent-request status, readiness timeout/manual retry,
preservation of higher ascension values, and save-call failure reporting. Its original API-shaped
fakes are not evidence that an exact game version accepts those API calls or persists them safely.
The same probe writes only synthetic temporary settings files to check complete replacement,
partial-write failure preserving the previous bytes, rename failure, and temporary-file cleanup.
This checks atomic publication, not power-loss durability of the directory entry.

## Ephemeral session launcher

The owned launcher and Windows bridge always pass `--headless --audio-driver Dummy` to the game.
Source checks establish that the launcher requests these flags and contains no listed system-input
API calls. They do not establish that the proprietary host obeys the flags or cannot affect the
desktop. That requires a separately authorized exact-host test.

The target-owned launcher tests can run without a game or provider source:

~~~text
bash experiments/managed-rust-interop/session-launcher.test.sh
bash experiments/managed-rust-interop/session-bridge.test.sh
bash experiments/managed-rust-interop/provider-build.test.sh
bash experiments/managed-rust-interop/session-install.test.sh
bash experiments/managed-rust-interop/live-authorization.test.sh
bash experiments/managed-rust-interop/package-runtime-addon.test.sh
bash experiments/managed-rust-interop/dev-cycle.test.sh
experiments/managed-rust-interop/session-launcher.sh --self-test
~~

They cover OS-CSPRNG output of at least 32 bytes, bounded whitespace-free encoding, per-launch
credential difference, runtime/mod versus gateway role separation, argument-leakage absence,
missing/wrong/correct authorization status handling, the already-running refusal predicate,
bounded startup timeout, owned process-group cleanup, the stdin-only WSL-to-Windows bridge, and the
fail-closed `LIVE_AUTHORIZATION` preflight. The bridge project is also built as a managed
warning-as-error check. These are synthetic tests and do not prove that an external provider binary
or the proprietary game accepts the environment.

An authorized disposable live session uses the exact provider binaries or explicit source
directories from their owning target revisions:

~~~text
experiments/managed-rust-interop/session-launcher.sh \
  --game-dir "<STS2 install>" \
  --gateway-binary "<sts2-gateway-runtime>" \
  --harness-binary "<sts2-harness-runtime>" \
  --mcp-binary "<sts2-mcp-server>"
~~~

The default one-shot run must observe listener enabled, unauthenticated rejection, authenticated
game and gateway readiness, and successful harness/MCP completion, then observe owned-process
cleanup and closed listeners. `--keep-alive` is reserved for manual inspection and must be
interrupted before evidence is recorded. The command output is boolean-only; credentials, saves,
host assemblies, private paths, and provider logs remain outside the repository and CI artifacts.
Before that command can inspect or mutate a host, its complete non-secret `LIVE_AUTHORIZATION`
record must be exported. `--authorization-check` validates the record without host access; an
expired, incomplete, non-loopback, or provider-enabled record is rejected by the owned launcher.

The synthetic installation test uses only temporary fake payloads. It verifies renamed package
filenames, refusal on process-inspection failure, direct symlink refusal, and expired admission.
Authorization metadata is an operator attestation; these tests do not prove profile isolation.

Run the Windows bridge integration and mocked process-guard checks without a game:

~~~text
dotnet run --project experiments/managed-rust-interop/session-launcher/bridge-tests/SessionWindowsBridgeTests.csproj --configuration Release -warnaserror
pwsh -NoProfile -NonInteractive -File experiments/managed-rust-interop/dev-cycle-process-tests.ps1
~~~

The bridge test checks PID/start-time/executable identity and argument quoting on any platform;
on Windows it also launches only its own synthetic child to verify captured output reaches EOF
while that child lives, NUL output does not block, and wrong identities cannot terminate it.
Windows cases explicitly skip elsewhere. The process guard test mocks enumeration and process
objects; it never discovers or kills real game processes. Package tests substitute fake compilers
and path converters, not proprietary assemblies. Neither suite proves live WSL/game behavior.

## Security and evidence language

Security tests fail closed when a fixture or precondition is absent. Logs and fixtures contain no
credentials, saves, private paths, multiplayer identifiers, or proprietary host content.

Use confirmed for reproduced results, source-derived for planning or inspected documents,
proposed for future design, inferred for bounded reasoning, and unverified when the required
runtime or external evidence was not run.

## Runtime slice checks

`cargo test --locked --offline -p sts2-game-mod-interop` includes synthetic loopback regressions
for idle headers, incomplete bodies, progress that must not extend the absolute deadline,
nonreading response peers, stop interruption, and joining a stopped listener with an open peer.
Parser tests also reject a header terminator that crosses the 8-KiB header limit. These tests
use no game, profile, external service, or real credentials; generous elapsed-time ceilings
are hang detection, not a real-time scheduling guarantee. They do not prove that managed host
callbacks terminate safely, nor that native unload is safe in a live game.

~~~text
dotnet run --project experiments/managed-rust-interop/contract-tests/RuntimeContractProbe.csproj --configuration Release
~~~

This probe links the actual managed Runtime-v1 contract/validation/ABI types to original minimal
host doubles. It exercises strict required/unknown/duplicate fields, null request sentinels,
numeric kinds/bounds, header/body epoch agreement, malformed JSON and invalid callback context,
fresh versus stale actions, and refusal before exceeding the canonical 1024 probe-action count.
It also checks that an invoked host effect without a visible witness remains an unknown transport
outcome rather than a canonical rejection.
It emits synthetic request/response JSON for independent canonical-schema checks. It does not
establish concrete Godot rendering, live thread affinity, or full protocol conformance.

~~~text
dotnet run --project experiments/managed-rust-interop/queue-tests/RuntimeQueueProbe.csproj --configuration Release
~~~

This source-linked managed probe compiles the production queue, callback, and ABI types against
original minimal host doubles. It tests capacity, FIFO, canceled pending work removal, completed
publication, barrier-controlled execution timeout, immutable late response, callback unavailability,
and sanitized possibly-applied exception handling. It uses no proprietary assembly, game, or profile.
It does not verify the concrete host contract implementation or live thread affinity.

The native runtime crate has bounded parser/identity tests, Clippy coverage, and a Windows
x86-64 release cross-build. The managed loader project builds against operator-supplied
`sts2.dll` and `GodotSharp.dll` without copying those assemblies into the repository. The checked-in
`protocol-artifact/runtime-v1/` copy is the canonical message reference.

The authorized probe confirmed starting the v1 listener inside STS2, authenticated state/action
requests, main-thread queue execution, a visible overlay witness, and reversible disposable-profile
cleanup for the recorded host. The Runtime-v2 candidate build does not promote to live gameplay
evidence. Gameplay mutation/settlement, process supervision/restart, multi-instance behavior, other
host versions, and other platforms remain `unverified`. Do not count a successful build or ABI
load-smoke as any of those remaining runtime results.

## Workshop package checks

The owner-local Workshop contract is covered by deterministic Rust tests:

~~~text
cargo test --locked --offline -p sts2-game-mod --test workshop
~~~

The fixture-only package staging test uses synthetic payloads and a synthetic preview:

~~~text
bash tools/workshop/test-package-item.sh
~~~

The managed validator is exercised without a host assembly by the .NET 9 probe:

~~~text
dotnet run --project experiments/managed-rust-interop/workshop/WorkshopValidationProbe.csproj --configuration Release
~~~

These checks cover manifest shape, unknown fields, path traversal, duplicate/case-collision
paths, sorted inventory, allowlist and compatibility drift, install readiness, unexpected files,
reparse-point policy, file-size/digest mismatch, and content-digest mismatch. They do not prove
Steam App Admin settings, Steam callbacks, subscription/download behavior, game discovery, or
host compatibility.

## Review correction (2026-09-04)

Run the host-free regression executable with the pinned .NET SDK:

```text
dotnet run --project experiments/managed-rust-interop/host-candidate-tests/HostCandidateProbe.csproj --configuration Release
```

It links the actual v2 managed candidate files against handwritten synthetic host doubles.
After the main-safety rebase it also links the production shared callback, lifecycle, network,
queue and strict v1 contract sources. Candidate requests pass through `ProcessRuntimeWork`;
combined queue cases check one uncertain dispatch and removal before an expired dispatch.
It checks semantic replay, identity ownership, run freshness, and uncertain outcomes.
It does not prove exact-host ABI compatibility or gameplay behavior. CI runs this separately
from the generic managed ABI and Workshop probes.

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
## Runtime-v3 and co-op checks

The combined `HostCandidateProbe` now runs 47 source-linked synthetic checks: the 12 v2-only
regressions plus shared v2/semantic identity and mutation-exclusion cases in both directions,
delayed queue admission, uncertainty, exact retries, read-only reconciliation and release after
independent completion. It compiles production routing and handler sources against owned fakes.
These checks do not load a host, touch a profile/save, or contact a provider.

The actual managed Runtime-v3 handler is compiled and exercised without host assemblies:

~~~text
dotnet run --project experiments/managed-rust-interop/gameplay-tests/RuntimeV3ValidationProbe.csproj --configuration Release
~~~

The probe links production source and supplies only a synthetic host and queue. It
covers discovering a newer generation, exact-generation mutation/catalog fencing,
settled and pending receipt replay, payload conflicts, session/epoch isolation,
unrelated transitions remaining unknown, operation/action-bound completion, delayed
completion polling, queue-time generation checks, and malformed numeric/duplicate
fields. CI runs it with the pinned .NET SDK and warnings as errors. These are
managed component results, not licensed-host gameplay evidence.

The probe also covers read-only scoped reconciliation of delayed completion and
terminal rejection, immutable receipt snapshots, UTF-8 text bounds, invalid co-op
enums, and the scoped semantic combat helper. It compiles all host-independent
managed gameplay helpers rather than only the envelope handler.

`--emit-contract-frames` emits fifteen deterministic JSON lines from the actual
managed handler for independent canonical-protocol validation, including accepted,
unknown, settled and rejected receipt paths. The captured frames were decoded and
validated with protocol commit `82507361890c1bdce6cffeaf7e616d93e53a7d99` in the
organization review. This is cross-language message evidence, not network or host evidence.

The checked-in `protocol-artifact/runtime-v3-gameplay` copy, matching source schema
and conformance case preserve the producer's exact eight-file checksum inventory.
`runtime_v3_contract` tests hash those bytes, validate every golden and reject
missing nullable fields, unknown tagged fields, duplicate discriminators, invalid
provenance, stale digests and cross-field contradictions. Rust recovery tests also
exercise operation-bound completion, refreshed correlation, uncertain dispatch and
JSON-safe generation exhaustion without fake mutation.

Native loopback tests exercise all six Runtime-v3 method/route mappings against all six request
kinds, reversed HTTP methods, and malformed/duplicate/root-kind input. A synthetic callback
counter establishes that rejected combinations never cross the ABI and accepted combinations
retain gameplay callback kind 6 alongside v2 kinds 3–5. These are HTTP admission tests, not full message conformance
or managed/live-host mutation evidence; the existing auth and absolute-deadline tests still run.

Source-level tests cover the Runtime-v3 contract mirror, generation-bound action catalog, duplicate
operation identities, stale observations, host-thread queueing, settlement witnesses, projection
redaction, and explicit unknown outcomes. Co-op checks cover two-to-four peer bounds, one local
identity, generation disagreement, missing peers, disconnect, ally targeting, and mutation
suspension in the isolated helper. The gameplay host does not consume the co-op
helper, so these tests do not establish integrated co-op admission, a licensed
target build, or live multiplayer compatibility.
