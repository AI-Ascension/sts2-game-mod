# Managed live-card source slice

## Scope

This source-only slice adds a bounded managed capture seam for the owner-local live-card
projection. `LiveCombatSource.CaptureLiveCardSnapshot` runs through the existing
`RequireThread` guard, copies supported `CardModel` values into owned records, and releases all
host references before returning. `LiveCardSourceReadAdapter` is configured alongside the
existing runtime-v3 host-thread owner, and `ModEntry.ReadLiveCardSnapshot` invokes that adapter
for an authenticated instance/content-manifest pair. The source does not mutate gameplay,
serialize a native save, or add an HTTP route.

`LiveCardSnapshotRegistry` assigns occurrence handles to the actual observed `CardModel`
references while they remain in one source-owned run incarnation. The emitted IDs contain a
per-source lifetime nonce, run incarnation, and bounded ordinal; object hashes and host pointers
never cross the boundary. A run stop, source invalidation, changed `RunState` reference, or
changed run key clears the handle table and rotates the incarnation. Every successful capture
advances a source-owned epoch and emits a new snapshot identity. The same seed after invalidation
or a recreated source therefore cannot reuse a prior card occurrence.

The managed capture currently copies the fields that the installed host adapter already observes:
card definition entry, content-manifest input, player owner identity, hand/deck/discard/exhaust
zone and position, upgrade level, title, upgraded flag, and resolved current cost. Base cost,
effective-cost provenance, upgrade variant/path, modifiers, flags, and effect-parameter overrides
remain explicit
`not_observed` fields. A missing required definition, manifest, owner, location, or upgrade field
rejects the whole capture rather than publishing an incomplete identity fence. Unobservable
collections are never represented as empty values.

## Evidence and limits

`live-card-source-tests/LiveCardSourceProbe.csproj` exercises source-owned capture with synthetic
object handles. It proves distinct duplicate-definition instances, same-run handle retention,
snapshot freshness, invalidation and repeated-seed rotation, duplicate rejection, required-field
rejection, explicit unavailable statuses, and the card-count bound. Those tests do not prove the
external STS2 process, host assembly compatibility, native interop delivery, or a stable native
CardModel occurrence ID.

The Rust `LiveCardSource` contract remains owner-local and fixture/unavailable only until a
versioned mapping is agreed. The managed DTO is intended to be mapped later by that owner; it is
not a protocol or gateway capability claim. The adapter returns an unavailable result if the host
callback cannot complete synchronously. Exact host execution and any native snapshot/restore
source remain unverified.
