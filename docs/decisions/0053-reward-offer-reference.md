# ADR 0053: Owner-local reward offer definition and generation reference data

- Status: Proposed; source-only owner boundary
- Date: 2026-09-14
- Tracking: game-mod #100
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0052](0052-event-reference.md)

## Context

Reward choices in the expert profile are generic value/label/kind records with a null domain; they
carry no complete item definitions or quantities, no selection/skip constraints, and no generation
rules. An agent cannot compare the exact contents of an offered reward or inspect the rules that
generated a reward pool independently of the current run. This decision defines an owned source
boundary. It does not claim a native extractor, a live run read, an RNG evaluation, a transport
route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/reward_reference` owns an immutable `RewardCatalog` produced by
`RewardCatalogProducer` from a bounded `RewardCatalogSource`. Static results bind to the existing
content-manifest cursor, the exact locale, and an owner-local producer version; a
`RewardCatalogReader` fences listing and exact lookups by locale, visibility scope, and single-use
continuations.

Each reward offer definition carries a localized label, a reward category (currency, card, relic,
potion, special grant, or owner-defined/unsupported/unknown), unlock and visibility state, a
selection group with choose/skip bounds and legal action references, offered items with typed
content definition and optional live instance references, static generation pool/rarity/eligibility/
modifier rules with evidence, and a state policy covering claim limit, capacity, replacement, and
multi-stage. `RewardField` keeps an observed empty collection distinct from not-observed,
unsupported, denied, failed, not-applicable, and unknown data.

### Definition, instance, and action identities

Static `RewardDefinitionReference` and `RewardItemReference` are scoped by the catalog. A live
`RewardOfferReference` binds a run/room/snapshot fence, a live `RewardItemInstanceReference` names an
existing item instance, and a transient `RewardActionReference` binds one rendered selection. These
are deliberately distinct types, so a definition ID, a live offer/item instance ID, and a transient
action ID can never be confused. This source-only slice defines the live identity vocabulary but
does not read a run.

### Preserved units and modified amounts

`RewardQuantity` retains the unmodified base amount, the exact visible amount after applied
modifiers, an explicit `modified` witness, and a unit identity. A currency quantity requires an
observed unit; card/relic/potion counts leave the unit explicitly not-applicable rather than
inventing one. Named categories are non-negative magnitudes; owner-defined, rule-backed, and unknown
categories leave the sign unspecified. A fixed magnitude below zero, or a fixed quantity whose
visible value contradicts its explicit `modified` witness, is rejected.

### Generation rules and probability

Each generation rule carries an explicit candidate pool, weighted rarity entries, eligibility
predicates, modifier rules, and an evidence label. `RewardProbability` is
`Exact { numerator, denominator, evidence }`, `Rule { rule_reference, evidence }`,
`Conditional { rule_reference, condition_reference, evidence }`, or `Unavailable(reason)`. It states
a reference probability only: it is never a sampled result, never a seed-specific assignment, and an
unknown probability is never converted into an invented value. Static rules remain distinct from a
future unrevealed roll, which is not represented as a value here.

### Offer states

`RewardOfferState` keeps offered, already-claimed, blocked-capacity, replacement-required,
optional-skip, multi-stage, and unknown distinct. The static `RewardStatePolicy` records which
states a definition can produce, but it is never itself a live observation and is not derived from
the selected run.

### Visibility on reference edges

Semantic-reference edges (`Reward`, `Item`) are checked against target visibility, not only target
existence. A referencing record more visible than its target is rejected with a typed
`HiddenReferenceLeak` that omits the protected identity, so the rejection cannot disclose it; a
generation rule more visible than an item it offers is rejected with the stricter `HiddenFutureLeak`.
Reserved-family alias spellings are normalized to their canonical reference kind before visibility
is enforced: a generic `Content { entity_kind: "reward" }` reference resolves to the same manifest
reward family as `Reward`, so the alias path and the canonical path produce the same rejection and a
public record cannot disclose a hidden or owner-only reward identity or label. Exact lookups withhold
out-of-scope targets as `ExcludedByScope`.

### Item membership

Each generation rule pool names the offered items it can produce; the relation is validated before
publication. Every offered item must be named by exactly one rule pool, no rule may offer the same
item twice, and no item may remain uncovered. This is the authoritative static connectivity for the
slice; a generic content reference stays an informational rule/content link and is not membership
evidence.

### Withheld versus observed-empty collections

Projected items, generation rules, rule rarity/eligibility/modifiers, and selection legal actions
are never silently collapsed into ordinary empty vectors. Each keeps an explicit `RewardFieldStatus`:
`Available` for a fully observed collection (including a genuinely empty one), `Partial` when scope
withheld some but not all entries, and `Denied` when every entry was withheld. The visible remainder
excludes the protected entries, and the same status is reported on list summaries, so a hidden
collection is distinguishable from an observed-empty one.

Duplicate or ambiguous definition, item, rule, requirement, modifier, action, and reference
identities are rejected before publication. Manifest, locale, and producer mismatches, unknown
manifest or intra-definition item references, hidden-future and hidden-reference leakage, malformed
identities/text, invalid probabilities/quantities/bounds/state policies, and definitions over the
nested aggregate byte bound fail with typed errors. An unsupported or unavailable family is never a
successful empty page, and the byte estimate counts nested custom kind strings so no field bypasses
the bound.

## Evidence and limits

Synthetic fixtures cover a multi-category catalog (currency, cards, relics, potions, special
grants), multiple offered items under a choose-one/optional-skip group, a modified currency value
with a preserved unit, weighted rarity, eligibility, modifiers, and generation probability,
bounded deterministic pagination with single-use continuations, exact reward/item lookup, hidden
item and rule withholding, locked/owner-only/hidden visibility, manifest/locale/producer and
stale-reference rejection, unsupported/unavailable families, dangling and duplicate identities,
hidden-future and reserved-alias leakage, malformed and oversized input, invalid
probabilities/quantities/bounds, and the nested definition byte limit. These prove deterministic
local validation and read-only projection only. Cross-reward reference visibility covers only reward
definitions in the same snapshot; manifest content families carry no visibility metadata and are
existence-checked. Native reward coverage, live run/room/snapshot offer and transient action reads,
RNG evaluation, thread affinity, shared transport/gateway/MCP delivery, and exact-host compatibility
remain unverified.