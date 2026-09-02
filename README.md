<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-dark.svg">
  <img alt="AI-Ascension — Inspect how AI requests to a game get fenced, one Rust contract at a time. Runtime: unverified. Deterministic tests: confirmed." src="https://raw.githubusercontent.com/AI-Ascension/.github/main/profile/assets/banner-light.svg" width="100%">
</picture>

# sts2-game-mod

> **AI-Ascension · tier 1: game-process adapter** — Game-process adapter: a bounded main-thread work queue, versioned ABI check, and HTTP request admission limits.
>
> **Status:** deterministic in-memory tests `confirmed` at the pinned commit · runtime, host, and game compatibility `unverified` · nothing is live.
> **Proof:** [45-second browser replay](https://ai-ascension.github.io/proof.html) · [Evidence ledger](https://ai-ascension.github.io/evidence.html) · [This repository on the map](https://ai-ascension.github.io/repositories.html#sts2-game-mod)
> **Owner:** The mod owner is responsible for the host boundary: main-thread queue, ABI gate, HTTP admission, and the Rust/native seam; the managed loader remains unimplemented; the game host stays authoritative.
> **Contribute:** [Organization guide](https://github.com/AI-Ascension/.github/blob/main/CONTRIBUTING.md) · [First tasks](https://ai-ascension.github.io/contributing.html)
>
> AI-Ascension is an independent project. It is not affiliated with or endorsed by Mega Crit or Valve and grants no rights to game files, assets, or marks.

Status: Wave 2 codebase initialization complete. The target-owned boundary seams compile and have
deterministic fake tests; no game behavior or game package is claimed.

## Responsibility and consumers

The mod owner maintains the managed loader, host translation, main-thread boundary, authoritative
local HTTP adapter, and narrow Rust/native seam. The game host is the authority for live state and
mutations. The future gateway consumes the mod's owner-local HTTP contract; MCP and harness traffic
reaches it only through their separate gateway and coordinator responsibilities.

This target does not own domain policy, gateway lifecycle or routing, MCP framing, model/provider
orchestration, experiment artifacts, or the accepted sixth sts2-protocol repository. Shared
contracts remain limited to language- and transport-neutral material with an independently decided
owner.

## Current contents

- [experiments/managed-rust-interop/](experiments/managed-rust-interop/) is the preserved,
  source-only managed .NET 9 to Rust C ABI experiment. It is not production code and has not been
  installed into a game profile.
- [crates/host/](crates/host/) owns the host port, bounded main-thread queue, dispatcher, and
  versioned ABI descriptor validation.
- [crates/http-adapter/](crates/http-adapter/) owns a transport-free HTTP request boundary and
  bounded admission guard; it does not open a listener or define public routes.
- [crates/game-mod/](crates/game-mod/) composes those seams and makes admission-versus-pump
  behavior explicit for the future managed host integration.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) records the target boundary and dependency graph.
- [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) records evidence levels and host claims.
- [docs/decisions/](docs/decisions/) records the managed/native, ownership, scaffold, and
  sixth-target and Wave 2 initialization decisions.
- [tools/repo-policy/](tools/repo-policy/) is the target-local Rust governance checker.

The managed loader, real host adapter, route catalog, domain behavior, and packaging remain
unimplemented. The initialized Rust crates are source-only ports and composition tests, not a
replacement for the managed loader or a game implementation.

## Evidence and provenance

The foundation and boundary seam are original target documentation and source tailored from the project
standards. The interop experiment predates this foundation and is retained in place. No product
implementation source was copied from sts2-harness-rust or any other implementation. No
proprietary host file, save, credential, personal path, or generated output is distributed.

Runtime behavior is unverified: the game has not been launched, the loader has not been discovered
by the game, no host assembly is stored here, no local HTTP listener is exposed, and no real
main-thread queue or mutation has been exercised. The deterministic fake tests cover only the
source-level queue, ABI gate, bounded adapter, and composition seam. The generic ABI experiment is
not a mod-load claim.

## Local validation

Run the policy checker from this directory:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Then run formatting, Clippy, and workspace tests as documented in
[docs/TESTING.md](docs/TESTING.md). These local gates prove repository and source-level invariants
only; they do not promote host compatibility beyond the evidence level recorded above.
