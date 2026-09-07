# Changelog

All notable changes to this target are recorded here. The repository has no released product
behavior; the dated runtime records below remain scoped to their named STS2 v0.107.1 fixtures and
do not establish release support.

## Unreleased

- Added explicit Windows/Linux runtime payload selection, platform-specific Workshop allowlists,
  managed native-library validation, and a checksum-gated install/update/rollback tool with
  synthetic lifecycle coverage. Workshop publication and exact-host runtime evidence remain
  separately gated.

- Added a guarded Linux x86-64 Workshop lifecycle operator and empty-item `ISteamUGC::CreateItem`
  helper with exact package/VDF checks, private durable journals, fail-closed unknown outcomes,
  synthetic Steam process/native-library coverage, and a pinned public callback ABI proof. The
  helper does not embed Steam credentials or SDK files; real Steam publication, subscription,
  download, and host-runtime evidence remain unverified.

- Added deterministic source-only Windows/Linux release bundle preparation with exact Git source
  and tree identities, embedded checksum inventory, fixed archive timestamps, proprietary-file
  refusal, and reproducibility self-tests. Native addon installation and Workshop lifecycle remain
  separately gated by exact host and publisher evidence.

- Record bounded native v0.107.1 Windows/Linux runtime-v3 campaigns and fresh replays through the
  host adapter, plus forced terminal Victory observation with disabled input and no legal catalog.
  The campaigns reached Defeat; the forced fixture is observation-only and does not establish a
  model-played Victory, native multiplayer, or broader host compatibility. See
  `docs/evidence/native-victory-observation-20260906.md` and the harness campaign records.

- Translate merchant purchases, card removal and shop exit through native controls;
  verified Windows Astra relic purchase and shop exit. Reject potion reward claims when
  native potion storage is full. See native shop evidence for unverified paths.

- Expose native treasure chest and relic choices through reward actions, with verified
  Windows Astra chest opening, Gorget acquisition and map continuation.

- Translate rest-site healing, single-card smithing and proceed through native controls;
  confirmed bounded Windows Astra healing, Bash upgrade and map continuation.

- Consume the coordinated Runtime-v3 continuation schema with argument-free proceed,
  confirm-selection and cancel-selection actions; reject mixed revisions and extra arguments.

- Read provider identity before live-combat launch, support the OpenAI Astra bridge, and
  record the selected provider/model instead of hardcoding Ollama.

- Add opt-in visible single-player combat demonstration with host action-completion witnesses,
  local-only demo saves, preserved mod consent, and prelaunch display, size and window mode.
  See `docs/LIVE_COMBAT_DEMO.md` for exact runtime evidence and isolation limits.

- Added opt-in same-seed practice replay for an active saved, single-player Custom run
  (PR #16), with explicit confirmation that current progress is discarded. Source-linked
  synthetic controller checks do not establish exact-host save lifecycle, UI confirmation,
  replacement-run behavior, or history effects; cleanup and restart have no rollback.

- Added the missing Runtime-v2 artifact checksum gate to CI alongside POC, Runtime-v1,
  and Runtime-v3 verification. Frozen artifact bytes remain unchanged.

- Integrated the semantic gameplay callback as kind 6 alongside frozen v2 IDs 3–5. Both
  profiles share host identity and pending-operation admission fences; source-linked regressions
  cover delayed dispatch, retries and reconciliation without promoting host compatibility.

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
- Split the managed Runtime-v3 handler by responsibility and removed its managed
  file-budget exemptions; source-linked probe coverage remains unchanged.

- Added the source-only Runtime-v3 fair-play host bridge, generation-bound typed legal-action
  catalog, postcondition receipt path, fail-closed unknown outcomes, and additive co-op projection.
  That source-only addition did not establish licensed host assembly/build or live gameplay
  compatibility; the separate dated native records cover only their named scenarios and artifacts.

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
  fencing. For this Runtime-v2 fake seam, no concrete host gameplay API was included; its live
  host mutation and settlement remain unverified. This historical entry does not describe the
  separate managed/native Runtime-v3 host path, and Runtime-v1 routes/tests are unchanged.

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
