# ADR 0060: Owner-local read-only run configuration reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #105
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md),
  [ADR 0039](0039-field-availability-and-completeness-boundary.md),
  [ADR 0058](0058-settings-reference.md)

## Context

Issue #105 requires the complete set of fields that affect an admitted run: the exact effective
values for standard and custom runs, each difficulty, and multiplayer configurations, together with
the provenance and configuration revision that produced them. A caller must be able to tell what it
requested from what the host settled, and a value the host may still change from one it fixed at run
start. Mode-specific optional fields, profile unlock rules, and modifiers the host cannot classify
must be represented explicitly rather than dropped. A visible seed must follow the existing seed
policy, RNG state must never appear in an ordinary configuration read, and rule or preview queries
must be bound to a configuration revision so a supported change invalidates dependent live queries.

This decision defines an owned source boundary. It does not claim a live read, a native extractor, a
transport route, a gateway/MCP adapter, exact-host comparison, or a new run-start API.

## Decision

`crates/game-mod/src/run_configuration_reference` owns an immutable `RunConfigurationCatalog`
produced by `RunConfigurationCatalogProducer` from a bounded `RunConfigurationCatalogSource`. Static
results bind to the existing content-manifest cursor, the exact locale, the admitted profile, and
the owner-local producer version `run-configuration-reference-producer-v1` through
`RunConfigurationBinding`. Each definition also carries a `RunConfigurationLiveBinding` with the run
identity, live instance identity, monotonic epoch, and configuration revision. A non-clonable
`RunConfigurationReader` fences listing and exact lookup by locale, revision, mode, visibility
scope, and single-use continuations.

### Requested versus settled, and fixed versus mutable

Every `RunFieldRecord` keeps the caller's `requested` setup and the host's `settled` value apart,
each as a `RunConfigurationField` that is either an available value or an explicit
`RunConfigurationUnavailableReason`. `RunProvenance` states where the settled value came from, and
`RunMutability` states whether the host may still change it. A modifier that claims to alter a field
whose value only echoes the request is rejected as `ModifiedFieldEchoesRequest`, so an altered setup
must be reported from a host-settled value.

### Closed field inventory and explicit non-values

`RunFieldKind` closes the inventory at mode, difficulty, character, loadout, act sequence,
modifiers, multiplayer scaling, active content, unlock rule, and seed. Mode, difficulty, character,
act sequence, and active content must always carry a settled host value; a missing one is a
`MissingRequiredField` and a `NotApplicable` outcome on such a kind is a
`NotApplicableRequiredField`. The remaining kinds are mode-specific or profile-specific and may
report `NotApplicable`, `Unsupported`, or `NotIntegrated` instead of a substituted default.
Modifiers the host cannot classify are retained with an explicit `RunModifierState::Unknown` and
are forbidden from claiming field effects the host did not verify.

### Caching, revision binding, and reuse

`RunConfigurationCacheKey` binds a preview or cache entry to the configuration revision and a
domain-separated fingerprint (`sts2.run-configuration.v1`) over the settled mode, difficulty,
character, loadout, act sequence, active modifiers, multiplayer scaling, active content, unlock
rules, and seed material when the policy makes the seed visible. Two runs that share a seed but
differ in difficulty, content, or modifiers therefore produce incompatible keys. Each definition
also carries a `seed_blind_cache` key computed without seed material and flagged `seed_blind`, so a
seed-blind entry can never be reused for a seed-aware query or the reverse. A definition reference
that names another revision, run identity, manifest, locale, or profile is rejected as
`StaleRevision`, `StaleRunIdentity`, or `StaleReference`.

A seed-blind projection or page substitutes the seed-blind key for the seed-aware key, so a
seed-blind scope observes no seed-derived value at all: seed material enters the seed-aware key only
through its own explicit part, never through the field walk.

### Seed policy and RNG state

The visible seed is exposed only under `RunSeedPolicy::Visible`; a withheld or unknown seed stays an
explicit non-value. A `RunVisibilityScope::SeedBlind` read refuses the seed field with
`SeedBlindScopeViolation` and drops the seed record from the projected definition. Identities in the
`rng_state` namespace are rejected with `RngStateNotPermitted`, so RNG state never becomes part of
an ordinary configuration read.

### Read-only by construction

`RunConfigurationCatalogSource::read_catalog` takes `&self`, and the trait declares no setter,
selector, mutator, or admission method: nothing on this boundary can change a mode, difficulty,
modifier, act sequence, content set, or profile, and nothing can start or resume a run. The slice
reuses the seeded-run profile and admission metadata instead of adding another run-start API.
`RunConfigurationReader` is deliberately not `Clone`, and each continuation is bound to one query
and usable once. Three regressions assert an exact source-read count, byte-identical repeated
production from one source, and unchanged retained definitions, each verified to fail under an
isolated mutation.

## Consequences

An owner-side consumer can inventory the exact build's run-affecting fields, observe settled values
with provenance and mutability, and fence preview and cache entries by configuration revision
without reading or writing game state. Because the boundary is source-only, native extraction, live
host change, integrated rule or preview queries, and exact-host comparison remain unverified.
