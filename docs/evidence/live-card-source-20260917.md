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
advances a source-owned epoch and emits a new snapshot identity while retaining the gameplay
observation generation from the existing runtime-v3 source as a separate field. The same seed
after invalidation or a recreated source therefore cannot reuse a prior card occurrence.

The managed capture currently copies the fields that the installed host adapter already observes:
card definition entry, content-manifest input, player owner identity, hand/deck/discard/exhaust
zone and position, upgrade level, title, upgraded flag, and resolved current cost. Base cost,
effective-cost provenance, upgrade variant/path, modifiers, flags, and effect-parameter overrides
remain explicit `not_observed` fields. A missing required definition, manifest, owner, location,
or upgrade field rejects the whole capture rather than publishing an incomplete identity fence.
Card collection size, auxiliary collection size, and auxiliary string byte bounds are enforced
before publication. Unobservable collections are never represented as empty values.

An instance selector now reads the immutable snapshot retained by the preceding authenticated
lookup-binding read. It does not recapture and advance the source epoch while checking that
selector. A source invalidation clears the retained snapshot before a new owner observation can
be admitted. LBR and bootstrap both pass through the existing runtime-v2 owner fence, including
caller, session, lease, and lease epoch.

## Evidence and limits

`live-card-source-tests/LiveCardSourceProbe.csproj` exercises source-owned capture with synthetic
object handles. It proves distinct duplicate-definition instances, same-run handle retention,
snapshot freshness, invalidation and repeated-seed rotation, duplicate rejection, required-field
rejection, explicit unavailable statuses, and the card-count bound. Those tests do not prove the
external STS2 process, host assembly compatibility, native interop delivery, or a stable native
CardModel occurrence ID.

The native listener now admits a fixed `POST /api/v1/game-information/live-observation-bootstrap`
route and dispatches a dedicated callback kind. The managed route parser is pinned to the
candidate bootstrap schema digest. It exports the workflow run namespace only after an
authenticated lookup-binding association matches the current source incarnation and native run
handle; the source-owned native run ID remains internal. The explicit invalidation hook is
available to restore/session owners, while wiring every host lifecycle event remains unverified.
The current content-manifest producer is unavailable, so no native route success is claimed.

The exact pinned host metadata confirms `ModelDb._contentById` as a private
`Dictionary<ModelId, AbstractModel>` and exposes `ModelId.Category/Entry`,
`AbstractModel.IsCanonical/IsMutable` and category/entry sorting fields. The new bounded
`NativeContentCatalogOwnerObservation` copies those values on the owner thread and groups actual
registry counts. The same metadata exposes no catalog generation/reload witness, definition
origin or override chain, or canonical semantic-input reader; those missing fields keep the
canonical producer fail-closed. This is source evidence, not a complete manifest or live-process
acceptance claim.

The Rust `LiveCardSource` contract remains owner-local and fixture/unavailable until a versioned
mapping is agreed. The managed DTO is intended to be mapped by that owner; it is not a protocol
or gateway capability claim. The adapter returns an unavailable result if the host callback cannot
complete synchronously. Exact host execution and any native snapshot/restore source remain
unverified.
