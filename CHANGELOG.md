# Changelog

All notable changes to this target are recorded here. The repository is in foundation preparation
and has no released product behavior.

## Unreleased

- Rebased the bounded Runtime-v3 card candidate after independent Runtime-v2 extraction in
  PR #26. Preserved v2 callback IDs, safety probes and launcher restoration. This unmerged
  alternative remains a proposal; the selected Exo lane supersedes it only after that lane merges.

- Completed the frozen Runtime-v1 artifact package with the canonical checksum inventory, golden
  messages, and referenced schema and conformance files from `sts2-protocol` commit
  `11e4252e39a77f0017b8e4f3720590e6162e8f53`. CI verifies the inventory; existing schema and
  manifest bytes are unchanged. This is packaging evidence only, not new host verification.
- Split the Runtime-v2 host adapter and guarded launcher restoration from PR #14, preserving
  frozen wire bytes and callback IDs 3–5. Bounded Runtime-v3 card routes remain outside this
  change. Removed raw provider-output logging; the source-only adapter retains unknown outcomes.

- Repaired host-candidate semantic retries, identity ownership, run freshness, and
  mutation-then-exception uncertainty. Removed unsupported state-delta settlement inference;
  independent host completion remains a blocker. Added source-linked managed regressions in CI.

- Enforced the ABI version 1 descriptor's zero-reserved-byte requirement. Invalid descriptors
  return `AbiError::NonzeroReservedBytes`; valid descriptor layout and version remain unchanged.
  Synthetic regression tests cover every reserved byte, without extending host evidence.

- Added a target-owned ephemeral runtime-session launcher and Windows environment bridge. Each
  launch creates distinct in-memory runtime/mod and gateway credentials with the OS CSPRNG, refuses
  an already-running game, verifies unauthenticated rejection plus authenticated game/gateway and
  harness readiness, and cleans up only its owned processes. Credentials are not placed in args,
  files, logs, or CI artifacts; the launcher uses no additional settings-framework mod.

- Added a deterministic first-party Steam Workshop package contract, manifest/checksum staging
  tool, managed pre-load validation, and synthetic Rust/managed fixture checks. Steam publication,
  subscription/download callbacks, and host-runtime Workshop evidence remain unverified.

- Added the owner-local Runtime-v2 release-like artifact copy pinned to schema digest
  `f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2` and a bounded deterministic
  in-memory fake seam for `end_turn`, receipt replay, reconciliation, cancellation, and timeout
  fencing. No concrete host gameplay API exists here; live host mutation and settlement remain
  unverified, and Runtime-v1 routes/tests are unchanged.

- Added a bounded runtime listener (default loopback), managed main-thread queue bridge, `runtime-v1`
  artifact copy, and the host-visible `show_runtime_probe` action with stale-generation handling.
- Added built-in AI-Ascension settings for listener enablement, bind address, and port, with
  status/authentication indicators, immediate Apply, and Reset controls; the bearer token
  remains environment-controlled.

- Confirmed the focused `runtime-v1` host probe in STS2 v0.107.1 on Windows x86-64, including the
  authenticated listener, main-thread dispatch, visible effect witness, and reversible cleanup.

- Added the real `AIAscensionSTS2GameMod` managed loader package and unique Rust companion. The package
  verifies ABI version 1, performs a bounded native smoke call, and logs a load marker in the exact
  installed game; gameplay, HTTP, and host mutation remain outside this load-smoke slice.
- Added an offline copy of the `sts2-protocol/poc-v1` release-like artifact and a deterministic
  game-mod mapping test for state read, accepted `use_budget`, rejected zero-unit action, and one
  settled-effect witness. This remains a fake core seam with no game-runtime claim.
- Added target-local repository governance, policy-as-code, immutable-action workflows, and
  boundary documentation.
- Initialized target-owned host, bounded main-thread queue, ABI gate, HTTP-adapter, and composition
  seams with deterministic fake tests; no game behavior or public route catalog was added.
- Preserved the managed/native interop experiment as a source-only, non-production boundary.
