# Agent operating contract

## Target

sts2-game-mod owns the game-facing boundary of the STS2 system: the managed loader entry point,
host integration, main-thread dispatch, authoritative local HTTP adapter, and the narrow Rust/native
seam. The mod captain owns this target tree only.

Wave 1 established the repository foundation. Wave 2 initializes only the target-owned host,
HTTP-adapter, and composition seams with deterministic fakes; it does not add game behavior,
public routes, or a real loader. The existing managed-rust-interop experiment is preserved as
source-only evidence and is not product behavior. Do not add further crates or placeholders merely
to satisfy a directory plan.

## Boundary rules

- The game host remains authoritative for state, legal mutations, and thread affinity.
- Managed code is limited to loader metadata, host callbacks, native-library lifetime, and ABI
  translation.
- Host objects must become owned values before they cross a thread or process boundary.
- The HTTP adapter owns request decoding, bounded responses, route mapping, and sanitized errors.
- The Rust/native seam uses fixed-width types, explicit ownership, versioned ABI data, and reviewed
  unsafe blocks.
- The core target is host-, transport-, process-, and filesystem-independent.
- The gateway owns lifecycle, routing, leases, isolation, and control-plane policy.
- The MCP server is a thin protocol adapter and never links to host code.
- The harness owns coordination, explicit instance context, experiments, and artifacts.
- The accepted sixth target, sts2-protocol, owns only genuinely shared language- and
  transport-neutral contracts after its own decision and conformance work.

Do not move a concern across these boundaries for convenience. An ownership or dependency change
requires a decision record under docs/decisions.

## Safety and provenance

Never add proprietary host assemblies, game binaries, game assets, saves, credentials, private
profiles, personal paths, generated build output, or copied implementation source. The local host
installation may be used only by an explicitly authorized compatibility test outside this tree.
Do not use the interop experiment as permission to install anything into a valued profile.

Claims must be labeled as confirmed, source-derived, proposed, inferred, or unverified. A compile
or generic ABI result is not proof that the game discovers, loads, or safely runs the mod.

Preserve unrelated files. Do not initialize Git, stage broadly, commit, push, deploy, publish,
install, launch a game, contact a provider, or modify a sibling target from this directory.

## Editing and validation

Read the applicable docs before changing a boundary or public contract. Use apply_patch for
repository edits. Keep handwritten files within policy budgets and split files by responsibility.
Do not hide failures with unconditional success, blanket retries, or ignored results.

The required local entrypoint is:

~~~text
cargo run --locked --offline --package repo-policy -- --strict
~~~

Run the applicable Rust gates as well:

~~~text
cargo fmt --all --check
cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings
cargo test --locked --offline --workspace --all-targets --all-features
~~~

The managed loader project needs an operator-supplied exact host assembly and is not a general
local prerequisite. The source-only managed probe can be built without that assembly. Record every
skipped host/runtime check and never convert it into a pass.

## Documentation

Update architecture, compatibility, security, testing, licensing, workflow, and release documents
when their subject changes. Public routes, ABI symbols, host callbacks, queue behavior, and package
contents need a project-owned requirement and a deterministic test before implementation.

The final handoff must state the target, changed paths, exact command results, unverified limits,
and the next owner action. Do not claim a merge, release, deployment, or runtime compatibility
without its separate evidence.
