# ADR 0082: Retained live reward-offer reader

Status: source implementation proposal for supervisor review; no acceptance or runtime evidence is claimed.

## Context

The owner-local reward catalog describes static reward definitions and generation rules. It does not
observe the current run's reward offers. GameMod issue #100 needs a separately fenced source layer
that can retain visible live offers while keeping static definition IDs, live offer IDs, item IDs,
group IDs, and action IDs distinct. The frozen game-information query profile has no reward entity
kind; this decision adds no protocol kind, schema, route, or capability.

## Proposed source boundary

reward_reference::live defines the object-safe RewardLiveSource port. Its capture method returns
detached owned values for the selected instance and exact RewardCatalogBinding. No host-specific
implementation or guessed host member name is part of this slice. The reader owns the static
RewardCatalogReader, validates every supplied static reward/item/selection reference through it,
and retains one immutable live snapshot. A static item reference resolves before acquisition; the
reader does not require deck or inventory membership.

Each snapshot binds the manifest/locale/producer, selected instance, run, room, source epoch,
source state generation, source snapshot ID, and an optional source revision with an explicit
availability state. Live offer, group, item, item-instance, and action IDs use separate validated
types. Offer IDs are unique within one snapshot; group IDs and item IDs are unique within their
owning offer; action IDs are unique within their owning group. Repeated nested raw IDs under
different parents are valid and are distinguished by the full parent-qualified references. No
unconfirmed snapshot-global uniqueness rule is imposed on group, item, or action IDs. Per-reader
Arc scope plus checked reader-local generation fences references and move-only continuations; no
global counter, cursor registry, or stale-handle tombstone is retained.

Every new capture invalidates the preceding snapshot before calling the source. Source, bounds, or
validation failure leaves no current snapshot. Callers must invalidate after claim/selection,
room/run/epoch/profile changes, restore/restart, source failure, or catalog/locale replacement.
Queries never recapture and expose no claim, selection, skip, RNG, or mutation method. Live source
actions are copied only when supplied; no legal action is synthesized from a category or static
rule. A source-reported BlockedCapacity remains distinct from observed action presence. The
reader's disposition is a projection of those copied values, not proof of native host eligibility.

RewardField and RewardFieldStatus preserve available empty, zero, false, partial, denied,
not-observed, unsupported, failed, unknown, and not-applicable states. Hidden/unknown records are
not returned, and owner-only records require owner scope. Item reads and static item resolution
enforce the owning offer's visibility before inspecting child fields; a child handle cannot widen
the caller's scope. Inputs may represent currency, card,
relic, potion, special, custom, unsupported, and unknown offers. Exact base/visible quantity,
currency unit, and modification witnesses remain intact. Static generation rules are resolved as
definitions only; this reader never evaluates hidden RNG outcomes. Available content without a
static item binding is returned as unavailable/unsupported rather than advertised as a catalog
resolution.

## Source limits and accounting

The proposed source caps are 64 offers, 16 groups per offer, 64 items per offer, 16 actions per
group, 64 targets per action, 64 entries per page, 128 KiB accounted bytes per offer, and 1 MiB
accounted bytes per retained snapshot. Existing 256-byte identity and 16 KiB text limits apply;
formula inputs remain capped at 32. Counts are rejected before retention, input is measured without
Debug, and checked arithmetic fails closed. The accounting estimates owned Rust allocation
capacity; it is not a canonical JSON, UTF-8 response, MCP, gateway, or transport byte proof. A
future negotiated serializer must independently enforce its request, item, page, text, cursor,
depth, and aggregate UTF-8 budgets.

## Compatibility, evidence, and remaining owner facts

This is a source-only prerequisite for #100. It does not change the static catalog, old exported
types, frozen protocol profile, managed producer, Gateway, MCP, Harness, or consumer repositories.
The Rust fixture composes the capture/retain/query/resolve/invalidate path with PR146's existing
synthetic catalog producer and reader. It is synthetic source evidence only. Validation receipts
apply to their pinned source revision; they do not automatically validate later source changes.
Synthetic fixtures do not establish host integration or runtime behavior.

The exact host type/member that provides current visible live groups, item identities, action IDs,
states, quantities, modified currency, source revision, and visibility/completeness remains
unverified. Exact source-thread affinity and claim/room/restore invalidation hooks also remain
unverified. The adapter owner must map only members confirmed in the selected host build and refuse
unavailable facts explicitly. A disagreement among host eligibility predicates remains a
conditional risk; it does not establish native reachability. Caller-supplied evidence labels do not
establish independent owner provenance. Native UI/read-only/RNG/claim/room acceptance and downstream
consumer adoption require separate qualified receipts.
