# ADR 0058: Owner-local read-only game settings reference data

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #110
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md),
  [ADR 0039](0039-field-availability-and-completeness-boundary.md),
  [ADR 0053](0053-reward-offer-reference.md)

## Context

The expert observation carries no general settings model: standalone addon settings and profile
management are not a game-settings read interface. An agent therefore cannot inspect the settings
that affect interaction or interpretation without changing the player's configuration. Issue #110
requires an owner-local inventory of game, profile, and addon setting families that exposes
allowlisted identities, localized labels and descriptions, value types, stored/effective/default
values where supported, valid ranges and options, and source/scope; that distinguishes a stored
preference from the effective runtime value and from restart-required settings; that withholds
private sentinels; and that returns revision-bound bounded sections without ever altering files,
resolution, locale, key bindings, or profile selection while answering.

This decision defines an owned source boundary. It does not claim a native extractor, a live read,
a transport route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/settings_reference` owns an immutable `SettingsCatalog` produced by
`SettingsCatalogProducer` from a bounded `SettingsCatalogSource`. Static results bind to the
existing content-manifest cursor, the exact locale, the selected profile, and the owner-local
producer version `game-settings-reference-producer-v1` through `SettingsCatalogBinding`. A
non-clonable `SettingsCatalogReader` fences listing and exact lookup by locale, category, level,
visibility scope, and single-use continuations.

### Read-only by construction

`SettingsCatalogSource::read_catalog` takes `&self`, and the trait declares no setter, selector,
resolver, or writer: no method on this boundary can store a preference, select a profile, change
resolution, or write an owner file. `SettingsCatalogReader` is deliberately not `Clone`, its cursor
registry is mutable, and each continuation is bound to one query and usable once. Repeated consumer
reads on one catalog return identical values, and repeated production from one source is
byte-identical; an isolated mutation that made a read alter stored content was detected by both
read-only regressions, so the claim is falsifiable rather than an argument from absence.

### Stored, effective, and restart state

One `SettingValueField` distinguishes the persisted preference from the runtime-effective value and
from the source default. Each field carries `SettingsEvidence` and the `SettingsReadSeam` it was
read through. `SettingsRestartState` states whether a change requires a restart. A value that is
supported but not observed stays an explicit `SettingsUnavailableReason` rather than becoming an
invented zero, empty string, or fabricated description.

### Per-profile versus global

`SettingsLevel` separates `Global`, `Profile`, and `Addon` (plus `Unknown`), and `SettingsProfile`
plus `SettingsProfileKind` identify the profile the values were read for. The catalog carries that
profile in its binding, so a reference produced for another profile is rejected as
`StaleReference`; the same fence rejects a stale locale.

### Value types, constraints, and closed options

`SettingsValueType` covers boolean, integer, enumeration, key-binding, and text values.
`SettingConstraint` declares an inclusive integer range with an optional positive step, a closed
option set, a maximum text length, or a named owner-owned pattern. Constraints, options, keys, and
references are bounded locally and a constraint that does not apply to the declared value type is
rejected. The stored *and* effective values are both validated against the declared constraints, so
a value outside its range, or an enumeration value outside the declared closed option set, fails
with a typed error instead of being projected.

### Withholding private and hidden settings

`SettingsVisibility` is `Visible`, `OwnerOnly`, `Hidden`, or `Unknown`; `SettingsSensitivity` is
`Public` or `Private`; `SettingsVisibilityScope` is `Public`, `Reference`, or `Owner`. A private
setting is observable only in the owner scope, an owner-only setting is observable from the
reference and owner scopes, and a hidden or unknown setting is observable in no scope. A private or
hidden setting must withhold every value field; supplying one is rejected with
`ValueMustBeWithheld` rather than sanitized. Credentials and private endpoints are therefore
excluded from public discovery by construction, and the withheld values are dropped rather than
redacted in place.

### Run-affecting links

`RunConfigurationLink` is `RunAffecting { configuration_id }`, `NotRunAffecting`, or
`Unavailable(reason)`. A run-affecting setting must carry a matching `RunConfiguration` semantic
reference into the manifest's `run_configuration` family, and a run-configuration reference without
a run-affecting link is rejected, so the link and its reference must agree in both directions.

### Coverage, membership, and provenance

`SettingsFamilyState` is `Handled`, `Unsupported`, or `Unavailable`. An unsupported or unavailable
family reports its state and projects nothing; it is never a successful empty page. A handled family
must inventory exactly the manifest's setting identities, so an unknown, duplicate, or missing
definition and a family-count disagreement are rejected before publication. Every projected record
keeps its availability, evidence label, and read seam, and unknown or unsupported data is never
converted into a successful partial result labelled complete.

### Versioned contract

The family identity `setting`, the run family identity `run_configuration`, and the producer version
are closed constants, and pages are bounded by `SETTINGS_MAX_PAGE_ITEMS`; aggregate definition bytes
are bounded by `SETTINGS_MAX_DEFINITION_BYTES`. This slice defines the game-owned vocabulary and
validation, and its names and shapes must still be reconciled with the accepted versioned
game-information contract in an owner ADR before any wire version is frozen.

## Evidence and limits

Synthetic fixtures cover stored-versus-effective and per-profile-versus-global values, restart
requirements, integer ranges, closed enumeration options, key-binding values, run-configuration
links, owner-only, private, and hidden withholding, bounded deterministic pagination with single-use
continuations, stale profile and locale rejection, unsupported and unavailable families, missing,
duplicate, and unknown definitions, every catalog fence, malformed identities and text, oversized
definitions, and source-failure mapping. The read-only acceptance criterion is covered by two
regressions that assert identical repeated results and an exact source-read count, each verified to
fail under an isolated mutation. These prove deterministic local validation and read-only
projection only. Native settings extraction, live reads, thread affinity, shared
transport/gateway/MCP delivery, and exact-host compatibility remain unverified.
