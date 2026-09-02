# sts2-game-mod

Status: the target-owned boundary seams and one deterministic `poc-v1` fake mapping compile and
have tests; no game behavior or game package is claimed.

## Responsibility and consumers

The mod owner maintains the managed loader, host translation, main-thread boundary, authoritative
local HTTP adapter, and narrow Rust/native seam. The game host is the authority for live state and
mutations. The future gateway consumes the mod's owner-local HTTP contract; MCP and harness traffic
reaches it only through their separate gateway and coordinator responsibilities.

This target does not own domain policy, gateway lifecycle or routing, MCP framing, or
model/provider orchestration. It consumes the checked-in `sts2-protocol/poc-v1` release-like
artifact as inert data; it does not link a protocol implementation or a sibling repository.

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

The managed loader, real host adapter, route catalog, and packaging remain unimplemented. The
POC's core port is a fake seam in this repository, not the game implementation or a claim that the
future host can perform the same action.

## Evidence and provenance

The foundation and boundary seam are original target documentation and source tailored from the project
standards. The interop experiment predates this foundation and is retained in place. No product
implementation source was copied from another implementation. No
proprietary host file, save, credential, personal path, or generated output is distributed.

The POC behavior is test-confirmed only for the local fake core port: a state read, one accepted
action, and one zero-unit rejection preserve the bounded state and effect witness. Runtime
behavior remains unverified: the game has not been launched, the loader has not been discovered by
the game, no host assembly is stored here, no local HTTP listener is exposed, and no real
main-thread queue or mutation has been exercised. The generic ABI experiment is not a mod-load
claim.

## Local validation

Run the policy checker from this directory:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Then run formatting, Clippy, and workspace tests as documented in
[docs/TESTING.md](docs/TESTING.md). These local gates prove repository and source-level invariants
only; they do not promote host compatibility beyond the evidence level recorded above.
