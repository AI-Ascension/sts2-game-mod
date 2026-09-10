# Intended producer and consumer path

This note makes the proposal reviewable without claiming cross-owner adoption. The protocol
profile remains a candidate until the protocol owner accepts the schema and each owner supplies
the evidence listed below. The mod now has a source-only producer candidate wired to its local
native route; that implementation does not admit the protocol artifact for gateway, MCP, or
harness use.

## Producer

The game-mod producer is the source-only rest-action companion to
`experiments/managed-rust-interop/game-loader/RuntimeV4ExpertSupport.cs`. It owns the
`/v4/instances/{instance_id}/expert-rest-action` request boundary, envelope identity checks,
operation receipts, host-thread dispatch, and serialization of this profile. It must not route
rest actions through the existing potion-only `RuntimeV4ExpertSupport` parser.

On the host thread, the producer reobserves `LiveCombatSource.ObserveExpert()`, finds the one
visible enabled `NRestSiteButton` whose `Option.OptionId` equals the requested opaque ID, and
retains that button and option reference for the operation. Immediate options invoke the native
button (`ForceClick` or the reviewed typed native method). The producer emits
`rest_option_completed` only after the retained native action completes, the observation
generation advances, and the option-specific witness is read from authoritative state.

For Smith and Mend, the first button invocation ends at an actionable native selector. The
producer allocates a fresh `selection_id`, records the native selector state, and emits
`rest_option_selection_requested`; the rest operation itself is settled at this boundary. Each
follow-up selection action is a separate operation and generation-fenced dispatch. Smith emits
progress after each card selection and accepts confirmation only when `remaining_count` is
zero. Mend exposes `select_player` and `cancel_selection` from its retained
`NTargetManager`/`PlayerChoiceResult` path. Selector closure and the native option callback must
complete before a final option-specific witness is emitted.

Selector progression is bound to the native choice surface. A prior choice that remains present on
that surface keeps its typed follow-up action unless it was selected; a native choice that becomes
unavailable may disappear from both the observation and the next catalog. Once `remaining_count`
reaches zero, no selectable action is emitted and the native `confirm_selection` and
`cancel_selection` controls remain. A completion that violates those catalog rules stays unknown.

A missing, ambiguous, stale, or unavailable native witness produces `unknown` and retains the
same operation identity for reconciliation. The producer never treats generation change,
screen closure, a button becoming unclickable, or an HTTP success as an effect witness.

## Consumers

The gateway forwards the profile only after exact protocol, digest, provenance, lease, instance,
session, and operation validation. It keeps the response envelope intact so that downstream
consumers can bind the witness; it does not reinterpret `rest_option_id` as a local enum.

The MCP adapter maps the rest action and the typed selector actions one-for-one. It exposes the
selector's `selection_id`, required and remaining counts, selected choice IDs, and legal action
catalog to the harness. It rejects a player selector unless the catalog contains the typed
`select_player` action and rejects a completion unless root and transition witnesses match.

The harness owns the sequence `rest_option` -> optional selector progress -> confirmation. It
uses a new operation ID for every follow-up, preserves the returned state and generation, and
reconciles an `unknown` result by the original operation ID. It never sends a follow-up against
an older selector generation or retries an uncertain native mutation as a new operation.

The shared `runtime-v4-expert` observation remains a nested state projection. Its `legal_actions`
field includes the additive `rest_option` actions projected from the currently visible enabled
native rest buttons. Stateful selector follow-ups remain in the rest profile's
`transition.selector` catalog, which is authoritative for `select_player`, count-aware Smith
selection, and the rest-specific cancellation path. This keeps selector state separate while
making the native rest-option surface visible through the shared expert observation.

## Evidence before adoption

1. The protocol owner copies `schema.json` to the owned schema path, recomputes its digest, and
   adds manifest, checksums, and conformance fixtures.
2. The mod owner implements the producer and supplies host-independent serialization tests plus
   authorized native evidence for immediate options, multi-card Smith, and Mend target flow.
3. Gateway, MCP, and harness owners add strict validators and a serialized request/response
   round-trip using these exact action and selector shapes.
4. A non-author verifier reruns schema, semantic mutation, old-potion preservation, and
   producer/consumer round-trip checks against the adopted digest.

The source-only managed probe in
`experiments/managed-rust-interop/rest-action-tests/` exercises the serialized producer and
consumer boundary, including rejection of a follow-up that drops a still-visible native choice
and acceptance of a follow-up after the native surface removes that choice. The Smith fixtures in
`smith-two-pick-confirmation-sequence.json` bind each follow-up request to the exact typed
catalog entry from the preceding response and require the action reference to be echoed without
an opaque ID rewrite.

The checked-in producer exporter and regression command is
`bash experiments/managed-rust-interop/rest-action-producer-capture.sh`. With no argument it
writes to an ignored temporary directory and compares both generated fixtures byte-for-byte with
the candidate producer payload. An optional output directory exports the same two files there.
It uses the selected .NET 9 SDK command, synthetic hosts, and isolated build/cache directories;
set `DOTNET` when the pinned SDK is not first on `PATH`. It does not read a host assembly or
write the production payload.

Until those gates pass, the profile remains a bounded protocol design proposal for cross-owner
consumers. The local mod source candidate does not advertise gateway, MCP, harness, or live
gameplay capability.
