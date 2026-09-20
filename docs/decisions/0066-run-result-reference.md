# ADR 0066: Owner-local completed-run result, score, and prior-summary reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-20
- Tracking: game-mod #107
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0041](0041-save-profile-selection-and-disposable-boundary.md), [ADR 0060](0060-run-configuration-reference.md)

## Context

A completed run is not reachable as owned data. An agent cannot read how the last run ended, which
character and configuration it used, how far it climbed, how long it took, what deck and inventory
it finished with, how the displayed score was composed, or which statistics the host reported.
Reading those values through the game's own post-run screen means driving the game: opening that
screen selects a profile, and a summary list read is only reachable after a save is loaded. Neither
effect is recoverable by the reader, and a partial run that has not yet been persisted is
indistinguishable from a completed one on that screen.

The organisation also evaluates runs outside the game, so a harness evaluator score and a synthetic
metric can be attached to the same run as the game's own score. A reference read that cannot tell
those three apart reports a number whose authority it does not know.

This decision defines an owned source boundary that copies the owner's completed-run results and
prior run summaries into an immutable catalog that can be listed and read without selecting a
profile, loading a save, or starting a run. It does not claim a native extractor, a live run read, a
transport route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/run_result_reference` owns an immutable `RunResultCatalog` produced by
`RunResultCatalogProducer` from a bounded `RunResultCatalogSnapshot` read through a read-only
`RunResultCatalogSource`. The catalog binds the existing content-manifest cursor, the locale, and an
owner-local producer version, so a catalog and every read it produced are fenced to one content
revision and one locale. A snapshot whose manifest binding, locale, or producer version belongs to
another revision is refused rather than read, and a declared family that reports no records
produces an explicitly empty catalog instead of an absent one.

### Result identity, position, and explicit field availability

One `RunResultInput` carries a stable opaque `result_id`, the owning `profile_id` and `run_id`, the
completion outcome, finalization, persistence, the character and configuration identities, the
reached act and floor, the duration with its clock kind, the ending deck and inventory, the
displayed score with its components and authority, the supported statistics, and typed trajectory
linkage.

Every value is a `RunResultFieldValue` carrying its own `RunResultFieldStatus`, so an unknown value
is never converted into a zero, an empty collection, or an invented description. `Absent` (the
owner has no value), `Unsupported` (the host cannot report one), and `Withheld` (the owner
deliberately keeps it) stay distinguishable, and each field is one closed `RunResultField` of the
documented inventory. Deck, inventory, statistic, and ending-entry identities resolve against the
content manifest, so an entry naming a definition the manifest does not carry is refused as
`UnknownManifestReference` instead of being published as opaque text.

### Score authority, absent modes, and unsettled results

`ScoreAuthority` separates the game's own `HostGameScore` from `HarnessEvaluatorScore` and
`SyntheticMetric`, and every component must agree with the authority it publishes. A record whose
origin is imported or harness-attached may not claim the host score at all. `ScoreMode` states
whether the score is componentized, total-only, or unsupported: a componentized record must
reconcile its components with the displayed total, an absent component collection in that mode is
refused as `UnreconciledScore`, and an authority that reports `NoScore` while naming a mode that
carries one is refused as `ScoreModeConflict`.

A terminal presentation is not a persisted result. `ResultFinalization::Finalized` requires
`RunResultPersistence::Confirmed`, a terminal outcome published as finalized but not persisted is
refused, and a run whose outcome is still `Unfinished` may not carry a total at all. The animation
that starts at the end of a run therefore cannot be read as a settled score.

### Scoped listing, bounded pages, and detail that stays unavailable

Current terminal results are read through one `RunResultReference` and a live fence that binds the
instance, run, and epoch, and an unfenced or stale fence is refused. Prior summaries are listed
through `RunSummaryListQuery` and paged through a single-use `RunSummaryContinuation`; a
continuation is bound to the catalog revision, the query, and the filter that minted it, so a page
walk cannot silently mix two queries or survive a revision change. Listing is bounded by
`RUN_RESULT_MAX_PAGE_ITEMS`.

Visibility is enforced on every read: a `Hidden` record is observable in no scope, an `OwnerOnly`
record never leaves an owner scope, and a listing read outside its scope omits the record rather
than returning it. One run identity belongs to one profile, so a second record claiming a run
another profile owns is refused as `ProfileIsolationViolation`.

A summary whose older detail the catalog does not carry reports `RunResultDetailState::Unavailable`
and is refused as `DetailUnavailable` rather than reconstructed from the summary, the score, or the
manifest. A summary naming a detail the catalog has not finalized is refused as
`PendingDetailPublished`, and a summary naming a detail it does not carry at all is refused as
`MissingResultDetail`.

### Read-only by construction

`RunResultCatalogSource::read_catalog(&self, ...)` is the only production seam and
`RunResultCatalog` exposes no setter. The reader surface offers `list`, `current`, `get`,
`get_summary`, and `detail` and nothing else: there is no profile selection, no save load, no run
start, no score write, and no acknowledgement method. `RunResultReadAuthority` is exactly
`NotGranted`, so every published page and record states the authority it withholds, and the type
makes the alternative unrepresentable rather than merely discouraged. A shared borrow is enough to
read everything, which is what makes the boundary read-only by construction.

## Evidence and limits

Synthetic fixtures cover a finalized victory with reconciled host-score components, a defeat in a
total-only mode, an abandonment reporting no score, a terminal outcome observed before the host
confirmed it persisted anything, and a withheld record, over one manifest, plus five prior
summaries with a retained detail, a detail the catalog does not carry, an owner-only summary, and a
hidden summary. Read-only fixtures prove one source read per production, that a failure leaves the
source and catalog untouched, unchanged retained records after every read, and that every read is
reachable through a shared borrow.

Rejection fixtures prove that a manifest, locale, or producer-version mismatch; a declared coverage
that does not match the records reported; a duplicate result, summary, run, statistic, deck entry,
or score component; an ending entry that does not resolve in the manifest; a score component whose
authority disagrees with its record, whose contributions do not reconcile with the displayed total,
or that is claimed while the family is unavailable; a path-shaped identity; a zero or oversized
entry count; an oversized record, summary, or aggregate; a stale, foreign, or unknown reference; a
page size the catalog cannot serve; a reused or query-mismatched continuation; and a live fence
presented to a history read are each refused with a closed error.

These prove deterministic local validation, scope enforcement, and read-only projection only.
Native completed-run extraction, live run reads, profile selection, save loading, run starts,
thread affinity, shared transport/gateway/MCP delivery, and exact-host compatibility remain
unverified.
