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

An action is only interpretable against the context the party is in, so a member's values alone are
not enough: a caller also needs the public phase and step the party is on, which votes are open and
which members have signalled a choice, and which member is aiming at which. That context is public
by construction and is therefore published, while authority, synchronization receipts and the epoch
stay with the modules that already own them.

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
`CoopField` of the documented inventory. The inventory covers character, health,
character-specific resources, piles, relics, potions, powers, special mechanics, and the member's
readiness for the current public step; readiness is `PublicToParty` because it is the signal the
party acts on rather than private state. Character, relic, potion, power, effect and card
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

A member also carries a membership generation that a rejoin increments, and a reference carries the
generation it was minted for. A reference minted before a rejoin therefore names a member that no
longer exists and is refused as `StalePeerGeneration` rather than resolved against the rejoined
member, which is what keeps one member's pre-rejoin state from being read as its post-rejoin state.
Generations are unique within a party, so a fence cannot confuse two members.

### Public turn context, votes and targeting

`CoopPartyContext` publishes the party's phase, its monotonic public step, the votes it is taking,
and the targeting relationships between its members, and it is the same at every scope that
observes the party. A vote states what is being decided and which members have signalled a choice,
but never which option any member picked: a member's own selection is its private choice, so only
the fact that it decided is published. A vote taken in a phase that admits none, a vote whose own
phase contradicts the party's, a vote that repeats an option or names a member the party does not
carry, and a targeting relationship that names a non-member or names itself are each refused by
name, so an incoherent context cannot be read as a step that never resolves.

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
The fence also names the observation epoch, and a fence naming an epoch other than the one the
catalog holds is refused as `EpochMismatch`: an episode's record must not satisfy another episode's
fence, which is what keeps one live party read from being answered with another's state.
Members are listed through `CoopPeerListQuery` and paged through a single-use
`CoopPeerContinuation` bound to the catalog revision, the locale, the party, the scope and the page
size that minted it, so a page walk cannot silently mix two queries or survive a revision change.
Listing is bounded by `COOP_MAX_PAGE_ITEMS`, and a party, member, effect, scaling, resource, entry
or pile collection past its local bound, or a party past its aggregate retained-byte budget, is
refused by name.

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
whose local-only fields are refused, and a public context carrying one open vote and one targeting
relationship. Read-only fixtures prove one source read per production, that every later read is
served from the retained catalog, that a reader is not clonable, that a continuation is single-use
and reader-local, and that the local view is the only projection carrying local-only fields.

Rejection fixtures prove that a manifest, locale, or producer-version mismatch; a declared coverage
that does not match the party reported; a duplicate member, effect, scaling or pile identity; a
path-shaped or over-long identity; a party identity aliasing its instance or run; a party with no
local member or more than one; a value carried by an inactive or lagging member; a local-only value
or pile on an ally; a pile whose visibility contradicts its kind; an impossible health value; a
zero entry count; a dangling manifest reference; an unresolved or foreign effect target; a scaling
rule whose shape contradicts its kind; a duplicate membership generation; a member reference minted
before a rejoin; a vote taken in a phase that admits none, contradicting the party's own phase,
repeating an option, or naming a non-member; a targeting relationship naming a non-member or
itself; a live fence naming another observation epoch; an empty present collection; a collection
past its local bound; and an aggregate party past its byte budget are each refused with a closed
error.

These prove deterministic local validation, scope enforcement, and read-only projection only.
Native party extraction, live party reads, party membership changes, action dispatch, profile
selection, save loading, thread affinity, shared transport/gateway/MCP delivery, and exact-host
compatibility remain unverified. Sync receipts, authority grants and observation epochs are
deliberately not republished. The epoch a party was read at is carried only as the fence a live
read is checked against; no epoch is published as state a caller can act on.
