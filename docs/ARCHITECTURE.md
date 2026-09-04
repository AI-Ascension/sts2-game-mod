# Architecture

## Responsibility

sts2-game-mod is the game-facing translation boundary. It adapts the managed loader and host
callbacks into owned Rust values, schedules host work on the game main thread, exposes the
authoritative local HTTP surface, and composes the narrow native seam.

This document records boundaries, the initialized source-level seams, the managed load-smoke
package, and dependency direction. The Rust source proves deterministic port composition plus one
local fake `poc-v1` mapping; the managed package separately proves loader discovery and the native
ABI smoke call in one recorded game version.

## Initialized source seam

`crates/host` contains the target-owned `HostPort`, owned request and snapshot projections, a
bounded FIFO main-thread queue, a deterministic dispatcher, and a versioned ABI descriptor gate.
`crates/http-adapter` accepts an already-framed request view and enforces a body bound without
opening a socket or defining a route catalog. `crates/game-mod` composes those ports so an HTTP
admission is queued before a host pump can submit it. Fake host, queue, ABI, adapter, and
composition tests make those source-level invariants deterministic.

This seam is intentionally opaque and preparatory. It is not the managed loader, a public HTTP
schema, a game-rule implementation, or a native implementation copied from another repository.

## Runtime addon package

`experiments/managed-rust-interop/game-loader` is the loader-facing .NET 9 class library. It
references only the operator-supplied `sts2.dll` and `GodotSharp.dll` at build time, carries the
host's `ModInitializer` metadata, loads the adjacent unique native companion, verifies ABI version
1, calls the checked-add smoke export, logs a bounded success marker, and places a top-layer Godot
debug banner with the verified ABI and result when the process was launched with the exact
`--debug` argument. The companion is built from the target-owned Rust crate as
`AIAscensionSTS2GameModNative.dll` on Windows. The package script stages only the managed DLL, native DLL,
and manifest; it never copies proprietary host assemblies into the repository or package.

This is a load-smoke implementation, not the game behavior implementation. It does not expose an
HTTP listener as part of the loader-only smoke path, access host game objects outside the bounded
runtime callback, mutate a run, or claim gameplay action/effect compatibility. The separate runtime
bridge described below owns the new host-visible probe.

### Ephemeral session orchestration

`experiments/managed-rust-interop/session-launcher.sh` is a development/test orchestrator owned by
this target. It does not move gateway, harness, or MCP ownership into the mod. It receives explicit
provider binaries or source directories, starts each provider in a dedicated POSIX process group,
and starts the Windows game through the checked-in .NET bridge. The launcher generates a fresh
runtime/mod credential and a different gateway credential with the OS CSPRNG for each session. The
runtime credential is supplied to the game over bridge stdin and to the gateway as `STS2_MOD_TOKEN`;
the gateway credential is supplied only as `STS2_GATEWAY_TOKEN` to the gateway and harness/MCP
chain. No credential is persisted or placed in an argument or log.

The bridge is the explicit WSL-to-Windows environment boundary. It sets `STS2_RUNTIME_TOKEN`, the
selected loopback-default bind address and port, and the non-secret `STS2_RUNTIME_SESSION=1` on the
game process. The session flag is accepted only as an ephemeral launch override; saved settings
remain owned by the in-game panel. Readiness probes require unauthenticated rejection followed by
authenticated success, use bounded timeouts, and cleanup targets only recorded child groups and
the recorded game PID. An already-running game is refused rather than adopted or killed.

### Built-in settings boundary

`StandaloneProfileSettings` owns the AI-Ascension tab; it does not require ModConfig. It clones a
native settings tab and panel using the host's private `_tabs` seam, so exact-host rendering and
focus compatibility require separate evidence. ADR 0011's optional ModConfig bridge is historical,
not the current implementation. The current controls are Runtime API, bind address, port, Reset,
target profile, and Apply full profile unlock. The CLI `--debug` and
`--ai-ascension-unlock-all` remain available; there is no persisted one-shot unlock toggle.

Profile unlocking is an explicit persistent progress mutation. The manual action may switch the
active profile to the selected profile; the launch argument uses the initialized current profile.
Both run on a deferred main-thread frame and reject active runs and pending, canceled, or failed
run saves before switching or mutating. Readiness is bounded to 600 frames and rechecked after a
profile switch. The operation marks content discovered and raises ascension to at least 10 without
lowering existing maxima. It invokes the host `SaveProgressFile` once per attempt and does not
report success if that call throws. This does not establish crash-atomic progress persistence or
undo partially applied in-memory changes after failure. Manual retry is explicit.

Production-linked synthetic tests cover these guards without a host or real profiles. They do not
prove the proprietary API shape, disk durability, settings rendering, or live profile safety.

## Minimal POC mapping

`protocol-artifact/poc-v1/` is a checked-in, release-like copy produced by the protocol owner. The
mod verifies its manifest and schema identity locally, then maps the four message shapes without a
cross-repository dependency. `PocMod` accepts only a state request or an action request, forwards
the typed `use_budget` action through `PocCorePort`, and returns the bounded observation with the
same protocol version, schema digest, correlation ID, instance ID, and generation. An accepted
action creates one `EffectWitness`; a core rejection returns its stable error identity and creates
no witness.

The test's core implementation is a fake. The mapping is source/test evidence for the boundary,
not evidence of host callbacks, a local listener, game mutation, or settlement in STS2.

## Runtime-v2 fake boundary

`protocol-artifact/runtime-v2/` is the exact owner-local copy of the release-like Runtime-v2
artifact from protocol commit `8d4b2f574cf860a71f2a5e4ce3308ac069cb1527`. Its schema digest is
`f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2`, its recorded generator is
`hand-authored`, and `SHA256SUMS` is verified from this repository. The Rust consumer includes
local bytes and has no sibling-checkout or path dependency.

`RuntimeV2Mod` owns the bounded in-memory operation receipt store and queue. The only game port in
this wave is `FakeRuntimeV2Game`; it advances a copied observation for the argument-free
`end_turn` action and never reaches a game object. The seam proves admission, exactly-once fake
application, fresh settlement witnesses, idempotent replay, fail-closed identity/generation and
phase checks, bounded capacity, cancellation timing, unknown outcomes, reconciliation, and timeout
removal. Runtime-v1 routes and tests are unchanged.

No concrete host gameplay API exists in this repository. The Runtime-v2 fake is deterministic
source/build/test evidence, not live STS2 gameplay evidence; live host mutation and settlement are
unverified.

## Repeat-seed practice replay

`SeedReplayController` and the standalone settings partials own the narrow repeat-seed feature in
the managed/game boundary. The persisted toggle is off by default. The one-shot action snapshots
only the active host run's character, acts, modifiers, seed, and ascension, then schedules one
main-thread operation. It accepts only a single-player, one-player Custom run; standard, daily,
multiplayer, and missing-state paths fail closed.

After confirmation, the controller uses `RunManager.CleanUp(graceful: true)`,
`SaveManager.DeleteCurrentRun`, and `NGame.StartNewSingleplayerRun` with `GameMode.Custom`. It
does not access save files directly or create a run-history entry. This resets to the seed's
beginning; it is not a checkpoint or later-floor restoration mechanism. The exact-host UI and
gameplay behavior remain unverified until a disposable host test is recorded in
[`repeat-seed.md`](repeat-seed.md).

## System boundaries

The target crosses four distinct boundaries:

1. The managed loader boundary supplies the host-required metadata and keeps native-library
   lifetime explicit.
2. The host boundary translates host objects, callbacks, and thread affinity into owned values
   and commands.
3. The HTTP boundary decodes bounded requests, maps them to the host/application ports, and
   returns sanitized responses and errors.
4. The native seam carries only reviewed, versioned, transport-neutral ABI values.

The host remains authoritative for legal game state and mutation effects. An accepted request is
not reported as completed until the host reaches the corresponding result.

## Component ownership

| Component | Owns | Must not own |
| --- | --- | --- |
| managed loader | load metadata, callback entry, native lifetime, ABI translation | domain rules, HTTP, MCP, persistence |
| host boundary | host adaptation, main-thread dispatch, lifecycle observation | public MCP or gateway policy |
| HTTP adapter | local route decoding, response mapping, bounded errors | host object internals or gateway leases |
| game-mod composition | wiring of target-local components | a second domain or transport implementation |
| core target | host-independent semantics and policy | HTTP, host APIs, processes, filesystems |
| gateway target | lifecycle, routing, leases, isolation, control plane | game rules or host ABI |
| MCP target | thin MCP-to-gateway translation | host access, game rules, lifecycle ownership |
| harness target | coordination, explicit instance context, experiments, artifacts | game authority or wire reinterpretation |
| protocol target | only approved shared language/transport-neutral contracts | target-specific host or transport behavior |
| session launcher | disposable process/env orchestration and readiness evidence | provider implementation, saved settings, credentials, gameplay |

## Dependency direction

The intended direction is:

~~~text
protocol (if accepted) <- core <- mod host boundary
                              ^        ^
                              |        +-- HTTP adapter
                              +-- gateway
                                    ^
                               MCP server
                               harness
~~~

The diagram is a boundary map, not permission to add dependencies now. The mod must not depend
on MCP, gateway control-plane internals, harness model code, or proprietary host source. A future
protocol dependency must be justified by a genuinely shared contract and a recorded owner.

## Main-thread and lifecycle invariants

- Host reads and mutations execute on the game main thread.
- Network intake may be off-thread, but queued host work has bounded capacity and explicit overload.
- Accepted work resolves to success, failure, or cancellation; it is never silently dropped.
- FIFO ordering, cancellation before and after acceptance, timeout meaning, and shutdown behavior
  are explicit contract decisions.
- Host calls do not occur while holding unrelated locks.
- Native unload is not assumed safe until a host lifecycle test proves it.
- Thread, callback, pointer, lifetime, allocator, and panic assumptions are documented at the ABI.

## Protocol and data rules

The owner-local HTTP contract remains separate from the copied POC artifact. The runtime adapter owns
the bounded `runtime-v1` route/ABI mapping and its sanitized errors; MCP envelopes and gateway leases
remain outside this repository. Broader gameplay routes, semantics, ordering, and versioning require
later project-owned requirements and fixtures.

Host objects never cross the HTTP or native boundary. Convert them to owned, validated values;
never expose debug strings, panic text, private paths, save contents, or raw host references.

## Evidence status

The load-smoke report confirms game discovery, managed initializer invocation, and the paired native
ABI call for one exact installed host. The focused runtime report additionally confirms the bounded
listener, managed main-thread dispatch, and host-visible probe effect for that same host. It does not
prove game mutation, safe fault isolation, or compatibility with another host or platform.

## Runtime adapter slice

ADR 0010 adds a narrow owner-local host path around the shared `runtime-v1` artifact. The native
companion binds a configured local address (default `127.0.0.1`), enforces bounded
HTTP/header/body/response sizes, requires a bearer token, and admits only `/health/ready`,
`/api/v1/runtime/state`, and `/api/v1/runtime/action`. The built-in AI-Ascension settings tab
applies and persists the enablement, address, and port immediately, while
`STS2_RUNTIME_BIND_ADDRESS` and `STS2_RUNTIME_PORT` remain explicit environment overrides. It
passes borrowed request bytes through a versioned C ABI callback; the managed side copies them into
owned values before queueing.

Each accepted connection has one absolute 10-second socket-I/O deadline, including header/body
reads and response writes; partial progress does not reset it. Socket waits poll the stop flag
at most every 50 milliseconds, subject to OS scheduling. The nonblocking accept loop needs no
self-connection to stop. Shutdown still joins the listener thread before releasing callback
lifetime; the synchronous managed callback must enforce its own execution bound (currently a
five-second queue wait). Socket deadlines do not cancel host work already admitted or forcibly
interrupt a managed callback. An expired exchange can close without an HTTP error body.

The managed bridge installs one bounded queue pump on `SceneTree.ProcessFrame`. Admission and
callback execution are coordinated through a 64-entry pending queue with atomic admission,
execution claim, and timeout removal; at most 16 entries execute per frame. A callback timeout
after execution starts reports an unknown transport outcome, never a false rejection. Error paths
on the network callback do not read host observations. The source-linked queue regression suite
does not establish exact-host compatibility. State observation and
the `show_runtime_probe` action run on that host thread. The action adds the existing status overlay,
checks that the overlay is present, then advances generation and emits the effect witness. The
listener, token, identity, lease, and correlation checks are owner-local enforcement; the gateway
still owns external authorization, lease issuance, and fencing.

The route/ABI implementation and tests are confirmed at source/build level, and the dated host
report confirms a real request, live main-thread dispatch, and the bounded host-visible effect for
STS2 v0.107.1 on Windows x86-64. Game-rule compatibility, process supervision, and the action's
semantics beyond this probe remain unverified. The action is deliberately not a gameplay mutation.

## Steam Workshop boundary

[ADR 0015](decisions/0015-steam-workshop-first-party-package.md) adds a first-party executable Workshop package path for the existing runtime addon. The
Rust module at crates/game-mod/src/workshop.rs is a pure owner-local contract: it parses bounded
manifests, enforces sorted hashed file roles and safe relative payload paths, maps Steam install
states to wait/ready/reject decisions, and compares an item against an exact package policy.

The managed loader's WorkshopContent.cs is the filesystem and loader gate. When a
sts2-workshop-manifest.json is present beside the managed assembly, the loader requires the
operator-configured App ID, published file ID, game version, and platform, rejects unexpected
entries and reparse points, verifies payload sizes and SHA-256 values, and checks the deterministic
content digest before loading the native companion. A package without this marker remains
compatible with the existing local/load-smoke path; a marked package fails closed when its policy
is absent or mismatched.

tools/workshop/package-item.sh stages only the exact managed assembly, loader manifest, and native
companion, emits the manifest/checksum inventory, and writes the operator-only Steam Workshop VDF
outside the content directory. The Steam API callback/initialization adapter is intentionally not
implemented without the Steamworks SDK. Steam publication, subscription/download behavior, and
Workshop-driven game discovery are therefore still unverified.
