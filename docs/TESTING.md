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
cargo run --locked --offline --package repo-policy -- --strict
cargo fmt --all --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
~~~

The workspace now also contains the target-owned host, HTTP-adapter, composition, and copied
`poc-v1` mapping seams. The commands prove source-level structure, queue/ABI/adapter composition,
artifact identity, and deterministic fake tests. They do not launch the game or prove host gameplay.

## Planned layers

| Layer | Purpose | Environment |
| --- | --- | --- |
| unit | validation, ABI values, error mapping, queue policy | any CI host |
| component | dispatcher, lifecycle, configuration, HTTP, fake host | deterministic local |
| protocol | exact owner-local HTTP shapes and golden fixtures | local/CI |
| integration | real bounded sockets and packaged components | isolated CI/local |
| host | loader, callbacks, main-thread and settlement behavior | exact disposable game |
| release smoke | packaged bytes and install/start behavior | authorized clean host |

Wave 2 claims unit/component coverage for the initialized ports, composition, and fake POC mapping
only. The POC artifact test is a local conformance check for copied bytes and message shape; no
host, integration, or release-smoke layer is claimed.

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
launch the exact executable with a bounded `--quit-after` value. The observed marker and all host
inputs must be recorded in a separate evidence report; this is not part of ordinary CI.

## Security and evidence language

Security tests fail closed when a fixture or precondition is absent. Logs and fixtures contain no
credentials, saves, private paths, multiplayer identifiers, or proprietary host content.

Use confirmed for reproduced results, source-derived for planning or inspected documents,
proposed for future design, inferred for bounded reasoning, and unverified when the required
runtime or external evidence was not run.
