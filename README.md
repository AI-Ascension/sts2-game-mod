<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Runtime: unverified. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# sts2-game-mod

> **AI-Ascension · tier 1: game-process adapter** — Game-process adapter: a bounded main-thread work queue, versioned ABI check, and HTTP request admission limits.
>
> **Status:** deterministic tests and managed load-smoke `confirmed` at the pinned commit · host gameplay, HTTP, and compatibility `unverified`.
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-game-mod)
> **Owner:** The mod owner is responsible for the managed loader package, host boundary, main-thread queue, ABI gate, HTTP admission, and Rust/native seam; the game host stays authoritative.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: the target-owned boundary seams and one deterministic `poc-v1` fake mapping compile and
have tests. A thin managed loader package now loads the Rust companion and has passed a real
load-smoke launch against the recorded STS2 host; gameplay and HTTP behavior remain outside this
slice.

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
  command. The package is intentionally limited to load-smoke and ABI evidence.
- [crates/host/](crates/host/) owns the host port, bounded main-thread queue, dispatcher, and
  versioned ABI descriptor validation.
- [crates/http-adapter/](crates/http-adapter/) owns a transport-free HTTP request boundary and
  bounded admission guard; it does not open a listener or define public routes.
- [crates/game-mod/](crates/game-mod/) composes those seams and makes admission-versus-pump
  behavior explicit for the future managed host integration.
- [protocol-artifact/poc-v1/](protocol-artifact/poc-v1/) is the offline copied artifact consumed by
  the deterministic mod/core boundary test.
- `crates/game-mod/src/poc/` maps state reads and one typed `use_budget` action through a narrow
  `PocCorePort`, records correlation/instance/generation metadata, and emits one settled-effect
  witness for an accepted action.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) records the target boundary and dependency graph.
- [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) records evidence levels and host claims.
- [docs/decisions/](docs/decisions/) records the managed/native, ownership, scaffold, and
  sixth-target and Wave 2 initialization decisions.
- [tools/repo-policy/](tools/repo-policy/) is the target-local Rust governance checker.

The managed loader and packaging are implemented for the load-smoke slice. The real host adapter,
route catalog, and game action implementation remain unimplemented. The POC's core port is still
a fake seam in this repository, not the game implementation or a claim that the host can perform
the same action.

## Evidence and provenance

The foundation and boundary seam are original target documentation and source tailored from the project
standards. No product implementation source was copied from another implementation. No proprietary
host file, save, credential, personal path, or generated output is distributed. The local load-smoke
uses the operator's installed host assembly without adding it to this repository or release output.

The POC behavior is test-confirmed only for the local fake core port: a state read, one accepted
action, and one zero-unit rejection preserve the bounded state and effect witness. The managed
loader has also passed load-smoke in STS2 v0.107.1 and logged a successful Rust ABI call. Live HTTP,
main-thread queue behavior, host state/action mutation, effect settlement, and broader compatibility
remain unverified.

## Local validation

Run the policy checker from this directory:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Then run formatting, Clippy, and workspace tests as documented in
[docs/TESTING.md](docs/TESTING.md). These local gates prove repository and source-level invariants
only; they do not promote host compatibility beyond the evidence level recorded above.
