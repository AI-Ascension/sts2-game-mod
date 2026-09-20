# ADR 0065: Owner-local read-only action availability and preview reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-20
- Tracking: game-mod #104
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md),
  [ADR 0039](0039-field-availability-and-completeness-boundary.md),
  [ADR 0058](0058-settings-reference.md),
  [ADR 0063](0063-selection-reference.md)

## Context

Issue #104 requires an explanation of why a presented legal action is or is not available right now
and a target-specific preview of the consequences it would have, before anything is dispatched. An
unmet resource, an invalid or dead target, a bounded destination with no free capacity, an option
disabled by the current mode, and a selection the action still requires must each name the state that
produces them, so a refused action can never be published as a bare disabled label. A preview must
state deterministic target-specific consequences at one controlled start, report a random or
unsupported chain as explicit uncertainty, and never be mistaken for permission to act.

The boundary must stay read-only and must not become a second mutation API: reading an explanation or
a preview must never dispatch, play, use, buy, end, confirm, or advance an action, and must never
approximate a consequence by applying and undoing a real one. A preview asked for after a relevant
transition must be refused rather than answered with the newer frame, and a hidden outcome must not
become reconstructible by asking again.

This decision defines an owned source boundary. It does not claim a live frame read, a native
extractor, a transport route, a gateway/MCP adapter, exact-host comparison, or a new action API.

## Decision

`crates/game-mod/src/action_reference` owns an immutable `ActionCatalog` produced by
`ActionCatalogProducer` from a bounded `ActionCatalogSource`. Static results bind to the existing
content-manifest cursor, the exact locale, and the owner-local producer version
`game-action-preview-reference-producer-v1` through `ActionCatalogBinding`. Family coverage is
explicit (`ActionFamilyState`), and a family the source cannot project is refused rather than
reported as an empty catalog. A non-clonable `ActionReader` fences listing, target paging, exact
lookup, explanation, and preview by locale, visibility scope, legal-action generation, and
single-use continuations, and one reader never accepts another reader's continuation.

### One read model for availability and consequence

`ActionKind`, `ActionParentOperation`, and `ActionTargetKind` name the supported play-card,
use-potion, end-turn, rest, shop-purchase, event-choice, and relic families with owner-defined and
unsupported tokens for actions this producer cannot type. A legal action whose described target
resolves to a known manifest family must not report itself `Unknown`
(`KindDisagreesWithDefinition`), a target that reports a manifest family may not resolve to a
different one (`InvalidInput("target_kind")`), and a reference to an identity absent from the
manifest is an `UnknownManifestReference`.

### An availability explanation names the state behind the refusal

`ActionAvailabilityExplanation` carries the host's own refusal code and reason text together with
every bounded cost contributor and every target restriction the action declares, the identities of
the contributors and restrictions that currently block it, and `explains_refusal()`, which is true
only when a refused action states a reason and at least one blocking contributor or unsatisfied
restriction produces it. A refusal token with no state behind it is refused as
`InvalidRefusalSupport`, so the defect this slice exists to remove cannot be republished. A refused
subject must state a reason and its host text, an offered one must state neither, and a visible
subject may not report a withheld reason (`InvalidEligibility`, `InvalidTargetEligibility`). A
contributor never claims affordability it cannot settle, so an unobserved amount may not report
`affordable` (`InvalidCostContributor`), and a restriction may not contradict its own kind and
satisfaction declaration (`InvalidRestriction`).

### Target-specific consequences at one controlled start

`ActionPreviewQuery` separates the static frame the caller observed from the live fence it may or may
not hold, and `ActionPreviewResult` states the resolved target, the declared consequence set, the
classification that describes how confidently it is known, whether any part of the outcome is
withheld, and what the caller must still re-validate. `DeterministicExact` must state a consequence
and may not name an omission or assumption, `Unavailable` may not carry a consequence and names the
subject it could not describe, and every other class must publish the omissions and assumptions it
relies on. A random or unsupported chain reported as exact is refused by name
(`UncertainPreview`), a classification the definition does not declare is refused
(`UndeclaredPreviewClass`), and a consequence that would change nothing is refused as
`InvalidPreviewChange` rather than published as a change. A preview approximated by applying and
undoing a real action is refused as `SimulatedPreview`.

### A preview grants no dispatch authority and never answers a stale frame

`ActionAvailabilityExplanation` and `ActionPreviewResult` each carry
`ActionDispatchAuthority::NotGranted` and a `required_prerequisites` list naming a fresh
legal-action catalog, a fresh run epoch, and fresh target validation, so no explanation or preview
can be mistaken for permission to act. A frame bound to another legal-action generation is refused
as `StaleActionReference` instead of being answered with the current frame's consequences. A query
that supplies a live fence without the transient instance identity, an instance identity without a
live fence, or an incomplete fence is refused as `MissingLiveFence`, and a static query that
contradicts its own fence is refused as `UnexpectedLiveFence`.

### Read-only by construction

`ActionCatalogSource::read_catalog` takes `&self`, and the trait declares no dispatch, play, use,
buy, end, confirm, or advance method: nothing on this boundary can execute an action, and the slice
retains no transient host action it could execute. A resolved live action instance in a static
definition is rejected as `InvalidInstanceReference`. A subject the catalog cannot resolve at all is
`NoSupportedPreview`; a subject it resolves but the source does not describe is answered with an
explicitly `Unavailable`-class result, so an absent preview is never published as an empty
consequence set. A definition or preview that is hidden or owner-only is never returned in a scope
that may not observe it, a withheld record is dropped rather than replaced by a placeholder, and an
upstream withheld status is preserved rather than recomputed. The slice owns the `ACTION_` and
`ACTION_REFERENCE_` identifier prefixes; the merged `act_reference` slice keeps `ACT_` and
`ACT_REFERENCE_`, so neither boundary's constants can be confused for the other's.

## Evidence and limits

Synthetic fixtures cover one available card play with living, dead, invalid, and explicitly
unsupported targets, one potion refused for an unpayable charge with a partial preview that names its
random omission, one end turn refused for a full destination, one event choice refused until its
selection is answered, one disabled rest option whose cost was never observed, one owner-only relic
use, and one hidden shop purchase. Explanation fixtures prove that each of the six refusal reasons
names the blocking contributor or restriction that produces it. Preview fixtures prove the exact
target-specific change at one controlled start (`6` → `12`, delta `6`), the explicit uncertainty of a
random chain, the `Unavailable`-class answer for an undescribed subject, and the refusal of an
unresolvable one. Read-only fixtures prove an exact source-read count, byte-identical repeated
explanations and previews, an unchanged retained snapshot, and that a live, incomplete, stale, or
contradictory fence is refused. Rejection fixtures prove the eligibility, target-eligibility,
refusal-support, cost-contributor, restriction, instance-reference, undeclared-class, uncertain,
change, dangling-target, dangling-reference, hidden-reference-leak, cross-generation, uncovered
target, missing-family, family-count, sanitized-source, oversized-identity, oversized-text,
oversized-preview-collection, over-count-definitions, and over-budget-aggregate failures each
reject by name. The refusal-support, target-eligibility, cost-affordability, restriction, uncertain,
changed-nothing, undeclared-class, dangling-target, uncovered-target, cross-generation,
oversized-identity, oversized-text, oversized-preview-collection, over-count-definitions, and
over-budget-aggregate assertions were each verified to fail under an isolated production mutation
whose bytes were restored exactly.

These prove deterministic local validation and read-only projection only. Native action-registry
extraction, a live frame read, real host consequence computation, exact-host comparison, thread
affinity, and shared transport/gateway/MCP routes remain unverified, and no native or exact-host
result is claimed.
