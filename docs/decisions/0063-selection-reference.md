# ADR 0063: Owner-local read-only selection and candidate reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-19
- Tracking: game-mod #103
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md),
  [ADR 0039](0039-field-availability-and-completeness-boundary.md),
  [ADR 0058](0058-settings-reference.md),
  [ADR 0062](0062-rest-site-reference.md)

## Context

Issue #103 requires what a supported selector asks the player to choose and which candidates it
presents, before anything is chosen: the selection identity and its parent operation, the localized
prompt, the exact-build kind, the required/minimum/maximum picks, the ordering and duplicate rules,
the confirmation and cancellation semantics, and the explicit selector generation; and, per
candidate, the definition it resolves to, its kind, resolved eligibility with a refusal reason, the
documented prospective effects, and its evidence and visibility labels. The same read model must
carry card, player, reward, shop-removal, upgrade, rest, and co-op selectors, while a selector type
this producer cannot name is accounted for by coverage metadata rather than left hard-coded unknown.

The boundary must stay read-only and must not become a second mutation API: reads must never emit a
click, confirm, cancel, or back, must never advance a prompt, and must never admit or continue a run.
Reconciling the picks a caller reports must describe the remaining count and the candidates that are
still selectable, so an early confirmation, a duplicate choice, and a stale selector are never
represented as legal.

This decision defines an owned source boundary. It does not claim a live selector read, a native
extractor, a transport route, a gateway/MCP adapter, exact-host comparison, or a new selection API.

## Decision

`crates/game-mod/src/selection_reference` owns an immutable `SelectionCatalog` produced by
`SelectionCatalogProducer` from a bounded `SelectionCatalogSource`. Static results bind to the
existing content-manifest cursor, the exact locale, and the owner-local producer version
`game-selection-reference-producer-v1` through `SelectionCatalogBinding`. Family coverage is explicit
(`SelectionFamilyState`), and a family the source cannot project is refused rather than reported as
an empty catalog. A non-clonable `SelectionReader` fences listing, candidate paging, and exact lookup
by locale, visibility scope, selector generation, and single-use continuations, and one reader never
accepts another reader's continuation.

### One read model for every supported selector family

`SelectionKind`, `SelectionParentOperation`, and `SelectionCandidateKind` name the supported card,
player, upgrade-target, relic, potion, reward-item, shop-removal, rest-option, and co-op families,
with owner-defined and unsupported tokens for prompts this producer cannot type. A selector whose
described candidate resolves to a known manifest family must not report itself `Unknown`
(`KindDisagreesWithDefinition`), so no supported prompt can stay hard-coded unknown, and a candidate
that reports a manifest family may not resolve to a different one
(`InvalidInput("candidate_kind")`).

### Candidates, eligibility, and documented prospective effects

`SelectionEligibility` keeps eligible and ineligible distinct: a refused candidate must state why,
an offered candidate must not carry a refusal, and a visible candidate may not report a withheld
reason (`InvalidEligibility`). `SelectionProspectiveEffect` keeps the documented before and after
values with their target and references, and an effect that would change nothing is refused as
`InvalidProspectiveEffect` rather than published as a change. A `SelectionSemanticReference` to an
absent identity is a `DanglingReference`, a reference that names no manifest family is `None` rather
than assumed, an identity absent from the content manifest is an `UnknownManifestReference`, and a
record more visible than its target is a `HiddenReferenceLeak` whose restricted identity is
deliberately omitted.

### Types the producer cannot name are covered, never silently omitted

Every candidate the host reports is either described by a typed record or named by a coverage
record: an accounted-for candidate is required (`UncoveredCandidate`), an untyped selector must also
name itself (`UncoveredSelection`), and a coverage record that names neither the selector nor one of
its reported candidates is refused (`InvalidCoverageRecord`). Duplicate candidate, coverage, and
definition identities are each refused by name, so a newly audited entry cannot be silently omitted
and an audit record cannot stand in for an unnamed target.

### Progress reconciliation, confirmation, and staleness

`SelectionProgressInput` reuses the exact selector-identity fence of the produced prompt. Reconciling
a caller's picks against one selector generation reports the picked count, the remaining count, the
candidates still selectable under the declared duplicate rule, and whether the prompt may be
confirmed; an early confirmation is refused with the exact shortfall (`PrematureConfirmation`), a
repeated pick under a no-repeat rule is refused (`DuplicateChoice`), a pick outside the presented
domain is refused (`UnknownCandidate`), more picks than the selector accepts are refused
(`ExcessPicks`), and a reference bound to another selector generation is refused
(`StaleSelectorReference`) instead of being answered with the current candidate domain. Candidate
pages repeat the same snapshot: a continuation is bound to the selector, the scope, and the exact
pick state it was issued for, so it cannot resume a different prompt or a changed choice.

### Multi-step selectors name their next domain

`SelectionNextDomain` names the next selection identity, its candidate kind, and its exact selector
generation, so a multi-step sequence is resolved by identity and generation instead of being
inferred. A next domain that names nothing is `DanglingReference`, one bound to another generation is
`StaleSelectorReference`, and a declaration that contradicts its own steps is
`InvalidStepDeclaration`. Advancing before the required picks are made is refused
(`IncompleteSelection`), and the last step of a sequence reports that it ends there.

### Read-only by construction

`SelectionCatalogSource::read_catalog` takes `&self`, and the trait declares no click, confirm,
cancel, back, selection, mutator, or admission method: nothing on this boundary can choose an
option, close a prompt, or admit a run, and the slice retains no transient host action it could
execute. A resolved `SelectionActionReference` is rejected as `InvalidSelectionAction` so a live
action can never enter the static slice. A definition that is hidden or owner-only is never returned
in a scope that may not observe it, and a withheld record is dropped rather than replaced with a
placeholder, so a collection whose every record is withheld reports `Denied` instead of an observed
empty collection. Regressions assert an exact source-read count, an unchanged retained candidate set
across list, candidate, get, and progress reads, refusal of a reused, foreign, or pick-state-bound
continuation, scope withholding without placeholders, and the uncovered-entry, kind-agreement,
eligibility, prospective-effect, pick-rule, confirmation, stale-selector, and next-domain
rejections; the accounted-candidate, duplicate-choice, selectable-candidate, stale-generation,
incomplete-step, withheld-eligibility, and coverage-target assertions were each verified to fail
under an isolated production mutation.
