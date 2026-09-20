# Changelog

All notable changes to this target are recorded here. The repository has no released product
behavior; the dated runtime records below remain scoped to their named STS2 v0.107.1 fixtures and
do not establish release support.

## Unreleased

- Added the owner-local read-only action availability and preview reference slice for
  sts2-game-mod#104. The `action_reference` producer composes the existing content-manifest and
  locale witness with typed legal-action definitions carrying the parent operation, exact-build kind,
  resolved eligibility, bounded cost contributors, target restrictions, observed targets, and
  declared previews, and explains why a presented action is or is not available right now:
  `ActionAvailabilityExplanation` reports the host's own refusal code and reason text together with
  the identities of the blocking cost contributors and unsatisfied restrictions, so an unmet
  resource, an invalid or dead target, a full capacity, a disabled option, and a required selection
  each name the state that produces them, and a refusal token with no state behind it is refused as
  `InvalidRefusalSupport` rather than republished as a bare disabled label. `ActionPreviewQuery` and
  `ActionPreviewResult` separate the static frame from the live fence a caller may hold and report
  the target-specific consequences at one controlled start with an explicit classification, the
  omitted interactions, and the prerequisites a caller must still re-validate; an exact
  classification may not name an omission or assumption, a random chain reported as exact is refused
  by name, a preview approximated by applying and undoing a real action is refused as
  `SimulatedPreview`, and a consequence that would change nothing is refused rather than published.
  Every explanation and preview carries `ActionDispatchAuthority::NotGranted` and names a fresh
  legal-action catalog, epoch, and target validation, a frame from another legal-action generation is
  refused as `StaleActionReference`, a live, incomplete, or contradictory fence is refused as
  `MissingLiveFence` or `UnexpectedLiveFence`, a subject the catalog cannot resolve is
  `NoSupportedPreview` while an undescribed one is answered with an explicitly `Unavailable`-class
  result, and a hidden or owner-only subject is never disclosed at a scope that may not observe it.
  `ActionCatalogSource::read_catalog` takes `&self` and declares no dispatch method, and regressions
  assert an exact source-read count, byte-identical repeated explanations and previews, and an
  unchanged retained snapshot. Evidence is source-only and synthetic; a live frame read, native
  action-registry or consequence extraction, and exact-host comparison remain unverified. Refs #104.
- Added a source-only completed-run result and prior-summary reference for sts2-game-mod#107. A
  completed run was not reachable as owned data: reading its outcome, character and configuration,
  reached act and floor, duration, ending deck and inventory, displayed score, and reported
  statistics through the game's own post-run screen means driving the game, and the summary list is
  only reachable after a save is loaded. `crates/game-mod/src/run_result_reference` now copies those
  values into an immutable catalog fenced by the existing content-manifest cursor, the locale, and
  an owner-local producer version, with one closed field inventory whose every value states whether
  it is present, absent, unsupported, or withheld, so an unknown value is never published as a zero
  or an empty collection. The game's own score stays distinct from a harness evaluator score and a
  synthetic metric, a componentized score must reconcile with its displayed total, a terminal
  presentation that the host has not persisted as a finalized result is refused rather than read as
  a settled score, and a partial or unsettled result stays explicitly unavailable. Current results
  are read through a live fence, prior summaries are listed through single-use continuations bound
  to one query and one revision, a summary whose older detail the catalog does not carry stays
  unavailable instead of being reconstructed, a hidden or owner-only record is never returned
  outside a scope that may observe it, one run identity belongs to one profile, and the reader has
  no profile selection, no save load, and no run start, so a completed-run read cannot change the
  game. Evidence is source-only: 46 tests over synthetic fixtures assert score authority,
  reconciliation, scope enforcement, page and continuation bounds, and read-only production; no
  native extractor, transport route, or exact-host compatibility is claimed. Refs #107.

- Refused the demo shape on a local bridge for sts2-game-mod#193. The demo shape exported
  `STS2_LIVE_EPISODE=true` for every provider, and the harness restricts a live episode to the
  OpenAI Astra provider, so `--run-kind demo` with the ollama bridge started the guardian, gateway,
  MCP and harness and was only then rejected at harness preflight. The launcher now refuses the
  demo before any of them start, with a sentence that names the restriction and the provider kind
  it was given. A demo is not made to work on a local bridge: the harness makes the combat demo and
  the campaign episode mutually exclusive, so a local demo would have to name the combat demo
  alone, which also stops the replay capture `STS2_LIVE_EPISODE` enables. Whether that change is
  wanted is the open product question recorded in sts2-game-mod#193; this refusal is the safe
  half, and it cannot break a working path because the harness already refused this combination
  before any effect. Evidence is source-only: `live-combat-session.test.sh` asserts the refusal
  sentence, that the provider was still asked to describe itself, and that neither the guardian nor
  the gateway, MCP or harness started, and the case fails when the refusal is removed. Refs #193.

- Added a campaign shape that needs no OpenAI Astra provider for sts2-game-mod#179. Both bounded
  shapes set `STS2_LIVE_EPISODE=true`, campaign mode additionally demanded Astra, and the harness
  restricts a live episode to Astra, so a campaign run had no composed path through this
  repository's own launcher without that provider. A campaign run against the local ollama bridge
  now names the campaign episode (`STS2_CAMPAIGN_EPISODE=true`) instead, which the harness admits
  for a local bridge, so the no-paid-provider slice recorded for sts2-game-mod#79 is reachable here;
  an OpenAI Astra campaign still names the live episode. Each shape unsets the other flag, because
  an inherited export naming two modes is exactly what the harness refuses. The launcher also
  requires the pinned harness binary to carry `STS2_CAMPAIGN_EPISODE` and refuses the local
  campaign shape before starting anything when it does not, rather than starting a host, gateway
  and bridge for an episode the harness then rejects. The shape needs a reachable local ollama
  daemon, and the pinned `gemma4:31b-cloud` model is named as a cloud model, so whether a run
  answers from local weights or proxies to Ollama's cloud is unverified. Evidence is source-only:
  `live-combat-session.test.sh` asserts both shapes, the unset flag, and the pre-launch refusal,
  and its shape assertions fail when the campaign episode naming is removed. Native campaign
  execution remains unverified. Refs #179.

- Made a refused launch contract readable by the runtime consumer that needs it for sts2-game-mod#185.
  A refused contract and a lane that never declared one both answered `host_not_configured`, and the
  refusal threw before `StartRuntimeServer` ran, so a refused launch produced no listener at all and
  the reason the isolated-user-directory check already computed reached only `game.log`. A refused
  launch now starts the listener, skips the profile-touching bootstrap the refusal exists to prevent,
  and answers `launch_contract_refused_<reason token>` — for example
  `launch_contract_refused_isolated_user_dir_mismatch` — on the gameplay read and dispatch routes,
  while a lane that never declared a contract keeps `host_not_configured` and its compound dispatch
  code unchanged. Only the stable reason token crosses the wire; the diagnostic that names the
  compared directories stays in `game.log`. Evidence is source-only and executable in the new
  `SeededRunLaunchContractProbe` (managed-source job), which pins every code as a literal, holds the
  never-declared answers to their previous values, and proves no directory name reaches a response. The
  harness still discards the code when it reports `episode requires recovery before policy can
  continue`, so naming the reason in an episode failure remains a separate sts2-harness change. Native
  observation of a refused launch on Windows remains unverified. Refs #185.
- Added the owner-local read-only selection and candidate reference slice for sts2-game-mod#103. The
  `selection_reference` producer composes the existing content-manifest and locale witness with typed
  selection definitions carrying the parent operation, localized prompt, exact-build kind, the
  required/minimum/maximum pick rule, ordering and duplicate rules, confirmation and cancellation
  semantics, and an explicit selector generation, and typed candidates carrying the definition they
  resolve to, their kind, resolved eligibility with a refusal reason, documented prospective effects,
  and explicit evidence and visibility labels. Every candidate the host reports is either described
  by a typed record or named by a coverage record, so a newly audited entry is refused rather than
  silently omitted; an untyped selector must also name itself and each candidate it presents; a
  selector whose described candidate resolves to a known definition family may not report itself
  `Unknown`; a coverage record that names nothing observed is refused; and duplicate selection,
  candidate, and coverage identities are rejected. A documented prospective effect that would change
  nothing is refused rather than published, a refused candidate must state why while an offered one
  carries no refusal, and an impossible required/minimum/maximum rule, a self-closing prompt that
  also offers an explicit cancel, and a multi-step declaration that contradicts its own next domain
  are each rejected. Reconciling the picks a caller reports reuses the exact selector-identity fence
  and reports the remaining count and the candidates still selectable under the duplicate rule, so an
  early confirmation, a duplicate choice, an out-of-domain or excess pick, and a selector reference
  bound to another generation are never represented as legal, and a candidate-page continuation is
  bound to the selector, scope, and pick state it was issued for. The boundary is read-only by
  construction (`read_catalog(&self)`, no click, confirm, cancel, back, or advance method, no second
  mutation API), a transient selection action cannot enter the static slice, a hidden or owner-only
  definition is never returned outside a scope that may observe it, and a fully withheld candidate
  collection reports itself denied rather than observed empty. Source-only evidence; native selector
  extraction, live selection behavior, and exact-host compatibility remain unverified. Refs #103.
- Added the owner-local read-only rest-site and rest-option reference slice for
  sts2-game-mod#102. The `rest_site_reference` producer composes the existing content-manifest and
  locale witness with typed rest-site definitions carrying a localized name and description, and
  typed options carrying their reported kind, the definition they resolve to, resolved availability
  and refusal reason, requirements, costs, limits, effects, selection domain with its candidates,
  documented prospective comparison, and explicit evidence and visibility labels. Every option the
  host reports is either described by a typed record or named by a coverage record, so a newly
  audited button is refused rather than silently omitted; an option whose definition resolves to a
  known family may not report itself `Unknown`; and duplicate site, option, requirement, cost,
  limit, effect, candidate, and coverage identities are rejected. A documented healing formula keeps
  its base fraction, flat bonus, and named modifiers separate instead of publishing a folded total,
  a comparison derives its own completeness rather than claiming it, and an option-set reference
  from an earlier rest menu is refused rather than answered with current options. The boundary is
  read-only by construction (`read_catalog(&self)`, no rest, heal, smith, upgrade, transform, mend,
  or selection method), a transient rest action cannot enter the static slice, a hidden or
  owner-only definition is never returned outside a scope that may observe it, and a fully withheld
  collection reports itself denied rather than observed empty. Source-only evidence; native rest
  extraction, live rest or selection behavior, and exact-host compatibility remain unverified.
  Refs #102.
- Added the owner-local read-only shop inventory, service, and restock reference slice for
  sts2-game-mod#101. The `shop_reference` producer composes the existing content-manifest and locale
  witness with typed inventory entries carrying a definition reference, item kind, displayed price
  and currency, stock state, sale or stacked-discount state, purchase action, and capacity or
  eligibility restrictions; supported services with their exact-build identity, cost, selection
  domain, limits, eligibility, and prospective change; static pricing contributors and reference
  formulas; and restock rules with the inventory generation they produce. An entry whose reported
  kind contradicts the definition family it resolves to is rejected, so no supported item stays
  hard-coded `Unknown`; a documented formula stays a formula instead of being folded into an
  invented total; a blocked reason stays separate from the displayed price; and a stock reference
  from any other restock generation is refused rather than answered with current stock. The boundary
  is read-only by construction (`read_catalog(&self)`, no purchase, sale, restock, or gold-spending
  method), a transient purchase action cannot enter the static slice, reads fail closed, and
  duplicate entry, service, definition, and contributor identities, dangling or scope-leaking
  references, and malformed prices, discounts, stock, or service declarations are rejected.
  Source-only evidence; native shop extraction, live purchase or restock behavior, and exact-host
  compatibility remain unverified. Refs #101.
- Added the owner-local source-only reference text and public-screen text slice for
  sts2-game-mod#109. The `reference_text` producer copies the inventoried non-gameplay reference
  families (tutorial, help, lore, credits, and the public UI text a screen currently displays) and
  the semantic description of supported public screens into an immutable catalog fenced by the
  existing content-manifest cursor binding and an owner-local producer version. A family or screen
  kind the producer cannot project is reported as explicit unsupported scope with the value that
  could not be supplied, a locked document and a withheld private input keep an explicit reason
  instead of empty text, markup, non-Latin, and multiline text are preserved verbatim inside
  per-document and per-screen byte bounds, executable presentation and text shaped as an
  instruction to its consumer are refused rather than repaired, and a blocking tutorial or message
  is readable without clicking, confirming, or dismissing it. Listings are family-partitioned with
  bounded single-use continuations fenced to one family filter, one catalog revision, and one
  query, and the boundary is read-only by construction (`read_reference(&self)`, no setter).
  Source-only evidence; native reference-text extraction, live run reads, and exact-host
  compatibility remain unverified. Refs #109.

- Corrected the run configuration reference slice's seed-blind cache guarantee for
  sts2-game-mod#105. The fingerprint builder walked the closed field inventory and hashed the
  settled value of every kind it found — including `Seed` — so `seed_blind_cache` was seed-derived
  even though it is flagged `seed_blind`, and a seed-blind page summary still carried the seed-aware
  key. Two runs identical except for a visible seed therefore produced different `seed_blind_cache`
  fingerprints, which made the shipped "computed without seed material" and "a seed-blind scope
  never observes a seed or a seed-derived value" claims false. Known-seed material now enters the
  seed-aware key only through its own explicit part, and a seed-blind projection or page substitutes
  the seed-blind key for the seed-aware key. Caught by an independent agent review after the
  original merge; the fix ships with the regression that failed against the defect
  (`seed_blind_keys_and_projections_never_carry_seed_material`). Source-only evidence;
  native run-configuration extraction and exact-host compatibility remain unverified. Refs #105.

- Added the owner-local read-only run configuration reference slice for sts2-game-mod#105. The
  `run_configuration_reference` producer composes the existing content-manifest, locale, and profile
  witness with a closed field inventory of run configuration (run identity, mode, difficulty,
  mutability, provenance, sensitivity, visibility, seed policy, and modifiers) and keeps
  requested-versus-settled and fixed-versus-mutable values distinct, including explicit non-values
  for withheld, not-applicable, unsupported, unavailable, and not-observed fields. A live binding and
  configuration fingerprint bind every read to the manifest, locale, producer identity, revision,
  profile, and run identity; list pages and exact reads are bounded, single-use, and revision-bound,
  and a seed-blind scope never observes a seed or a seed-derived value. The boundary is read-only by
  construction (`read_catalog(&self)`, no setter), and unknown, duplicate, missing, unsupported,
  unavailable, malformed, or oversized input fails closed. Three regressions assert byte-identical
  repeated production, an exact source-read count, and unchanged retained definitions, each verified
  to fail under an isolated mutation. Source-only evidence; native run-configuration extraction and
  exact-host compatibility remain unverified. Refs #105.

- Added the owner-local read-only game settings reference slice for sts2-game-mod#110. The
  `settings_reference` producer composes the existing content-manifest and locale witness with the
  selected profile and a closed producer version, and exposes allowlisted setting identities,
  localized labels/descriptions, categories (language, accessibility, input, display, audio,
  gameplay interaction), levels (global, profile, addon), value types, stored/effective/default
  values with evidence and read seam, restart requirement, and declared ranges/options. Private and
  hidden settings must withhold every value and are observable only in the owner scope or in no
  scope, a run-affecting setting must agree with its `run_configuration` reference in both
  directions, and unknown, duplicate, or missing definitions, stale profile/locale, unsupported or
  unavailable families, and malformed or oversized input fail closed. The boundary is read-only by
  construction (`read_catalog(&self)`, no setter) and two regressions assert identical repeated
  results plus an exact source-read count, each verified to fail under an isolated mutation.
  Source-only evidence; native settings extraction and exact-host compatibility remain unverified.
  Refs #110.

- Added the owner-local `locale_reference` catalog so rendered game text can be read in any
  supported locale without switching the active game language (sts2-game-mod#111). One request now
  returns the requested locale, the effective locale that actually supplied the text, the exact
  fallback chain consulted, the text revision, the effective direction, and an explicit
  `Complete`/`Partial` completeness report. A definition's `(entity_kind, namespaced_id)` identity
  stays byte-identical in every language, so a localized answer never changes which entity it
  describes. An exhausted fallback chain fails closed with the closed `NotFound` error rather than
  an empty string or a zero, only the requested plural form then `Other` then `Unknown` is ever
  consulted (the served form is reported) and an unavailable placeholder value is carried as
  `LocaleUnavailableReason`, an unresolved placeholder stays visible as
  `UnresolvedPlaceholder(name)` together with the
  input it needs, and an effect amount is carried verbatim so localization can never silently change
  a number. The declared placeholder set and the set actually used must match; a fallback chain is
  rejected when it is empty, too deep, unordered, unsupported, or cyclic; owner-supplied text is
  validated as presentation, so ordinary markup, non-Latin, and right-to-left text are preserved
  exactly while control characters and script-scheme content are rejected; and a listing
  continuation is bound to one locale, one catalog revision, and one query. Reading is one-way:
  there is no setter, no locale switch, no profile or configuration mutation, and no live run read.
  Source-only evidence; native rendered-text extraction, live run reads, locale switching, and
  exact-host compatibility remain `unverified-native`. Refs #111.

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
