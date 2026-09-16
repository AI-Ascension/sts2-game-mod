# ADR 0043: game-owned native checkpoint capture port

- Status: Proposed; source-only contract
- Date: 2026-09-13
- Tracking: game-mod #80
- Depends on: [ADR 0037](0037-native-checkpoint-coverage-inventory.md)

## Context

ADR 0037 inventories the state that a native checkpoint must close and keeps every phase
unverified. A follow-up implementation needs one owner boundary that can reject an unsafe
request without inventing a public observation, earlier floor, or resumable save. The protocol
identity contract is available as an inert consumer contract, but it does not authorize native
capture or define game-specific fields.

## Decision

`crates/game-mod` owns a typed `CheckpointCapturePort` for the game-thread capture seam. A request
is bound to the live instance, session, lease and epoch, run, profile, and logical operation. Each
identity component is non-empty and bounded to 256 UTF-8 bytes. The port classifies the complete
boundary matrix before it can return an artifact:

| Boundary | Current capability |
| --- | --- |
| Settled map choice | unavailable: exact-host evidence required |
| Stable player-turn combat | unavailable: exact-host evidence required |
| Later-turn combat | unavailable: exact-host evidence required |
| Reward/card selection, event, shop, rest | unavailable: coverage inventory pending |
| Enemy turn, animation, transition, unknown phase | rejected as `unsafe_boundary` |

The checked-in `UnavailableCheckpointCapture` implements this disposition for every request.
No route, native callback, persistence operation, or phase is enabled by this record.

When a producer is eventually admitted, it must execute on the host game thread after admitted
action/save/effect work settles, hold an owner barrier through boundary validation and capture,
and transfer only immutable owned bytes. A busy, mid-effect, enemy execution, selection
transition, unknown settlement, missing required field, operation conflict, or persistence
failure returns a typed rejection. A durable success is never published when persistence fails.
Duplicate operation admission returns the original receipt only when the bound request is
identical; a changed request conflicts.

`CheckpointCaptureReceipt` is the trusted artifact channel. It keeps canonical bytes private to
the owner, records whether the result is `InMemory` or `Durable`, and exposes the exact-state,
checkpoint-manifest, and blob digests separately. The checkpoint ID is computed from the
canonical `ascension.checkpoint_manifest.v1` envelope, including compatibility and coverage
references, restore descriptors, boundary, origin, and parent identity; it is not a digest of the
exact-state payload alone. The wrapper accepts bytes already validated by the protocol owner; it
does not implement canonicalization, restore, or a native state extractor.
Ordinary model/browser surfaces must receive only scoped opaque references from a later
control-plane owner.

## Protocol-vector witness

`protocol-artifact/exact-state-v1/selected-vectors.json` pins the complete positive, equivalence,
and distinctness set from `sts2-protocol` revision `8a2e66f5d2190a0fca7f146dc3508e8d55515ea`:
26 positives, 2 equivalence pairs, and 8 distinctness pairs, including the profile guarantee that
absent, null, empty, and explicit unknown values stay distinct. It covers key ordering,
tagged `uint64`, signed zero, unchanged public fields with changed hidden health/RNG values, and
raw duplicate/numeric/trailing-input rejection cases. The test recomputes the recorded
domain-separated state/blob digests from the pinned canonical bytes, then independently compares
`golden-manifest.json` and its manifest-derived checkpoint ID. The complete canonicalization
implementation and conformance suite remain protocol-owned; this target deliberately does not
duplicate them.

## Evidence and limits

The vectors and port tests are synthetic source evidence only. They do not prove exact-host
field availability, native serialization, thread affinity, capture stability, persistence,
restoration, or support for any STS2 phase. Native capability remains unavailable until the
coverage inventory, exact-host capture tests, and independent evidence required by ADR 0037
pass.
