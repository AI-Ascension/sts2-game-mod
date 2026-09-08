# ADR 0033: additive Runtime-v4 rest-option action transport

Status: proposed; requires agreement from the protocol, mod, gateway/MCP and harness owners
before any consumer or artifact update.

## Problem

The accepted `runtime-v4-expert` observation has a `rest_option` action arm with a
`rest_option_id`. The separately reviewed `runtime-v4-expert-action` transport accepts only
`use_potion`, returns only `potion_id` in its action reference, and settles only a
`potion_use_settled` transition. An observed native rest option therefore needs an additive,
strictly separate action profile.

## Decision

Keep the existing `runtime-v4-expert-action` profile byte-for-byte compatible. Its protocol
version, artifact, schema digest, endpoint, potion-only action, goldens, and settlement meaning
remain supported for existing clients. No old client is forced to understand this profile.

Add the independently versioned `runtime-v4-expert-rest-action-v1` profile:

* protocol version: `runtime-v4-expert-rest-action-v1`;
* artifact: `sts2-protocol/runtime-v4-expert-rest-action`;
* profile: `expert-rest-action`;
* schema source after protocol adoption: `schemas/runtime-v4-expert-rest-action-v1.schema.json`;
* endpoint: `/v4/instances/{instance_id}/expert-rest-action`;
* proposal schema digest: `bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd`.

The profile reuses the authenticated envelope fields and status vocabulary, but has its own
strict schema and digest. The action reference is an opaque native `rest_option` identity or a
typed rest-selector follow-up. `rest_option_id` remains the native `RestSiteOption.OptionId`;
the transport does not maintain a duplicated option-name enum.

## Transitions and witnesses

An effect-completing option returns:

```json
{
  "kind": "rest_option_completed",
  "before_generation": 7,
  "after_generation": 8,
  "rest_option_id": "heal",
  "completed": true,
  "effect_witness": { "version": "rest-effect-witness-v1" }
}
```

The real witness is a closed, typed object containing the operation ID, option ID, resulting
generation, and bounded option-specific evidence. The supported evidence forms are HP change,
card set change, relic set change, stat change, and native completion. Root and transition
`effect_witness` values are required to be equal by semantic validation. A generation increment,
screen closure, button disappearance, or HTTP success without this witness remains `unknown`.

An option that opens a selector returns a completed first boundary:

```json
{
  "kind": "rest_option_selection_requested",
  "before_generation": 9,
  "after_generation": 10,
  "rest_option_id": "smith",
  "selector": {
    "selection_id": "selection:10:smith",
    "selection_kind": "card",
    "required_count": 2,
    "selected_choice_ids": [],
    "remaining_count": 2,
    "legal_actions": []
  },
  "effect_witness": null
}
```

The selector contract is versioned by this profile and carries a stable selection identity,
required count, selected choices, remaining count, and a complete typed follow-up catalog. Each
follow-up is a separate generation-fenced operation. Smith may emit
`rest_option_selection_progressed` after each `select_card`; confirmation is valid only after
the required number of distinct cards has been selected. The final confirmation emits
`rest_option_selection_completed` with `smith_applied` card evidence. Mend uses the same
selector contract with `selection_kind: "player"` and a typed `select_player` action. Its final
receipt carries `mend_applied` evidence and the selected player identity. `cancel_selection` is
typed and operation-bound; a cancelled response has no settlement witness.

All settled transitions require `after_generation > before_generation` and
`after_generation` equal to the outer response generation. A selection boundary is never
encoded as `rest_option_completed`. The shared `runtime-v4-expert` observation remains nested
and unchanged; the rest profile's `transition.selector` catalog is authoritative for the
rest-specific player action and count-aware selector follow-up.

## Native admission and settlement

The intended producer is the rest-action companion to
`experiments/managed-rust-interop/game-loader/RuntimeV4ExpertSupport.cs`. On the host thread it
reobserves `LiveCombatSource.ObserveExpert()`, finds exactly one visible enabled
`NRestSiteButton` whose public `Option.OptionId` matches the requested ID, and retains the exact
button and option references. Dispatch invokes the native button path (`ForceClick` or the
reviewed typed selection method); it never calls an option implementation directly or
synthesizes a rest effect.

For selector options, the producer allocates a fresh selection ID only after the native selector
is visible, unambiguous, and actionable. It retains the native selector state and emits the
typed catalog. The Smith path must preserve repeated card selections and count state. The Mend
path must bind `NTargetManager`/`PlayerChoiceResult` to `select_player` and cancellation. The
final selector callback must complete before the producer emits the option-specific witness.
Missing, ambiguous, stale, or unavailable native evidence stays unknown and reconciles by the
original operation identity. `proceed` remains a separate action.

The intended consumer chain is gateway exact forwarding, MCP one-for-one action mapping, and
harness orchestration of `rest_option` followed by optional selector progress and confirmation.
Each owner validates protocol, digest, provenance, lease, operation, generation, selection ID,
counts, legal action catalog, and witness bindings. The full ownership and adoption checklist is
in [the implementation path](../proposals/runtime-v4-expert-rest-action-v1/implementation-path.md).

## Native coverage and evidence boundary

| Native option type | First boundary | Required final witness |
| --- | --- | --- |
| `CloneRestSiteOption` | effect completion | `clone_applied` with added card IDs |
| `CookRestSiteOption` | effect completion | `cook_applied` with removed card IDs |
| `DigRestSiteOption` | effect completion | `dig_applied` with added relic IDs |
| `HatchRestSiteOption` | effect completion | `hatch_applied` with added relic IDs |
| `HealRestSiteOption` | effect completion | `heal_applied` with HP/max-HP evidence |
| `KindleRestSiteOption` | effect completion | `kindle_applied` native completion evidence |
| `LiftRestSiteOption` | effect completion | `lift_applied` stat or native completion evidence |
| `SmithRestSiteOption` | selector boundary | `smith_applied` upgraded card IDs after count-complete confirmation |
| `MendRestSiteOption` | player selector boundary | `mend_applied` target player and native/HP evidence |

Linux v0.107.1 API metadata confirms the public native types used by this boundary:
`RestSiteOption`, `NRestSiteButton.Option`, `NRestSiteButton.SelectOption(RestSiteOption)`,
and `NRestSiteRoom.Options`/`GetButtonForOption`. That metadata does not establish every
runtime effect witness. The option rows, multi-card Smith flow, and Mend target flow therefore
remain implementation and host-verification gates, rather than advertised capabilities.

## Rollout

The protocol owner copies the proposal schema to its owned source path, recomputes the digest,
adds manifest, checksums, and conformance fixtures, and preserves the old potion artifact and
digest. The mod owner then implements serialization, native admission/dispatch, selector state,
and settlement. Gateway/MCP and harness owners add strict validators and round-trip fixtures only
after this decision is accepted. A non-author verifier must rerun schema, semantic mutation,
old-potion preservation, and serialized producer/consumer checks against the adopted digest.

No host files, saves, provider sessions, installation state, or live-game claim is part of this
proposal. The proposal schema and goldens are contract evidence only.
