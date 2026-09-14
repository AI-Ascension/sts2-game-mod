# ADR 0052: Owner-local event definition and branch reference data

- Status: Proposed; source-only owner boundary
- Date: 2026-09-14
- Tracking: game-mod #99
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0050](0050-act-encounter-reference.md)

## Context

The current event projection maps event state values to choices whose label is the raw value and
whose domain is null; there is no event narrative, option requirement, structured cost, or
outcome/branch reference. An agent cannot inspect an event's pages, eligibility, or offered choices
independently of the current run. This decision defines an owned source boundary. It does not claim
a native extractor, a transport route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/event_reference` owns an immutable `EventCatalog` produced by
`EventCatalogProducer` from a bounded `EventCatalogSource`. Static results bind to the existing
content-manifest cursor, the exact locale, and an owner-local producer version; an
`EventCatalogReader` fences listing and exact lookups by locale, visibility scope, and single-use
continuations.

Each event entry carries a localized title, a category, unlock and visibility state, bounded
localized narrative pages, event eligibility predicates, and a choice/branch graph of options with
stable option IDs. Each option carries localized text, typed requirements, structured costs, and
possible outcomes. Costs distinguish HP loss, max-HP change, gold, and card/potion/relic/item
removal from owner rules, and keep an owner rule reference and evidence label. Outcomes carry
possible effects with an owner rule reference and an evidence-qualified probability, plus a
follow-up page, an explicit terminal state, or an explicit unavailable state.
`EventField` keeps an observed empty collection distinct from not-observed, unsupported, denied,
failed, not-applicable, and unknown data.

### Page-to-option membership

Each narrative page carries an ordered `offered_options` association; options are not a global
unordered pool. The relation is validated before publication: every offered identity must name an
option of the same event, no page may offer the same option twice, every option must be offered by
exactly one page, and no option may remain uncovered. A page must be no more visible than any
option it offers, so a visible page never advertises an owner-only or hidden choice. This is the
authoritative static branch connectivity for the slice; a generic `Option` semantic reference stays
an informational rule/content link and is not membership evidence.

### Visibility on reference edges

Semantic-reference edges (`Page`, `Option`, and cross-event `Event`) are checked against target
visibility, not only target existence. A referencing record more visible than its target is
rejected with a typed `HiddenReferenceLeak` that omits the protected identity, so the rejection
cannot disclose it. `follow_up` page edges retain the stricter `HiddenFutureLeak` check. Exact
`get`/`get_page`/`get_option` lookups continue to withhold out-of-scope targets as
`ExcludedByScope`, so a hidden or owner-only page, option, or event can never be disclosed through a
reference edge.

### Withheld versus observed-empty collections

Projected eligibility, option requirements, costs, outcomes, and outcome effects are never silently
collapsed into ordinary empty vectors. Each keeps an explicit `EventFieldStatus`: `Available` for a
fully observed collection (including a genuinely empty one), `Partial` when the scope withheld some
but not all entries, and `Denied` when every entry was withheld. The visible remainder excludes the
protected entries, and the same status is reported on list summaries, so a hidden collection is
distinguishable from an observed-empty one.

### Amount sign convention

Fixed amounts follow one documented rule per kind. Every named cost
(`HpLoss`, `MaxHpChange`, `Gold`, `CardRemoval`, `PotionRemoval`, `RelicRemoval`, `ItemRemoval`) is a
non-negative magnitude because the kind already names the direction. Named effect kinds
(`AddCard`, `RemoveCard`, `ModifyCard`, `GainRelic`, `LoseRelic`, `GainPotion`, `LosePotion`,
`GainGold`, `LoseGold`, `Heal`, `Damage`) are likewise non-negative magnitudes, while
`MaxHpChange` alone is a signed delta that may be negative. Owner-defined, rule-backed, follow-up,
and unknown kinds leave the sign unspecified. A fixed magnitude below zero is rejected with a typed
`cost_amount` or `effect_amount` error; formula and unavailable amounts are unaffected.

`EventProbability` is `Exact { numerator, denominator, evidence }`, `Rule { rule_reference,
evidence }`, or `Unavailable(reason)`. It states a reference possibility only: it is never a sampled
result, never a seed-specific assignment, and an unknown probability is never converted into an
invented value. Definition identities are namespaced and scoped by event, option, and outcome;
`EventInstanceReference` and `EventActionReference` are separate live-run and transient-action
identities. A branch may not target a page more restricted than the outcome that reaches it, so a
hidden page is never revealed by a more visible branch, and the reader withholds hidden pages,
options, costs, and outcomes at public and owner scopes.

Duplicate or ambiguous event, page, option, requirement, cost, outcome, effect, and reference
identities are rejected before publication. Manifest, locale, and producer mismatches, unknown
manifest or intra-event page/option references, hidden-future leakage, malformed identities/text,
invalid probabilities/amounts/kinds, and definitions over the nested aggregate byte bound fail with
typed errors. An unsupported or unavailable family is never a successful empty page, and the byte
estimate counts nested custom kind strings so no field bypasses the bound.

## Evidence and limits

Synthetic fixtures cover a multi-page event, a conditional option, a numeric cost, a random outcome,
a selector-producing choice, bounded deterministic pagination with single-use continuations, exact
event/page/option lookup, hidden-outcome withholding, locked/owner-only/hidden visibility,
manifest/locale/producer and stale-reference rejection, unsupported/unavailable families, dangling
and duplicate identities, hidden-future leakage, malformed and oversized input, invalid
probabilities/costs/effects, and the nested definition byte limit. Added regression fixtures prove
that a visible page, option, or event cannot reference a hidden/owner-only target, that withheld
collections report `Denied`/`Partial` distinctly from `Available` empty ones, that page-to-option
membership rejects unknown, uncovered, and duplicate associations, and that invalid fixed-amount
signs are rejected per kind. These prove deterministic local validation and read-only projection
only. Cross-event reference visibility covers only event definitions in the same snapshot; manifest
content families carry no visibility metadata and are existence-checked. Native event coverage, live
run/instance and transient action reads, RNG evaluation, thread affinity, shared
transport/gateway/MCP delivery, and exact-host compatibility remain unverified.