# Changelog

All notable changes to this target are recorded here. The repository has no released product
behavior; the dated runtime records below remain scoped to their named STS2 v0.107.1 fixtures and
do not establish release support.

Completed entries that no longer fit this file's preferred size budget are preserved verbatim in
[`docs/CHANGELOG-ARCHIVE.md`](docs/CHANGELOG-ARCHIVE.md).

## Unreleased

- Added a Rust-only static game-facts adapter for accepted Protocol profile game-facts-reference-v1.
  It preserves ordered caller-supplied metadata labeled SourceDerived and refuses live, Confirmed, unknown-to-subset, and
  unbound requests without values. Manifest comparisons prove consistency only; caller evidence,
  a production owner inventory, exact-host support, and consumer adoption remain unverified. See ADR 0077.

- Added an internal profile-progress readiness observer for the pinned v0.107.1 host lifecycle.
  It remains unavailable until an observed ProcessFrame owner thread and a verified lifecycle load
  agree on the same profile/progress generation. The source-linked synthetic probe is not native
  readiness or seeded-run acceptance evidence. See ADR 0076.

- Named the raw-wire admission in the live-session wrapper. Every shape it launches runs on a local
  provider bridge, but it left `STS2_EXO_ADMISSION` unset, so the harness was fail-closed to the
  reviewed envelope and refused the run during settings assembly with
  `STS2_EXO_PACKAGE_DIGEST is required` — before the bridge, gateway, MCP or game were touched. That
  envelope demands a complete inspected deployment identity only the packaged Exo executor
  supplies, so it was never this wrapper's contract. It now sets `STS2_EXO_ADMISSION=legacy` (harness
  ADR 0031): an explicit acknowledgement that the bridge is not envelope-admitted, not an admission.
  No placeholder digests are declared, since enveloped admission compares declarations against
  inspected bytes. The Astra lane's live-episode capability never depended on the envelope and is
  unchanged. Verified against locally built harness `ab7f121`. No game, host or provider run is
  claimed. Refs #260, Refs #193.

- Fixed the `runtime-v3-gameplay` text bound to count Unicode characters, as the schema's
  `maxLength` does, instead of UTF-8 bytes. `valid_text` compared `str::len()` against the 512 limit,
  so a 512-character non-ASCII name the schema admits was refused. The single byte-named constant is
  now two accurate ones: `RUNTIME_V3_GAMEPLAY_MAX_TEXT_CHARACTERS` for text and
  `RUNTIME_V3_GAMEPLAY_MAX_IDENTITY_BYTES` for the ASCII-only identity pattern, which stays a byte
  bound. Empty and control-character rejection is unchanged, as are the offered-attribute rules, the
  schema, its digest and every pin. `RUNTIME_V3_GAMEPLAY_MAX_TEXT_BYTES` remains as a deprecated alias of the legacy value 512 so
  callers still compile; nothing validates by it. Refs #256.

- Stopped offering `skip_reward` on the card-reward selection screen. The mod never added that
  alternative; it only recognized the host's own, whose callback is
  `PostAlternateCardRewardAction.EndSelectionAndDoNotCompleteReward` -- it ends the selection and
  explicitly does *not* complete the reward, so the same `reward:5:CardReward` was re-offered
  indefinitely and the skip path never asserted consumption. The host offers no consuming
  alternative there, and the only way to complete a card reward is to take a card, which is
  already offered as `select_card` and does consume it -- so the screen stays actionable and the
  livelock is removed without a stall. `skip_reward` is also refused in
  `LegalActionReference.Validate`, leaving it with no producer and no executor. The frozen
  protocol still decodes the `skip_reward` wire arm; that artifact is owned by `sts2-protocol` and
  its checksums are deliberately unchanged. Source-only: what the game does when a human presses
  Skip remains unverified. See ADR 0075 (#171).
- Completed the Runtime-v3 player observation producer for the already-delivered protocol PR #80
  fields: held-card descriptions, actual host-owned relics and potions, potion descriptions and
  target/usability values when available, and belt/max capacity. Host-unavailable values remain
  absent while observed-empty inventories remain empty. Updated the Rust contract mirror, managed
  codec, and action consumer for the shipped potion action shapes without changing the artifact or
  digest. Regression coverage runs the real projection source against synthetic host models and
  serializes it through the managed codec; reward-entry contents and fail-closed `skip_reward`
behavior remain covered. The full managed loader builds with zero warnings/errors against the
installed STS2 v0.107.1 host after a separately signed prerequisite repair; no game launch or
native acceptance is claimed. The added potion action cases are consumer payload compatibility only; Runtime-v3 does not offer or dispatch them. See `docs/COMPATIBILITY.md` and `docs/TESTING.md`. Refs #171.
- Fixed the `continue_run` offer gate, which refused every real setup catalog: it required *exactly
  one* `start_run`, but a setup screen offers one per playable character, so the gate returned
  `StartRunNotOffered` and silently restored the `sts2-game-mod#172` symptom it was written to fix.
  The condition is now "a `start_run` is present", which still refuses a catalog that is not a
  new-run catalog, and compatibility still comes only from the native owner. The two decision
  records also disagreed with the artifact -- both claimed the schema digest was unchanged, but
  `73979e6` moved it to `daa21690…` -- so ADR 0073 is now the single normative record and ADR 0072
  holds only that provenance (#172).
- Named the rejected header in the native interop listener's `unsupported_header` refusal, and
  admitted the negotiation headers standard clients send unprompted. The closed 12-name
  allow-list at `experiments/managed-rust-interop/native/src/runtime_http.rs:106` refused
  `Accept-Encoding`, which Python's `urllib` adds automatically; `obs-vm-setup/capture/README.md`
  records the cost -- a working API returned 400, and that probe concluded "neither result
  establishes absence of the API" (#239). The body gains a `rejected_header` sibling field and
  never carries a value. The allow-list **widened** by `accept`, `accept-encoding`, and
  `idempotency-key`, so strictly fewer requests are refused and an unlisted header is still
  refused. Header names are not constrained to the RFC 7230 token charset here, so the body is
  serialized through `serde_json` and a client-supplied name cannot inject a field. See
  `docs/COMPATIBILITY.md`; the companion gateway change is sts2-gateway#113.
- Made `sts2-game-mod` compile for the Windows target and added the leg that keeps it that way.
  `same_state` compared `volume_serial_number`/`file_index`, which are nightly-only, and two
  `.permissions().mode()` executable checks used the unix-gated `PermissionsExt` unguarded, so
  `cargo check --target x86_64-pc-windows-gnu` failed while no CI leg built Rust for Windows at all
  (#224). The Windows arm now uses the stable size-plus-modification identity the other non-Unix
  hosts already use, the executable checks are unix-gated, the unix-only `resolve_dotnet` symlink
  test is unix-gated, and a new `windows-rust.yml` job cross-checks
  `cargo check --workspace --all-targets --target x86_64-pc-windows-gnu` under `RUSTFLAGS: -D warnings`,
  because the host Clippy gate cannot see a warning on a `target_os = "linux"`-gated item and the
  leg otherwise forgave every warning on the surface it compiled (#226). Closes #224. Refs #226.

- Tightened the `Check documentation links` step to deny `rustdoc::private_intra_doc_links` and
  `rustdoc::redundant_explicit_links` alongside `broken_intra_doc_links`. Both are warn-by-default,
  so the step exited 0 and reported success while a link from a public item to a private one printed
  a warning that resolves only because the gate always passes `--document-private-items` — the class
  `#220` named and `#221` left open. No link is broken at `86cf39b1f`; this is a latent gate gap, so
  no source file changed and the gate is proven binding rather than repaired. The tightened command
  exits 0 with zero warnings on the unmodified tree, and exits 101 at `lib.rs:260` when a link to the
  private `DispatcherPort` is added, where the shipped single-lint command exits 0 with
  `generated 1 warning`. Documentation and gate only; no code, route, schema or native effect. Refs #222.
- Closed a data-redaction gap in the seeded-run request surface for sts2-game-mod#79. The pinned
  `seeded-run-v1` `identity` pattern `^[A-Za-z0-9_.:/-]{1,128}$` is needed by the release-like
  compatibility identities and the composite `context_id`, but those characters also spell a POSIX
  or Windows host path, so `instance_id = "/home/operator/sts2/profiles/slot1"` validated and
  reached the request body and the `x-sts2-instance-id` header. Rather than narrow the shared
  alphabet (`sts2-protocol`-owned, mirrored into four consumers), the grammar splits by role: an
  unchanged artifact class for release-like `name/version` identities, and a stricter opaque class
  for operator-supplied runtime identifiers and profile-baseline identity, refusing leading
  separators, a bare drive prefix, parent-directory hops and URI schemes. `:` and `/` stay
  admissible so the co-op producer's `instance:native-test` keeps working. The C# loader gate
  mirrors the Rust client and the opaque class is a strict subset of the artifact class, pinned by a
  managed `SeededRunIdentityRedactionProbe` and a client test pairing every refusal with an accepted
  control. Producer-side only, with no schema, digest or pin change and no native admission claimed.
  Refs #79.
- Made the #79 drive-prefix refusal load-bearing instead of decorative. The client test spelled its
  Windows drive path with backslashes, which the shared identity alphabet already refuses, so
  deleting the `has_drive_letter_prefix` rule entirely left the whole suite green — the PR's central
  security property was untested against regression. The case is now spelled `C:/Users/...`, the one
  shape the drive-letter rule alone can refuse; removing that rule now fails the suite. Also pinned
  the bare `.` and `./` as admitted controls by decision, on both sides, so the choice cannot be
  silently reversed. No behaviour, schema, digest or pin change. Refs #79.
- Repaired three defects the #79 redaction work introduced, all found by its own managed gate rather
  than by inspection. `SeededRunIdentityRedactionProbe.csproj` had a malformed
  `IntermediateOutputPath` (a start tag with no end tag), so the probe could not load at all;
  `SeededRunContextProbe.csproj` omitted `EnableDefaultCompileItems=false` unlike every sibling
  probe project, so it glob-absorbed the whole `seeded-run-tests/` directory and absorbed its
  siblings' private `Main` methods (CS0017) and boundary types (CS0103/CS0246) — the collision was
  latent until this branch added a file to that directory, and pinning the compile list restores the
  invariant that a probe's sources are declared by its own csproj. `IsOpaqueIdentity` also had a
  CS8602 null dereference, because `IsIdentity` is null-tolerant but the compiler cannot see through
  that, and it admitted a bare leading `:`, the drive or scheme separator with its prefix missing,
  which the method's own doc comment already said it refused. The opaque class stays a strict subset
  of the artifact class: `instance:native-test` and every accepted control still pass. The full
  managed probe suite, the client tests, clippy and the strict policy gate are green. Source-only;
  no schema, digest or pin change and no native admission claimed. Refs #79.
- Repaired the managed seeded-run context gate, which reported green while its own probe did not
  compile. `SeededRunContextProbe.csproj` was the one seeded-run project that still globbed its
  directory instead of listing sources, so every sibling probe file in it had to be excluded by
  hand; two probes added since then never were, so it compiled two entry points and could not build
  at all, and a new probe here breaks it again. The CI step joined this probe and the
  standard-admission probe with `;`, so the compile error was discarded and the second probe's exit
  code decided the step: the gate proved nothing about the context digest it is named for. The
  project now sets `EnableDefaultCompileItems=false` and names its own sources, like its six
  siblings, so adding a probe here cannot change what any other probe compiles, and the step joins
  with `&&` so a failure is not masked. Build and gate wiring only; no probe logic, contract or
  runtime effect. Refs #79.


- Repaired the game-mod crate's one dangling intra-doc link and gated the class durably. The
  `content_index` module linked a bare `[`ContentManifest`]`, but a bare link resolves only against
  the file's own scope, and this private module has no `use` statements at all, so the label resolved
  to nothing while the crate still built, linted and tested green. `ContentManifest` is real — the
  `pub struct` re-exported at the crate root — so this was a pure scope/path defect, fixed by
  qualifying the link to `crate::ContentManifest` (the pattern `sts2-harness#474` used for the same
  class). The durable half is a `cargo doc` step in the Rust job with
  `RUSTDOCFLAGS="-D rustdoc::broken_intra_doc_links --document-private-items"`, because a default
  rustdoc run skips this private module entirely; nothing had run `cargo doc`/`rustdoc` before, so no
  gate owned the class. Documentation and gate only; no code, route, schema or native effect. Refs #220.

- Made input availability gate exactness in the source-only `game_facts_reference` owner
  inventory for sts2-game-mod#207. `FactsInputAvailability::supports_exact_claim` and
  `FactsRuleEntry::inputs_support_exact_claim` now state that only a `required` input is fully
  stated, and `FactsInventory::is_exact_claim` consults them, so a host-confirmed rule carrying a
  conditional or unknown input no longer reads as an exact claim. Conditional and unknown status is
  still disclosed rather than dropped. The test cases moved to `_exactness`/`_validation` siblings
  with a shared `tests/support/` fixture to stay inside the size budget. Source-only; the extraction
  adapter and exact-host parity remain the follow-up. Refs #207.

- Added the source-only `game_facts_reference` owner inventory for sts2-game-mod#207, the proposed
  handoff that sts2-game-core#13 needs before any authoritative rules lookup. One inventory binds
  the exact build and mode with the content-manifest witness, the negotiated structured
  representation, and each supported rule's opaque id, evidence label, and typed inputs, where
  every input carries its unit and required/conditional/unknown availability. It is closed and
  fails closed: an empty rule set, a rule with no typed input, a repeated rule or input id, a
  non-opaque identity, and a combination naming an undeclared rule are refused, and a rule in a
  declared unsupported combination never reads as an exact claim even when host confirmed. No wire,
  adapter, or native parity is added; the game-owned extraction adapter and exact-host comparison
  remain the follow-up. See ADR 0074.

- Fixed the campaign-map launcher so it enables the harness map path. The map branch exported
  `STS2_CAMPAIGN_MAP_BOUND=true`, which no harness code reads, and never set the harness's real
  `STS2_ENABLE_MAP_CONTEXT` opt-in, so the map runner/exo path stayed disabled. The launcher now
  exports `STS2_ENABLE_MAP_CONTEXT=true`, with the matching `docs/LIVE_COMBAT_DEMO.md` correction.
  Source-only; native map extraction, provider delivery, and navigation remain unverified. Refs
  AI-Ascension/ascension-map-visualizer#1; #14.

- Added the host-offered `continue_run` producer admission beside `start_run` for
  sts2-game-mod#172: a compatible saved run could be detected by the native owner, but no mod action
  could continue it, so every episode abandoned the previous run. The setup-screen offer appends
  the harness-admitted `{"kind":"continue_run"}` / `{"kind":"continue_run","run_id":...}` shape only
  when the screen is `Setup`, exactly one `start_run` is offered, and the owner reported a
  compatible run; an absent or incompatible run adds nothing and an explicit `null` discriminator
  is refused rather than folded into an absent one. Dispatch reuses the existing admission/receipt
  lane unchanged and never substitutes `start_run`. Additive producer extension, no frozen-artifact
  or digest change; native resume evidence (T3) remains unverified.

- Corrected the shared native enemy-read failure path consumed by runtime-v3 and runtime-v4:
  required read failures, invalid health/identity values, duplicate identities and oversized
  collections now refuse rather than fabricate empty/dead observations or clamp health. Optional
  hidden/unavailable intents remain unknown. Added a production-linked managed regression probe
  and CI entry; native, cross-repository and gameplay acceptance remain unverified. Refs #84; #96.

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
