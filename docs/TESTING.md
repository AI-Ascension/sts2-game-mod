# Testing and evidence

## Purpose

Tests must prove observable boundary invariants without requiring a game whenever possible.
Host-dependent evidence runs only in an authorized disposable environment and records its exact
inputs, outputs, cleanup, and evidence level.

## Foundation commands

Run from the target root:

~~~text
cargo metadata --locked --no-deps --format-version 1
sha256sum -c protocol-artifact/poc-v1/SHA256SUMS
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
separate dated host report records the authorized runtime lane; these ordinary commands still do
not launch the game or prove gameplay or Runtime-v2 host settlement.

## Runtime-v2 deterministic seam

The focused `runtime_v2` test covers one admitted and settled `end_turn`, exactly-once fake
application, duplicate replay, conflicting operation identity, outside-combat and enemy-turn
rejection, stale generation and identity fencing, queue and receipt bounds, cancellation timing,
post-write disconnect reconciliation, and pre-dispatch timeout removal. `sha256sum -c` verifies the
copied release-like artifact from repository-relative paths. No test invokes STS2, a concrete host
gameplay API, `AutoProfileUnlock`, or any persistent profile/save/provider path.

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
| One-shot reset | A launch unlock request remains enabled after any failed or incomplete operation. It is cleared only after the profile save succeeds and the settings write succeeds with a confirmed read-back of `false`. |
| Manual retry and concurrency | Manual and launch requests share one readiness/main-thread attempt; concurrent requests do not double-save, while a failed, timed-out, or completed attempt can be retried without overlapping work. |
| Bounded diagnostics | Registration/read/write, listener, and profile failures produce bounded, sanitized categories only; logs contain no credentials, setting values, saves, private paths, or raw host exception details. |

The managed loader requires operator-supplied exact `sts2.dll` and `GodotSharp.dll` host assemblies;
they must remain outside the repository and package. Profile mutation and live listener
reconfiguration remain `unverified` unless separately exercised with disposable data in an
authorized host test that records the exact host tuple, setup, observations, and cleanup.

## Security and evidence language

Security tests fail closed when a fixture or precondition is absent. Logs and fixtures contain no
credentials, saves, private paths, multiplayer identifiers, or proprietary host content.

Use confirmed for reproduced results, source-derived for planning or inspected documents,
proposed for future design, inferred for bounded reasoning, and unverified when the required
runtime or external evidence was not run.

## Runtime slice checks

The native runtime crate has bounded parser/identity tests, Clippy coverage, and a Windows
x86-64 release cross-build. The managed loader project builds against operator-supplied
`sts2.dll` and `GodotSharp.dll` without copying those assemblies into the repository. The checked-in
`protocol-artifact/runtime-v1/` copy is the canonical message reference.

The authorized probe confirmed starting the listener inside STS2, authenticated state/action
requests, main-thread queue execution, a visible overlay witness, and reversible disposable-profile
cleanup for the recorded host. Gameplay mutation, process supervision/restart, multi-instance
behavior, other host versions, and other platforms remain `unverified`. Do not count a successful
build or ABI load-smoke as any of those remaining runtime results.
