# Changelog

All notable changes to this target are recorded here. The repository is in foundation preparation
and has no released product behavior.

## Unreleased

- Added a target-owned ephemeral runtime-session launcher and Windows environment bridge. Each
  launch creates distinct in-memory runtime/mod and gateway credentials with the OS CSPRNG, refuses
  an already-running game, verifies unauthenticated rejection plus authenticated game/gateway and
  harness readiness, and cleans up only its owned processes. Credentials are not placed in args,
  files, logs, or CI artifacts; the launcher uses no additional settings-framework mod.

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

- Added the real `AIAscensionSTS2Poc` managed loader package and unique Rust companion. The package
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
