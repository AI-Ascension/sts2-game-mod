# ADR 0067: Owner-local read-only progression reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-20
- Tracking: game-mod #108
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0041](0041-save-profile-selection-and-disposable-boundary.md), [ADR 0058](0058-settings-reference.md), [ADR 0066](0066-run-result-reference.md)

## Context

Progression is not reachable as owned data. An agent cannot read which unlocks and achievements a
profile holds, what gates one it does not hold, what it has not yet encountered, which compendium
entries it has discovered, how far each character has progressed, or which best records and
aggregate statistics the host reported.

Reading those values through the game's own menus means driving the game: a progression screen is
only reachable after a profile is selected and a save is loaded, and the act of browsing is
indistinguishable from the act of unlocking. Neither effect is recoverable by the reader.

The values are also not one kind of thing. An unlock, an achievement, a compendium discovery, a
character's progression, a best record, and an aggregate statistic are game progression. Account
and cloud data is not. A reference read that publishes them through one untyped shape reports
account-scoped data as game progression, and one that collapses "not held" and "not encountered"
reports a locked entry as an undiscovered one.

This decision defines an owned source boundary that copies the owner's progression records into an
immutable catalog that can be listed and read without selecting a profile, loading a save, or
changing game state. It does not claim a native extractor, a live progression read, a transport
route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/progression_reference` owns an immutable `ProgressionCatalog` produced by
`ProgressionCatalogProducer` from a bounded `ProgressionCatalogSnapshot` read through a read-only
`ProgressionReadPort`. The catalog binds the existing content-manifest cursor, the locale, the
profile identity with its existing freshness baseline, and an owner-local producer version, so a
catalog and every read it produced are fenced to one content revision, one locale, and one profile
revision. A snapshot whose manifest binding, locale, producer version, or baseline identity belongs
elsewhere is refused rather than read.

The profile identity, save slot, and freshness witness are the existing
`crate::save_profile::{UserDataIdentity, SaveSlotId, SaveProfileBaseline}` contract rather than a
second selector invented here, so this boundary never addresses a save path and never becomes a
second way to name a profile.

### A closed state vocabulary rather than one "not unlocked" value

`ProgressionReadState` keeps six states apart: `Unlocked`, `Locked`, `Undiscovered`, `NotTracked`,
`Unavailable`, and `Unclassified`. Collapsed into one value, a caller reports an untracked or
unclassified entry as a locked one and a locked entry as an undiscovered one. Three of the six
assert nothing about the entry, so they may carry neither a progress value nor a best value: a
source that states one is refused as `StateCarriesValue` rather than having the value published
beside a state that contradicts it.

A lock is never inferred from the state. `Locked` requires at least one stated requirement, so a
lock always names what gates it and `LockedWithoutRequirement` refuses one that does not; an
`Unlocked` entry may not carry a requirement the source reports as locked, refused as
`UnlockedWithUnsatisfiedRequirement`, because a requirement may be satisfied while the entry is not
held and an entry may be held while a sibling requirement is not.

### Progression domain and account scope

`ProgressionDomain` inventories seven domains, and `is_game_progression` separates the six game
domains from `AccountScoped`. A snapshot that declares the account-scoped domain `Projected` is
refused as `AccountScopedDomainClaimed`, an entry classified `AccountScoped` or placed in that
domain is refused as `AccountScopedEntryInProgression`, and an entry carrying an account or private
identifier beside game progression is refused as `AccountIdentifierInProgression` rather than
having the identifier copied into the payload.

Every domain states one `ProgressionDomainCoverage` row carrying its `ProgressionDomainState` —
`Projected`, `Unsupported`, or `Unavailable` — the entry count the source declares, and the fields
that domain does not project. A dropped, repeated, or miscounted row is refused as
`DomainCoverageIncomplete` or `DomainCountMismatch`, so a domain the source cannot serve is a stated
reason rather than an absent key read as an empty result, and declaring a field unsupported while an
entry reports it available is refused as `InconsistentField`.

### Field availability and content resolution

Progress, best value, requirements, content references, related identities, read state,
description, and title are each a closed `ProgressionField`, and every entry states exactly one
`ProgressionFieldAvailability` row per field. A missing row is refused as `MissingFieldCoverage`
and a repeated one as `DuplicateFieldCoverage`, because a field the source does not project must be
a declared coverage failure rather than a silently absent key. A row whose status disagrees with
the value the field actually carries is refused as `InconsistentField`. Values themselves are
`ProgressionFieldValue`, which carries status and value as one field, so a caller cannot read a
value without reading whether there is one.

Counts, elapsed times, and percentages are `ProgressionQuantity` with an explicit
`ProgressionUnit`. A percentage outside `0..=100` is refused as `UnboundedPercentage` rather than
clamped, because clamping would publish a number the host did not send. A collection stated
available but empty is refused as `EmptyPresentCollection`, since it then states nothing.

Content references and requirement definitions resolve against the content manifest: a family the
manifest does not handle is refused as `UnhandledManifestFamily`, a definition the manifest does not
carry as `UnknownManifestReference`, and a domain whose own manifest family is unhandled is refused
before its entries are read. Identities and localized text are bounded and validated, and one entry
exceeding its aggregate byte bound is refused as `EntryTooLarge` rather than truncated.

### Scoped listing, bounded pages, and single-use continuations

Listing is bounded by `PROGRESSION_MAX_PAGE_ITEMS` and filtered by domain, read state, and a
`ProgressionVisibilityScope` that is enforced on every read: a `Hidden` entry is observable in no
scope and an `OwnerOnly` entry never leaves an owner scope. Progression reads are fenced to the
snapshot's profile revision, so a query naming another revision is refused as
`ProfileRevisionMismatch` rather than answered from stale data, and naming another profile is
refused as `ImplicitProfileSwitch` rather than answered by switching the active profile. An offline
or foreign profile is only reachable through an explicitly supported port and is otherwise refused
as `ForeignProfileRequiresReadPort`.

A `ProgressionCatalogReader` is deliberately not clonable and its cursor registry is per-reader, so
a `ProgressionContinuation` is consumable exactly once and only by the reader that minted it; a
reused, foreign, or query-mismatched continuation is refused as `InvalidContinuation`. Because the
catalog's own `list` convenience serves a fresh reader, it refuses a result that does not fit one
page with `PartialPageRequiresReader` instead of returning a continuation no reader can consume,
and names `ProgressionCatalog::reader` as the paging entry point.

### Read-only by construction

`ProgressionReadPort::read_progression(&self, ...)` is the only production seam, the fail-closed
`UnavailableProgressionHost` is the default until an exact-host read is authorized, and the reader
surface offers `list` and `get` and nothing else: there is no unlock, no purchase, no save write, no
profile selection, and no requirement acknowledgement. `ProgressionReadAuthority` is exactly
`NotGranted`, so every published page and entry states the capability it withholds. A shared borrow
is enough to read everything.

## Evidence and limits

Synthetic fixtures cover one unlock, two achievements in the unlocked and locked states, an
undiscovered compendium entry, a best record, and an untracked statistic whose progress is
deliberately kept out of the result rather than reported as zero, over one manifest, plus a
profile-revision pair, a hidden and an owner-only entry, and an offline profile.

Rejection fixtures prove that a manifest, locale, producer, or baseline mismatch; a declared
coverage that omits or repeats a domain or disagrees with the observed count; an account-scoped
domain declared projected, an account-scoped entry, and an account identifier beside game
progression; an unhandled manifest family and an unresolved definition; a duplicate entry,
requirement, or field row; a missing field row and a row that disagrees with its carrier; a locked
entry without a requirement and an unlocked entry with a locked one; an undiscovered entry outside
the compendium; a state that asserts nothing carrying a progress or best value; a best value outside
the best-records domain; an unbounded percentage; an empty present collection; an out-of-bound
collection, field inventory, or entry size; a page size the catalog cannot serve; a reused, foreign,
or query-mismatched continuation; a filtered or read request for a domain the source does not
project; and a snapshot past the entry bound are each refused with a closed error.

Six suites hold 56 tests: the membership, identity, read-only, entry-eligibility, field-coverage,
and fence suites. Two of them were falsified by mutation: removing the missing-row rule fails
`every_field_row_is_required_and_a_gap_is_never_silent`, and removing the partial-page guard fails
`a_page_the_result_does_not_fit_into_requires_a_retained_reader`.

These prove deterministic local validation, scope enforcement, and read-only projection only.
Native progression extraction, live progression reads, profile selection, save loading, thread
affinity, shared transport/gateway/MCP delivery, and exact-host compatibility remain unverified.
