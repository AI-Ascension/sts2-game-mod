# ADR 0062: Owner-local read-only rest-site and rest-option reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #102
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md),
  [ADR 0039](0039-field-availability-and-completeness-boundary.md),
  [ADR 0058](0058-settings-reference.md),
  [ADR 0060](0060-run-configuration-reference.md)

## Context

Issue #102 requires what a rest site offers and what each option would do before it is chosen: the
localized name and description of every offered option, its requirements, costs, limits, and
effect/rule references; the current availability and refusal reason; resolved public parameters
such as the healing amount; target/selection requirements; the existing host-generated action
identity; and supported prospective before/after comparisons including upgrade choice detail. A
reported kind must not contradict the definition it resolves to, a documented healing formula must
stay distinct from the observed amount, and an option a producer cannot type must be accounted for
by name rather than silently omitted. The boundary must be read-only: nothing may rest, heal,
smith, upgrade, transform, mend, or select anything.

This decision defines an owned source boundary. It does not claim a live rest read, a native
extractor, a transport route, a gateway/MCP adapter, exact-host comparison, or a new rest API.

## Decision

`crates/game-mod/src/rest_site_reference` owns an immutable `RestSiteCatalog` produced by
`RestSiteCatalogProducer` from a bounded `RestSiteCatalogSource`. Static results bind to the
existing content-manifest cursor, the exact locale, and the owner-local producer version
`game-rest-site-reference-producer-v1` through `RestCatalogBinding`. Family coverage is explicit
(`RestFamilyState`), and a family the source cannot project is refused rather than reported as an
empty catalog. A non-clonable `RestSiteReader` fences listing and exact lookup by locale, visibility
scope, and single-use continuations, and one reader never accepts another reader's continuation.

### Reported kind versus resolved definition

`RestOptionKind` closes the supported option families at clone, cook, dig, hatch, heal, kindle, lift,
smith, and mend, with owner-defined and unsupported tokens for host buttons this producer cannot
type. An option whose definition resolves to a known manifest family must not report itself
`Unknown`: a mismatch is rejected as `KindDisagreesWithDefinition` with the reported kind and the
resolved family, so no supported option can stay hard-coded unknown. A reference that names no
manifest family is `None` rather than assumed, a reference to an absent identity is a
`DanglingReference`, and a record more visible than its target is a `HiddenReferenceLeak` whose
restricted identity is deliberately omitted. Option references resolve inside their own rest site
instead of the manifest.

### Audited options and explicit coverage

Every option the host reports is either described by a typed record or named by a coverage record.
An option the producer cannot type rejects the whole snapshot as `UncoveredOption` unless a
`RestCoverageRecord` names it, so a newly audited button is refused rather than silently omitted.
Duplicate site, option, requirement, cost, limit, effect, candidate, and coverage identities are
each refused by name.

### Availability, refusal reason, and action identity

`RestOptionState` keeps offered, disabled, not-offered, and unknown distinct, and an offered option
must not carry a refusal reason while a disabled option must explain itself (`InvalidAvailability`).
A transient `RestActionReference` is rejected as `InvalidInput("rest_action")` so a live action can
never enter the static slice; only the retained action-kind token is copied.

### Documented healing, selection, and comparisons

`RestHealAmount` separates the base fraction, the flat bonus, the named modifiers, and the observed
total: a reported total the named contributors do not justify is refused as `InvalidEffect`, so a
documented formula is never folded into an invented amount. `RestSelectionRequirement` names the
domain and its bounds, and candidate lists reuse the selector feature, including the co-op player
selector that resolves a player reference. `RestProspectiveComparison` derives its own completeness
from its own values, so a partial change is never labelled complete and a comparison claiming
completeness without a resolved change is `InvalidInput("comparison_complete")`.

### Option-set generations

`RestSiteDefinition` carries the option-set generation the host produced. A `RestOptionSetReference`
is fenced to that exact generation, so a rest-menu reference from an earlier option set is refused
as `StaleOptionSetReference` instead of being answered with the current options.

### Read-only by construction

`RestSiteCatalogSource::read_catalog` takes `&self`, and the trait declares no rest, heal, smith,
upgrade, transform, mend, selection, mutator, or admission method: nothing on this boundary can rest,
upgrade a card, mend a relic, select an option, or admit a run, and the slice retains no transient
host action it could execute. A definition that is hidden or owner-only is never returned in a
scope that may not observe it, and a withheld record is dropped rather than replaced with a
placeholder, so a collection whose every record is withheld reports `Denied` instead of an observed
empty collection. Regressions assert an exact source-read count, an unchanged retained option set
across list, get, and availability reads, refusal of a reused or foreign continuation, scope
withholding without placeholders, generation-fenced availability, and the uncovered-option,
kind-agreement, hidden-leak, heal-total, and comparison-completeness rejections; the uncovered
option, kind agreement, option-set generation, and heal-total assertions were each verified to fail
under an isolated production mutation.
