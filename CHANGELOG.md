# Changelog

All notable changes to this target are recorded here. The repository has no released product
behavior; the dated runtime records below remain scoped to their named STS2 v0.107.1 fixtures and
do not establish release support.

Completed entries that no longer fit this file's preferred size budget are preserved verbatim in
[`docs/CHANGELOG-ARCHIVE.md`](docs/CHANGELOG-ARCHIVE.md).

## Unreleased

- Added the owner-local semantic event and causal-provenance reference slice for sts2-game-mod#128.
  What happened in a run was not reachable as owned data, and the obvious substitute — recovering
  events by comparing two snapshots — is exactly what this slice refuses, because a difference
  cannot say who caused a change, whether two simultaneous changes were one event or two, or
  whether a change came from the gameplay the boundary was watching. The
  `semantic_event_reference` producer composes the existing content-manifest and producer witness
  with a closed inventory of fourteen authoritative kinds: card play, damage, block, heal, resource
  change, status and modifier application and removal, pile movement, room transition, choice,
  offer and purchase. Each record states its own coverage, so a dropped or unsupported interval is
  disclosed rather than closed by an invented event, a zeroed quantity, or a renumbered sequence; a
  gap is the same type as an event and keeps its sequence number, the capture window states where
  capture began, whether history predates it, and every span inside the captured range that is not
  fully captured, and a captured record inside a declared gap, a gap outside any declared interval,
  and a window that contradicts where capture began are each refused. A causal parent is either
  explicitly stated or explicitly absent and the two fields must agree, a kind that admits a cause
  must state one or its absence while a kind that admits none must omit it, a parent this history
  does not contain or that does not precede its child is refused rather than kept as unverified
  causality, a disclosed gap is not a stated cause, and an imported event never states a parent
  because its causality was settled when it was captured. Sequence order is monotonic and
  contiguous inside one run, branch, episode and epoch, and two positions from different scopes are
  not one order. Definition, live-instance, action and event identities stay distinct and opaque, a
  subject minted in the wrong namespace or aliasing the event or the run it belongs to is refused,
  and a content reference the manifest does not carry is refused rather than dropped.
  `SemanticEventSource::read_catalog` takes `&self` and declares no replay, rollback, re-run, or
  dispatch method, listing is bounded and fenced to one catalog revision, scope and page size
  through a single-use continuation, and `SemanticHistoryAuthority` is exactly `NotGranted`.
  Evidence is source-only and synthetic; native event capture, persistence, indexing, a query
  engine, and exact-host comparison remain unverified, and the harness-owned queryable run history
  stays blocked for full integration and acceptance. Refs #128.
- Added the owner-local co-op party and member-state reference slice for sts2-game-mod#106. The
  `coop_reference` producer composes the existing content-manifest and locale witness with one
  party and its members: character identity, health, character-specific resources, relics, potions,
  powers, special mechanics, the five pile kinds, readiness for the current public step, shared
  effects with their scopes, the scaling rules that state how an effect grows with the party, and
  the public phase, step, vote and targeting context an action is read against. Visibility is a
  property of the field rather than of the reader, so every row states whether it is present,
  absent, unsupported, withheld, not permitted, or stale instead of defaulting to zero or an empty
  collection, and a local-only potion stock or a local-only draw pile or hand is refused for an
  ally with its kind and visibility intact rather than emptied, so a reader sees that a hand exists
  and is not its own instead of an empty hand. A party declares exactly one local member, because
  the local-only fields belong to exactly one member and a second local member would let either
  read the other's; a value carried by a joining, disconnected or left member has no coherent
  snapshot and a value whose data lags is not published as current, so both are refused rather than
  shown with a qualifier a reader might not read; a vote taken in a phase that admits none, naming
  an option twice, or naming a member the party does not carry is refused, a vote publishes which
  members decided but never which option any of them picked, and a targeting relationship names two
  members the party declares and is refused when it names itself; a targeted effect must name a
  member the party declares while a party-wide or per-member effect must name none, and a scaling
  rule must match the shape of its own kind instead of publishing one number for a shared pool, a
  per-member increment and a target-amplified effect. A member's membership generation is unique
  within its party and a reference carries the generation it was minted for, so a reference minted
  before a rejoin is refused as stale rather than resolved against the rejoined member, and a live
  fence naming another observation epoch is refused rather than answered with this party's record.
  A party identity is opaque, kept disjoint from the instance and run it belongs to, and never the
  single-player save profile. `CoopCatalogSource::read_catalog` takes `&self` and declares no join,
  leave, invite, or dispatch method, member listing is bounded and fenced to one catalog revision,
  locale, party, scope and page size through a single-use continuation, and `CoopReadAuthority` is
  exactly `NotGranted`. An identity or text past its byte bound, a collection past its local bound,
  and a party past its aggregate retained-byte budget are each refused by name. Evidence is
  source-only and synthetic; a native party extraction, a live party read, party membership changes,
  and exact-host comparison remain unverified. Refs #106.
- Added a source-only asset handle, media, and rendition reference for sts2-game-mod#112. An asset
  was not reachable as owned data: reading which icons, art, and audio exist for a definition, what
  each states about its media, or the bytes of a rendition under the installed resource pipeline
  means resolving a resource path and decoding into engine-owned memory for a live instance, and
  neither effect is recoverable. `crates/game-mod/src/asset_reference` now copies those values into
  an immutable catalog fenced by the existing content-manifest cursor, the locale, and an
  owner-local producer version, with one closed field inventory whose every value states whether it
  is present, absent, unsupported, or withheld, so an unknown value is never published as a zero or
  an empty string. An asset is named by an opaque handle that refuses a filesystem path or a URL, a
  kind that disagrees with its properties is refused, a retrievable asset that states no media
  properties is refused, and a media type a host could interpret as executable presentation is
  refused, so a rendition is never markup or script. Rendition bytes live behind a private side
  table that only the reader's retrieve emits, a metadata-only descriptor carries none, and a
  rendition that exceeds the stored-byte, dimension, duration, decoded-byte, or decode-ratio bound
  is refused. A hidden or owner-only asset is never returned outside a scope that may observe it, a
  handle leased to a passed generation and a handle absent from this catalog are both refused, a
  page that does not cover every matching asset needs a retained reader and its continuation is
  single-use and bound to one query and one revision, and the reader has no path resolution, no
  decode, no install, no extraction, and no mutation, so an asset read cannot change the game.
  Evidence is source-only: 76 tests over synthetic fixtures assert handle opacity, media and
  coverage consistency, markup refusal, bounded retrieval, scope and generation enforcement, page
  and continuation bounds, and read-only production; no native extractor, transport route, or
  exact-host compatibility is claimed. Refs #112.

- Added a source-only progression reference for sts2-game-mod#108. A profile's progression was not
  reachable as owned data: reading which unlocks and achievements it holds, what gates one it does
  not hold, what it has not yet encountered, which compendium entries it discovered, how far each
  character has progressed, and which best records and statistics the host reported means driving
  the game, and a progression screen is only reachable after a profile is selected and a save is
  loaded. `crates/game-mod/src/progression_reference` now copies those records into an immutable
  catalog fenced by the existing content-manifest cursor, the locale, the existing save-profile
  identity and freshness witness, and an owner-local producer version. Unlocked, locked,
  undiscovered, not-tracked, unavailable, and unclassified stay six separate states, so an entry
  that asserts nothing can carry neither a progress nor a best value and a lock always names what
  gates it; a percentage outside its range is refused rather than clamped, an untracked value is
  kept out of the result instead of being reported as zero, and one entry states a row for every
  field in its closed inventory, so a field the source does not project is a declared coverage
  failure rather than a silently absent key. The six game domains stay distinct from account-scoped
  data, which is never declared projected or published as progression, and every domain states
  whether it is projected, unsupported, or unavailable together with the count the source declares,
  so a domain the source cannot serve is a stated reason rather than an empty result. References
  resolve against handled manifest families, reads are fenced to one profile revision and one
  locale, a hidden or owner-only entry is never returned outside a scope that may observe it, and
  the reader has no unlock, purchase, save-write, or profile-selection entry point. Also fixed a
  fail-open page trap: `ProgressionCatalog::list` served a fresh reader, so a partial page handed
  back a continuation no reader could consume; it now refuses a result that does not fit one page
  with `PartialPageRequiresReader` and names the retained reader as the paging entry point.
  Evidence is source-only: 56 tests over six suites assert state separation, field-row coverage,
  domain coverage, reference resolution, scope enforcement, page and continuation bounds, and
  read-only production, and two of the rules were falsified by mutation; no native extractor,
  transport route, or exact-host compatibility is claimed. Refs #108.

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
  unchanged retained snapshot. An identity or text past its byte bound, a preview collection past
  its local bound, more definitions than the local bound, and a definition past the aggregate
  retained-byte budget are each refused by name. Evidence is source-only and synthetic; a live frame
  read, native action-registry or consequence extraction, and exact-host comparison remain
  unverified. Refs #104.
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
