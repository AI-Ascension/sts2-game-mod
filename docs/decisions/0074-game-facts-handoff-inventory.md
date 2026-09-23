# ADR 0074: Game-facts handoff inventory

Status: proposed; source-only inventory and refusal tests executed; native parity pending.

## Requirement and owner

Refs sts2-game-mod#207, a proposed decomposition subtask of sts2-game-core#13. Core retains the
pure rule models; game-mod owns the exact-build facts and the host comparison, and adds no host
dependency to core and no transport-wide ownership. The issue's initial blocker is owner agreement
on the supported rule inventory and a negotiated typed payload, because existing closed query
envelopes cannot silently carry new structured rules fields. This record covers the owner inventory
that agreement needs; it does not claim that agreement, and it does not claim native parity.

## Decision

Introduce `crates/game-mod/src/game_facts_reference`, a source-only owner inventory of supported
rules. One inventory binds, once and immutably:

- the exact build and mode it was taken from, together with the existing content-manifest
  invalidation witness (`ContentCursorBinding`);
- the negotiated structured representation, named and versioned rather than left in prose;
- each supported rule's opaque id, its evidence status, and the inputs it copies.

Every copied input carries an opaque name, the unit it is stated in, and whether it is required,
conditional, or explicitly unknown. The unit travels with the name so a fact can never be carried
as a bare prose tag that a consumer has to guess at, and the availability travels with both so a
consumer sees a disclosed unknown instead of an invented value.

Each rule records how it is known, using the repository's existing claim labels: `Confirmed`,
`SourceDerived`, `Proposed`, `Inferred`, or `Unverified`. Only a host-confirmed rule supports an
exact claim. The inventory also records the combinations it declares it cannot represent exactly;
a rule that takes part in such a combination never reads as an exact claim even when its own entry
is confirmed, so an unsupported interaction never returns an exact-looking result from a simplified
model.

## Refusals

The inventory is closed and fails closed at construction. It refuses an empty rule set, a rule that
copies no typed input, a repeated rule id, a repeated input name inside one rule, an input or build
identity that is not opaque, and a record that exceeds its local count or byte bound. A declared
unsupported combination must name two or more distinct *declared* rules and carry a bounded reason;
a combination naming an undeclared rule is refused rather than accepted as a fact about a rule the
inventory does not state.

Identities are opaque: a filesystem path, a control byte, a path separator or a traversal segment is
refused, so this vocabulary can never be turned into a host read.

## Compatibility and exclusions

This is additive. It adds one module and its tests, changes no wire schema, response code, profile,
digest, grant or legacy behavior, and it does not touch the pure core models or any transport.

The read-only adapter that would extract allowlisted facts through game-owned access and map them
into accepted core inputs is deliberately **not** built here. That adapter, and its rounding and
ordering provenance, belongs with the negotiated payload, and the exact-host parity evidence stays
with the explicitly authorized native lane. A schema-only or synthetic slice is not completion of
#207, and this record claims none.
