# Changelog

All notable changes to this target are recorded here. The repository is in foundation preparation
and has no released product behavior.

## Unreleased

- Added a bounded loopback runtime listener, managed main-thread queue bridge, `runtime-v1` artifact
  copy, and the host-visible `show_runtime_probe` action with stale-generation handling.

- Confirmed the focused `runtime-v1` host probe in STS2 v0.107.1 on Windows x86-64, including the
  authenticated listener, main-thread dispatch, visible effect witness, and reversible cleanup.

- Added optional ModConfig settings for the managed addon: a diagnostic debug-overlay toggle, a
  one-shot full-profile-unlock-on-next-launch toggle, and a manual full-profile-unlock action when
  a compatible framework/button API is available; the framework remains optional and game-runtime
  behavior is unverified.
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
