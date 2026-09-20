# ADR 0069: Owner-local co-op party and member-state reference

- Status: Proposed; source-only owner boundary
- Date: 2026-09-20
- Tracking: game-mod #106
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0041](0041-save-profile-selection-and-disposable-boundary.md), [ADR 0060](0060-run-configuration-reference.md)

## Context

A co-op party is not reachable as owned data. An agent cannot read which members are in the party,
which character each is playing, what each member's health, resources, relics, potions, powers and
special mechanics are, what the shared effects and scaling rules are, or which piles each member
holds. The host stores every member's state, so the values are present in the process, but presence
in a host that keeps every member's state is not permission to publish it: a hand, a draw pile and
a potion stock belong to the member that owns them, and a read that treats the host's possession as
permission would disclose one player's private state to another.

Reading those values through the game's own party screen means driving the game, and the party
screen is only reachable inside a run. The organisation also evaluates co-op runs outside the game,
where a member's data is a snapshot that lags, and a member that is joining, disconnected or gone
is not the same as a member whose values are simply absent.

This decision defines an owned source boundary that copies one party and its members into an
immutable catalog that can be read without acting in the party, selecting the single-player save
profile, or touching a save. It does not claim a native extractor, a live party read, a transport
route, a gateway/MCP adapter, or exact-host compatibility.

## Decision

`crates/game-mod/src/coop_reference` owns an immutable `CoopCatalog` produced by
`CoopCatalogProducer` from a bounded `CoopCatalogSnapshot` read through a read-only
`CoopCatalogSource`. The catalog binds the existing content-manifest cursor, the locale, and an
owner-local producer version, so a catalog and every read it produced are fenced to one content
revision and one locale. A snapshot whose manifest binding, locale, or producer version belongs to
another revision is refused rather than read, and a declared family that reports no party produces
an explicitly empty catalog instead of an absent one.

### Member fields, stated availability, and the closed inventory

Every member value is a `CoopFieldValue` carrying its own `CoopFieldStatus`, so an unknown value is
never converted into a zero, an empty collection, or an invented description. `Absent` (the host
reports no value for this member), `Unsupported` (the supported build does not carry the field),
`Withheld` (the owner keeps it), `NotPermitted` (this scope may not observe it) and `Stale` (the
retained value is older than the fence) stay distinguishable, and each field is one closed
`CoopField` of the documented inventory. Character, relic, potion, power, effect and card
identities resolve against the content manifest, so an entry naming a definition the manifest does
not carry is refused as `UnknownManifestReference` instead of being published as opaque text.

### Per-field visibility, and a refusal that is not an empty pile

Visibility is a property of the field, not of the reader. A field is `PublicToParty`,
`ExplicitlyShared`, `LocalOnly`, or `Unavailable`, and each pile kind answers the question for
itself: the discard, exhaust and play piles are shared, while the draw pile and the hand are local.
An ally's local-only pile is reported as `NotPermitted` with its kind and visibility intact, so a
reader sees "the hand exists and is not yours to see" rather than an empty hand, and a local-only
value carried by an ally is refused outright as `LocalOnlyValueOnAlly` rather than filtered. A
party declares exactly one local member, because the local-only fields belong to exactly one
member: a party with none could not own them, and a party with two would let either member read the
other's.

### Membership, freshness, and identity that is not a save

A member's membership and the freshness of its data are separate from the values they qualify. A
member that is joining, disconnected or left has no coherent snapshot, and a member whose data
lags is not published as current, so a present value in either state is refused as
`GameplayForInactivePeer` or `ValueWithoutCurrentFreshness` rather than shown with a qualifier a
reader might not read.

Identities are opaque: a path separator, a control byte, a traversal segment or an over-long value
is refused as `NonOpaqueIdentity` rather than sanitized, so a value that looks like a save or
profile path is never published as a party identity. A party is scoped to one live instance and one
run and is never the single-player save profile, so a party identity that aliases its own
`instance_id` or `run_id` is refused as `IdentityNamespaceCollision` instead of being resolved
against a save-scoped name later.

### Shared effects, scaling claims, and bounded listing

A shared effect states its scope: a party-wide or per-member effect names no target, and a targeted
effect must name a member the party actually declares, so a dangling target cannot be mistaken for
a member the reader simply cannot see. A scaling rule states which claim it makes instead of
publishing one number for all of them: a shared pool, a per-member increment and a
target-amplified effect are three different rules, a per-member rule must state its increment, and
an amplified rule must name an effect the party carries. A rule whose shape contradicts its own
kind is refused as `ScalingTargetMismatch`.

A retained party read needs no fence, and supplying one is refused as `UnexpectedLiveFence`; a
current party read requires a `CoopLiveFence` naming the instance and party the observation was
taken at, and an incomplete or foreign fence is refused as `MissingLiveFence` or `StaleLiveFence`.
Members are listed through `CoopPeerListQuery` and paged through a single-use
`CoopPeerContinuation` bound to the catalog revision, the locale, the party, the scope and the page
size that minted it, so a page walk cannot silently mix two queries or survive a revision change.
Listing is bounded by `COOP_MAX_PAGE_ITEMS`, and a party, member, effect, scaling, resource, entry
or pile collection past its local bound, or a party past its aggregate retained-byte budget, is
refused by name.

### Membership generations, and a reference that survives a rejoin

A member carries a membership `generation` that a rejoin increments, and `readiness` is one more
row of the same closed field inventory, so a member's stated readiness for the current public step
is an availability like any other rather than a bare boolean. A generation is unique within one
party, because two members sharing one would make a generation fence unable to tell a pre-rejoin
reference from a post-rejoin one; a repeat is refused as `DuplicatePeerGeneration`. A
`CoopPeerReference` names the generation it was minted for, so a reference produced before a rejoin
names a member that no longer exists and is refused as `StalePeerGeneration` instead of being
resolved against the rejoined member.

### Public context, and a vote that cannot mislead

The party's public context carries the phase it is in, the monotonic public step, the votes it is
taking and the targeting relationships between its members. It is deliberately separate from
authority: legal-action authority, synchronization receipts, host authority and the observation
epoch stay with the modules that already own them. A vote is public by construction, so it states
what is being decided and which members have signalled a choice, never which option any member
picked. A vote is refused as `VoteOutsideVotingPhase` when its own phase admits none or when it
contradicts the phase the party declares, as `InvalidVote` when it repeats an option identity or
names a member the party does not carry, and a targeting relationship is refused as
`InvalidTargeting` when it names itself, names a member the party does not carry, or repeats one
relationship. The phase is a property of the party rather than of one member, because two members
of one party cannot be in two phases at once.

### The live fence's epoch

A current party read is fenced to the observation epoch the catalog holds, not merely to the
instance and party: a fence naming another epoch is refused as `EpochMismatch` with both the
expected and the held epoch, so a read cannot be served from an observation the caller did not ask
for.

### Read-only by construction

`CoopCatalogSource::read_catalog(&self, ...)` is the only production seam and `CoopCatalog` exposes
no setter. The reader surface offers `party`, `current`, `peer`, `list_peers` and `local_view` and
nothing else: there is no party join, no leave, no invite, no action dispatch, no profile
selection, and no save load. `CoopReadAuthority` is exactly `NotGranted`, so every published party,
page and member view states the authority it withholds, and the type makes the alternative
unrepresentable rather than merely discouraged. A shared borrow is enough to read everything,
which is what makes the boundary read-only by construction.

## Evidence and limits

Synthetic fixtures cover one two-member party over one manifest: a local member and an ally, with a
shared effect, a targeted effect, and the three scaling claims as separate rules, plus an ally
whose local-only fields are refused. Read-only fixtures prove one source read per production, that
every later read is served from the retained catalog, that a reader is not clonable, that a
continuation is single-use and reader-local, and that the local view is the only projection
carrying local-only fields.

Rejection fixtures prove that a manifest, locale, or producer-version mismatch; a declared coverage
that does not match the party reported; a duplicate member, effect, scaling or pile identity; a
duplicate membership generation; a reference naming a generation the party no longer holds; a fence
naming another observation epoch; a vote in a phase that admits none, contradicting the party's
phase, repeating an option, or naming a member the party does not carry; a targeting relationship
naming itself, a stranger, or repeating; a
path-shaped or over-long identity; a party identity aliasing its instance or run; a party with no
local member or more than one; a value carried by an inactive or lagging member; a local-only value
or pile on an ally; a pile whose visibility contradicts its kind; an impossible health value; a
zero entry count; a dangling manifest reference; an unresolved or foreign effect target; a scaling
rule whose shape contradicts its kind; an empty present collection; a collection past its local
bound; and an aggregate party past its byte budget are each refused with a closed error.

These prove deterministic local validation, scope enforcement, and read-only projection only.
Native party extraction, live party reads, party membership changes, action dispatch, profile
selection, save loading, thread affinity, shared transport/gateway/MCP delivery, and exact-host
compatibility remain unverified. Sync receipts, authority grants and observation epochs are
deliberately not republished.
