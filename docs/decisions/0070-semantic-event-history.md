# ADR 0070: Owner-local semantic event and causal-provenance history

- Status: Proposed; source-only owner boundary
- Date: 2026-09-20
- Tracking: game-mod #128
- Depends on: [ADR 0038](0038-game-content-manifest-boundary.md), [ADR 0039](0039-field-availability-and-completeness-boundary.md), [ADR 0069](0069-coop-reference.md)

## Context

What happened in a run is not reachable as owned data. An agent cannot ask which cards were played,
how much damage, block or healing landed, which statuses and modifiers were applied or removed, how
cards moved between piles, when the party changed room, which choices were made, which offers were
presented, or what was purchased, and it cannot ask which of those events caused another.

The host observes all of it. It is also possible to recover a great deal of it by comparing two
snapshots: a health drop is damage, a pile difference is a card move, a new power is an application.
That reconstruction is attractive and wrong. A difference between two observations cannot say who
caused a change, whether two simultaneous changes were one event or two, or whether a change came
from the gameplay the boundary was watching at all. A history that infers causality from snapshot
differences manufactures causal claims the host never made, and a consumer cannot tell those claims
from observed ones. The same reconstruction hides its own gaps: a dropped frame becomes an event
that never happened, or a contiguous sequence that silently lost a step.

The harness owns the queryable history, its persistence, its indexing and its pagination. This
decision covers only the game-mod half: the bounded, authoritative semantic vocabulary the host
states, so the harness has something it can trust to query.

## Decision

`crates/game-mod/src/semantic_event_reference` owns an immutable `SemanticHistoryCatalog` produced by
`SemanticEventCatalogProducer` from a bounded `SemanticEventSnapshot` read through a read-only
`SemanticEventSource`. The catalog binds the existing content-manifest cursor and an owner-local
producer version, so a catalog and every read it produced are fenced to one content revision. A
snapshot whose manifest binding or producer version belongs to another revision is refused rather
than read, and a declared family that reports no history produces an explicitly empty catalog
instead of an absent one.

### A closed kind inventory, and no free-form event

`SemanticEventKind` is the closed inventory of fourteen kinds: card play, damage, block, heal,
resource change, status application and removal, modifier application and removal, pile movement,
room transition, choice, offer, and purchase. A kind this boundary cannot name is `Unsupported`
coverage rather than a free-form string, so a consumer never has to interpret an owner-defined label
to decide whether an event is understood. Each kind declares whether it requires a target, whether
it requires an actor, whether it must state a quantity, whether it must name content, and whether it
admits a cause; a record whose fields disagree with its kind is refused rather than published with a
default. No kind is derived from comparing two snapshots.

### Coverage as a record, not a reader's opinion

Every record carries its own `SemanticEventCoverage`. `Captured` is observed, `Dropped` and
`Unsupported` are disclosed gaps, and both are the same type so a gap still occupies its sequence
number and is never renumbered away. A gap carries no kind, origin, subject, quantity or reference,
because nothing was observed; a gap that carries an observed field is refused. The
`SemanticCaptureWindow` states where capture began, whether gameplay history exists before that
point, and every span inside the captured range that is not fully captured, so a consumer can tell
an absent event from an unwatched one. A captured record inside a declared gap, and a gap outside
any declared interval, are each refused: a disclosure that contradicts itself is not a disclosure.
Declared intervals must be incomplete, ordered, disjoint and inside the captured range.

### Sequence order inside one scope

Sequence numbers are monotonic and contiguous from the capture point inside one run, branch, episode
and epoch, and the `SemanticEventScope` travels with every event and every reference instead of
being assumed to be "the current run". `SemanticEventSequence::precedes` reports `false` for two
positions from different scopes rather than comparing two numbers that are not part of one order. A
repeat, a regression, or a history that does not begin where the window says capture began is
refused rather than renumbered.

### Causality as a stated fact

A `SemanticCausalParent` is either explicitly stated by the host or explicitly absent, and the two
fields must agree: a named parent is `Stated` and an unnamed one is `NotStated`. Any other pairing is
refused, so a consumer can rely on `provenance` alone to decide whether the absence of a parent means
anything. A kind that admits a cause must state one or its absence; a kind that admits none must
omit it. A stated parent that this history does not contain, or that does not precede its child, is
refused rather than kept as unverified causality, and a disclosed gap is not a stated cause. An
imported event carries no stated parent at all, because its causality was settled when it was
captured and re-deriving it at import time would be exactly the snapshot-difference inference this
vocabulary refuses.

### Namespaces, opacity, and manifest resolution

Definition, live-instance, action and event identities stay distinct, and the namespace travels with
each `SemanticEventSubject`, so two equal tokens from different namespaces are a collision to refuse
rather than an aliasing to accept. An actor or target is a live-instance identity, because an event
is about something that exists in the run; a definition, action or event identity used as a subject
is refused. A subject that aliases the event identity or the run it belongs to is refused. Every
identity is opaque: a control byte, a path separator or a traversal segment is refused, so this
vocabulary can never be turned into a host read. Content identities an event names resolve against
the content manifest, and a reference the manifest does not carry is refused rather than dropped.

### A read that observes and never acts

Reads are bounded and one-way. `SemanticEventReader` enforces the history fence, the catalog
binding, the scope and a single-use continuation, and it is deliberately not clonable so a
continuation cannot be replayed across readers. A fence naming another run, branch or episode is
stale; another observation epoch is refused rather than answered with this history. `SemanticHistoryView`
keeps gaps as records, and `SemanticEventSummary` summarises a gap by its coverage alone. Every
published read states `SemanticHistoryAuthority::NotGranted`: observing what happened is not
authority to replay it, to rewind to it, to re-run it, or to act in the run. No transport route,
native event capture, persistence, indexing, query engine, host ABI, or exact-host compatibility is
claimed.

## Consequences

- A harness that queries this history cannot mistake a reconstruction for an observation: an event
  is present because the host stated it, and a gap is present because the host disclosed it.
- A causal graph built from these records is as strong as its weakest stated parent, because every
  edge was named by the host rather than inferred, and every absence is marked as an absence.
- Consumers must handle coverage explicitly. A history is not a list of everything that happened;
  it is a list of what was observed plus the spans that were not, and code that ignores the window
  will overstate completeness.
- The boundary does not become a replay or rollback surface. Nothing here can re-apply an event,
  rewind a run, or change gameplay, so a history read cannot become a covert action channel.
- Emitting native events, persisting them, indexing them, querying them across runs, paginating
  them at the harness layer and traversing causality remain the harness owner's work; this record
  claims only the game-mod companion vocabulary it states.
