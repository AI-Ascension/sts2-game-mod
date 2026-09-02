# Contributing

Contributions to sts2-game-mod must preserve a narrow, reviewable game-facing boundary. Start
with [AGENTS.md](AGENTS.md), then read the documents relevant to the change:

- [docs/PRODUCT.md](docs/PRODUCT.md) for scope and non-goals;
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for ownership and dependency direction;
- [docs/CODING_STANDARDS.md](docs/CODING_STANDARDS.md) for Rust and managed code;
- [docs/TESTING.md](docs/TESTING.md) for deterministic and host evidence;
- [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md) for support claims;
- [docs/LICENSING.md](docs/LICENSING.md) for provenance and notices;
- [docs/WORKFLOWS.md](docs/WORKFLOWS.md) for automation and review;
- [RELEASING.md](RELEASING.md) for publication authority; and
- [SECURITY.md](SECURITY.md) for private vulnerability reports.

## Scope before implementation

Define a project-owned requirement and acceptance test before adding an HTTP route, field, error,
ABI symbol, host callback, queue rule, or package behavior. A change to ownership, dependency
direction, process topology, host strategy, public contract, or security posture needs a decision
record in docs/decisions.

The Rust/native experiment is evidence, not a product API. Do not copy or transliterate source
from another harness. Do not add a product crate or a placeholder crate without an identified
responsibility, consumer, and test seam.

## Safe changes

Preserve unrelated work and use apply_patch for edits. Do not add proprietary assemblies, game
assets, saves, credentials, private paths, or build output. Do not initialize Git or perform
commit, push, merge, release, deployment, installation, or game-runtime actions unless separately
authorized.

Keep the managed exception narrow: loader metadata, host callbacks, native-library lifetime, and
ABI translation only. Keep host calls on the game thread. Keep HTTP decoding and response mapping
at the HTTP boundary. Keep gateway, MCP, harness, core, and protocol concerns in their owning
targets.

## Required evidence

Run and report the exact commands applicable to the change:

~~~text
cargo run --locked --package repo-policy -- --strict
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-targets --all-features --locked
~~~

Host claims additionally require an exact game and assembly identity, platform, runtime, artifact
hash, disposable test data, setup, observed result, cleanup, and evidence level. A build or generic
ABI call is not load or runtime proof. A skipped check remains unverified.

## Pull requests

Explain the problem, responsibility owner, affected contracts, compatibility classification,
security/data impact, exact commands and results, documentation changes, and remaining blockers.
Update [CHANGELOG.md](CHANGELOG.md) for user-visible behavior. Reviewers must be able to identify
provenance for generated or adapted material and confirm that no private or proprietary data is
present.
