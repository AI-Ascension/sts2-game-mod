# Changelog

All notable changes to this target are recorded here. The repository has no released product
behavior; the dated runtime records below remain scoped to their named STS2 v0.107.1 fixtures and
do not establish release support.

## Unreleased

- Made the isolated-user-directory refusal actionable on Windows and enforced the launch contract
  before launch (sts2-game-mod#173). `LiveCombatDemo` refused with the single message "live demo
  requires its isolated user directory", which named neither the directory the game resolved nor
  which condition failed, so the failure could not be acted on. The decision now lives in the
  host-independent `IsolatedUserDirectoryCheck`, whose refusal names both directories and a stable
  reason token (`isolated_user_dir_unset`, `isolated_user_dir_unresolved`,
  `isolated_user_dir_mismatch`); `experiments/managed-rust-interop/live-combat-demo.ps1` resolves
  `override.cfg` plus the launch-scoped `APPDATA` root through the dot-sourceable
  `live-combat-demo-override.ps1` and refuses a mismatched declaration before the game starts. The
  source-only `SeededRunIsolatedUserDirProbe` (hosted `managed-source` job) and
  `live-combat-demo-override-tests.ps1` pin the reason tokens and diagnostics. Also corrects the
  #173 record: `AIAscensionSTS2GameMod.dll` does contain `STS2_LIVE_COMBAT`, `STS2_LIVE_USER_DIR`
  and the refusal string; the earlier scan missed them because a UTF-16 search started at an odd
  byte offset, so the recorded "none of those appear in this build" was a scan artifact, not a
  missing contract. Source-only evidence; the native seeded campaign remains `unverified-native`.
  Refs #173.

- Fixed the read-only Steam usability preflight so its classification describes the interactive
  session instead of the caller. `experiments/managed-rust-interop/steam-usability-preflight.ps1`
  read only `HKCU:`, so when the QEMU guest agent launched it in session 0 as a service account it
  emitted `absent_client` for an installation that was present and running for the logged-in
  operator; the `#79` native gate was recorded as blocked on that token. It now enumerates `HKCU:`
  plus every loaded interactive user hive (`S-1-5-21-*`) with unchanged tokens, single-key output
  and redaction rules. A/B through the guest agent on the same guest at the same instant:
  previous body `absent_client`, current body `process_present_account_indicated`, with one
  `Active` console session and `steam` running in session 1 observed independently. The
  `docs/evidence/seeded-run-criterion-evidence-map-20260917.md` runbook carries the dated
  correction. Preflight classification only; the native seeded campaign and its receipts remain
  `unverified-native`. Refs #79.

- Added the managed host-thread checkpoint capture seam and settlement classifier for #80 item 3.
  `CheckpointCaptureSource` (with `CheckpointCaptureSource.Settlement.cs` and
  `CheckpointCaptureSource.Rules.cs`) is host-thread-only, classifies settlement through the same
  `Quiescent / MidEffect / EnemyExecution / PendingSelectionTransition / Unknown` mapping as the Rust
  owner gate, copies the `checkpoint-payload-v1` families into owned records and immutable bytes only
  when the boundary is quiescent and unchanged across the window, and emits typed rejections (never a
  substituted value) for a required `unknown` family, a combat family at a settled map choice, an
  outstanding pending effect, an unbounded collection or a malformed identifier. It holds no host
  reference after returning, draws no RNG, advances no observation generation and is wired to no route
  or listener, so every boundary stays unavailable. The host-independent probe
  `experiments/managed-rust-interop/checkpoint-capture-tests/` runs in the hosted `managed-source` job
  and passes 92 checks, including that a quiescent capture is structurally identical to the pinned
  valid fixture and hashes to that vector's pinned canonical `blob_digest`. Synthetic evidence only;
  native field availability, ordering and restore semantics remain `runtime-unverified`. Refs #80.

- Added `docs/evidence/seeded-run-criterion-evidence-map-20260917.md`, mapping every #79 seeded-run
  acceptance criterion to its evidence at exact pins (mod `46b1ac6e`, host v0.107.1/`59260271`,
  protocol `bfe28e45`, harness/gateway/MCP heads, staged guest set); AC1 is `confirmed-source`,
  native rows remain `unverified-native` behind the interactive Windows/Steam session gate.

- Added the closed, versioned game-owned checkpoint payload contract `checkpoint-payload-v1`
  (`schemas/checkpoint-payload-v1.schema.json`, `ascension.checkpoint_payload.v1`) for the three
  first-release boundaries, a typed `CheckpointPayload` model that lowers to `CanonicalValue` and
  parses back strictly with typed errors, per-family `captured`/`unknown`/`not_applicable` coverage
  with required-unknown rejection, and pinned conformance fixtures with `SHA256SUMS`. Unsupported
  phases have no payload schema and keep the existing typed rejection; no phase is advertised as
  available. Synthetic source evidence only. See ADR 0057. Refs #80.

- Recorded the exact-build checkpoint coverage inventory from pinned host metadata. The opt-in
  `experiments/managed-rust-interop/checkpoint-coverage-reflection/` probe resolves every ADR 0037
  coverage family and every ADR 0040 RNG audit row to concrete host members (type, member, kind,
  declared type, visibility) on STS2 v0.107.1 / `59260271` (`sts2.dll` SHA-256 `a1f9e653…`)
  through metadata tables only, fails closed for any unmatched row, and never loads the assembly;
  `docs/evidence/checkpoint-coverage-inventory-20260917.{md,json}` and the `-rng.md` companion
  record the result. ADR 0037 and ADR 0040 carry dated amendments upgrading rows from
  `unverified` to `metadata-observed`; serialization, ordering, restore, and unknown-value semantics
  stay `runtime-unverified`, no phase is advertised, and hosted CI does not run the probe. Refs #80.

- Added the pinned `exact-restore-v1` game-mod consumer with strict frame/schema parsing, bounded
  closure staging, manifest and digest verification, Linux owner-private durable storage, and
  `COMMIT_INTENT` recovery semantics. The production fixed routes return typed unsupported errors
  before storage or managed dispatch because the local owner-fence provider and exact-host restore
  adapter are not available. Every existing operation also rechecks its stored full owner fence
  before any phase can disclose progress or touch staged data. Synthetic tests only; native
  restoration remains unsupported. See ADR 0042.

- Connected the canonical #83 `ContentManifestProducer` to the pinned
  `game-information-content-manifest-v1` codec and added the authenticated owner route
  `GET /api/v1/game-information/content-manifest`. Responses preserve `inventory_revision`, omit
  raw semantic and localized text, map source failures to closed protocol reasons, and refuse
  whole envelopes above 16 MiB. Native source extraction remains fail-closed with
  `missing_capability/source_unavailable`; exact-host registry coverage and live delivery are
  unverified. See ADR 0038.

- Completed the `protocol-artifact/exact-state-v1` consumer witness. `selected-vectors.json` now
  pins the complete `sts2-protocol` set instead of an 11-positive subset: 26 positives, 2
  equivalence pairs, and 8 distinctness pairs, including the profile guarantee that absent, null,
  empty, and explicit unknown values stay distinct. The artifact now carries a `SHA256SUMS`
  inventory so the existing CI discovery step verifies it like every sibling profile, and its
  manifest declares `checksums`. `tests/checkpoint_canonical.rs` asserts the coverage counts,
  re-derives every inventory digest from the checked-in bytes, and pins the four guarantee pairs;
  `tests/checkpoint.rs` asserts the manifest `checksums` key and the 26-vector coverage. Synthetic
  source evidence only; native capture, restore, and host compatibility remain unverified. Refs #80.

- Added a source-only `checkpoint::admission` controller that orders owner capture work against host
  mutation and binds one logical operation to one receipt: a settlement barrier refuses host
  mutation (`Busy`) between boundary validation and snapshotting, unsafe phases stay refused for
  every producer including the synthetic fixture, a duplicate operation replays its recorded
  receipt, changed bytes or a changed request under one operation is a conflict, a durable capture
  whose persistence fails records no receipt, and the ledger is bounded by
  `CHECKPOINT_ADMISSION_MAX_OPERATIONS`. Rejection variants `Busy`, `UnsupportedCoverage`,
  `PersistenceFailed`, `OperationConflict`, and `AdmissionLedgerFull` are now produced by owner
  code. The new owner checkpoint, ledger, and fixture-producer types render redacted `Debug` output
  instead of private payload bytes or exact digests. Synthetic source tests pass; native capture, restore, and host compatibility remain
  unverified. See ADR 0055.

- Added a source-only restricted `asc-jcs-state-v1` canonical encoder for game-owned checkpoint
  payloads: deterministic restricted-key ordering, exact `uint64`/`float64_bits` tagging, domain-separated
  state/blob identities pinned to protocol revision `8a2e66f5d2190a0fca7f146dc3508e8d55515ea7`, and a
  strict rejection matrix (duplicate keys, floats, exponents, negative zero, unsafe integers,
  invalid keys, trailing text). Synthetic conformance tests pass; native capture, restore, and
  host compatibility remain unverified. See ADR 0054.

- Corrected the source-only restricted canonical codec to the pinned profile's
  `CANONICAL_MAX_DEPTH` (64) nesting levels, including tagged numeric objects,
  and 16 MiB raw-input and canonical-output limits. Encoder writes check remaining bytes before
  allocation, including escaping expansion. Parser and encoder enforce `^[a-z][a-z0-9_]*$`
  object keys with typed rejections; all 14 pinned raw rejection vectors are included.
  Added exact-limit/overflow, typed integer, key grammar, surrogate, and hostile-depth regressions.
  Source-only; native capture, restore, and host compatibility remain unverified.

- Bounded the source-only restricted canonical codec at `CANONICAL_MAX_DEPTH` nesting levels
  in both the strict parser and the encoder; deeper inputs are rejected with
  `CanonicalError::DepthExceeded` instead of exhausting the process stack. Added parser regression
  coverage for arrays, nulls, booleans, escaped strings, Unicode values versus ASCII keys,
  safe-integer endpoints, escaped duplicate keys, and the depth boundary. Source-only; native
  capture, restore, and host compatibility remain unverified.

- Added the source/component `seeded-run-v1` native standard adapter with authenticated start and
  read-only reconciliation routes, selected-context and profile-baseline validation, canonical seed
  readback, and a `run_started` settlement witness. Its copied protocol artifact is schema digest
  `5c659f344be78f84e8d783986925d462714f933cac95d18943358992f7d3e2b8`, aligned with protocol main
  `d3ab5fca7d9d74bb31eeb3e5b343d8024ee44404`. Source/build evidence does not establish a live seeded
  run, save isolation, gameplay, or release compatibility; ADR 0031 remains proposed pending host
  verification.

- Added the source-only `runtime-map-v1` read profile with bounded player-visible topology,
  stable map-scoped identities, exact current legal bindings, generation fencing, and an
  authenticated `GET /api/map/v1/snapshot` route. Copied map artifacts, native route checks, and
  managed projection probes pass; live extraction, provider delivery, and navigation remain
  unverified. See ADR 0035.

- Added the source-only `runtime-v4-expert-rest-action-v1` candidate with authenticated rest-option
  and selector follow-up routes, typed Smith/Mend catalogs, option-specific completion witnesses,
  and same-operation unknown reconciliation. Its copied candidate artifact remains unadmitted;
  protocol consumers, live rest settlement, exact-host/package builds, and release compatibility
  remain unverified. See ADR 0036.

- Added locked Rust release-provenance tooling for runtime receipts, platform artifact manifests,
  Workshop staging, and source-distribution policy validation. The tooling and fixture gates pass;
  publication, installation, and host/runtime compatibility remain separately gated. See ADR 0034.

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

- Add the additive Runtime-v4 expert profile as a source/build candidate: native routes
  `GET /api/v4/runtime/expert-state`, `POST /api/v4/runtime/expert-action` and
  `GET /api/v4/runtime/expert-actions/{operation_id}` with ABI callback kinds 7 and 8, a
  managed player-visible expert projection with host-generated legal actions, and one admitted
  mutation (`use_potion`) fenced by lease, epoch, session, correlation, generation, state and
  operation identity. Pending v2, v3 and v4 mutations exclude one another. Copies of the
  `runtime-v4-expert` and `runtime-v4-expert-action` artifacts match merged protocol main
  `b3d3034f32e68d70c9e681f906ee37d74db153c4`. Runtime-v3 combat observations now project a visible
  single intent (ADR 0030)
  instead of `Unknown`, and `start_run` accepts any profile-unlocked character instead of only
  Ironclad. Source, build and synthetic-probe evidence only; live expert gameplay, potion
  settlement, exact-host builds and package builds remain `unverified`.

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
