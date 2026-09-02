<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Runtime: unverified. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# sts2-game-mod

> **AI-Ascension · tier 1: game-process adapter** — Game-process adapter: a bounded main-thread work queue, versioned ABI check, and HTTP request admission limits.
>
> **Status:** deterministic tests, managed load-smoke, and one exact-host runtime probe `confirmed` · host gameplay and broader compatibility `unverified`.
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-game-mod)
> **Owner:** The mod owner is responsible for the managed loader package, host boundary, main-thread queue, ABI gate, HTTP admission, and Rust/native seam; the game host stays authoritative.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: the target-owned boundary seams and one deterministic `poc-v1` fake mapping compile and
have tests. A thin managed loader package now loads the Rust companion and has passed a real
load-smoke launch against the recorded STS2 host; gameplay remains outside this slice. The bounded
runtime probe has also passed in an authorized disposable profile.

## Responsibility and consumers

The mod owner maintains the managed loader, host translation, main-thread boundary, authoritative
local HTTP adapter, and narrow Rust/native seam. The game host is the authority for live state and
mutations. The future gateway consumes the mod's owner-local HTTP contract; MCP and harness traffic
reaches it only through their separate gateway and coordinator responsibilities.

This target does not own domain policy, gateway lifecycle or routing, MCP framing, or
model/provider orchestration. It consumes the checked-in `sts2-protocol/poc-v1` release-like
artifact as inert data; it does not link a protocol implementation or a sibling repository.

## Current contents

- [experiments/managed-rust-interop/](experiments/managed-rust-interop/) contains the managed .NET 9
  loader package, its unique Rust companion library, the manifest, and the reproducible packaging
  command. The package now contains the load-smoke/ABI path and the bounded runtime probe source.
- [crates/host/](crates/host/) owns the host port, bounded main-thread queue, dispatcher, and
  versioned ABI descriptor validation.
- [crates/http-adapter/](crates/http-adapter/) owns a transport-free HTTP request boundary and
  bounded admission guard; it does not open a listener or define public routes.
- [crates/game-mod/](crates/game-mod/) composes those seams and makes admission-versus-pump
  behavior explicit for the future managed host integration.
- [protocol-artifact/poc-v1/](protocol-artifact/poc-v1/) is the offline copied artifact consumed by
  the deterministic mod/core boundary test.
- [protocol-artifact/runtime-v2/](protocol-artifact/runtime-v2/) is the offline copied release-like
  artifact for the bounded Runtime-v2 fake seam; it is pinned to schema digest
  `f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2` and has no sibling checkout
  dependency.
- `crates/game-mod/src/poc/` maps state reads and one typed `use_budget` action through a narrow
  `PocCorePort`, records correlation/instance/generation metadata, and emits one settled-effect
  witness for an accepted action.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) records the target boundary and dependency graph.
- [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) records evidence levels and host claims.
- [docs/evidence/runtime-addon-load-smoke-20260902.md](docs/evidence/runtime-addon-load-smoke-20260902.md)
  records the exact installed-host load-smoke inputs and observed log marker.
- [docs/evidence/runtime-v1-host-live-20260902.md](docs/evidence/runtime-v1-host-live-20260902.md)
  records the focused runtime probe against the exact installed host and disposable profile.
- [docs/decisions/](docs/decisions/) records the managed/native, ownership, scaffold, and
  sixth-target and Wave 2 initialization decisions.
- [tools/repo-policy/](tools/repo-policy/) is the target-local Rust governance checker.

The managed loader, packaging, and bounded runtime route source are implemented for this sprint.
The broader host adapter, gameplay action implementation, and game-rule mutation remain
unimplemented. The POC's core port is still a fake seam in this repository, not the game
implementation or a claim that the host can perform a gameplay action.

The Runtime-v2 seam is also fake-only: it proves bounded admission, exactly-once in-memory
application, retained receipts, and reconciliation for argument-free `end_turn`. No concrete host
gameplay API exists in this repository; live host mutation and settlement are unverified.

## Evidence and provenance

The foundation and boundary seam are original target documentation and source tailored from the project
standards. No product implementation source was copied from another implementation. No proprietary
host file, save, credential, personal path, or generated output is distributed. The local load-smoke
uses the operator's installed host assembly without adding it to this repository or release output.

The POC behavior is test-confirmed only for the local fake core port: a state read, one accepted
action, and one zero-unit rejection preserve the bounded state and effect witness. The managed
loader has also passed load-smoke in STS2 v0.107.1 and logged a successful Rust ABI call. The
runtime-v1 probe has confirmed live HTTP, managed main-thread dispatch, and a host-visible overlay
witness in that exact host. Gameplay mutation, effect semantics beyond the probe, and broader
compatibility remain unverified.

## Local validation

Run the policy checker from this directory:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Then run formatting, Clippy, and workspace tests as documented in
[docs/TESTING.md](docs/TESTING.md). These local gates prove repository and source-level invariants
only; they do not promote host compatibility beyond the evidence level recorded above.

## Bounded runtime slice

The interop package now includes an authenticated runtime adapter with two fixed routes (default
bind address `127.0.0.1`): `GET /api/v1/runtime/state` and `POST /api/v1/runtime/action`. The native
listener validates bounded HTTP input and identity headers, then invokes the managed bridge. Managed
work is queued and executed from the Godot `SceneTree.ProcessFrame` callback. The only accepted
action is the safe, host-visible `show_runtime_probe`; acceptance requires the status overlay to be
observed and returns the fresh `status_overlay_visible` witness defined by the copied
[`runtime-v1` artifact](protocol-artifact/runtime-v1/README.md).

The listener requires `STS2_RUNTIME_TOKEN` and the built-in Runtime API toggle. Its bind address and
port are staged, persisted, and applied immediately by the AI-Ascension settings tab;
`STS2_RUNTIME_BIND_ADDRESS` and `STS2_RUNTIME_PORT` override those values for automation. The
listener is disabled when its toggle is off, its token is absent, or its address or port is invalid.
The exact STS2 v0.107.1 Windows x86-64 probe is recorded as confirmed in the dated host evidence;
the package still does not implement gameplay mutation or claim compatibility with another host or
platform.
