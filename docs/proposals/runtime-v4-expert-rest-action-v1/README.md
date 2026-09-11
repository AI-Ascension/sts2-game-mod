# Proposed `runtime-v4-expert-rest-action-v1` profile

This directory is a review proposal. It is not an active protocol artifact and does not change
the existing `runtime-v4-expert-action` potion profile or the shared `runtime-v4-expert`
observation schema.

The proposed profile gives native rest-site options their own versioned action transport:

* protocol version: `runtime-v4-expert-rest-action-v1`;
* artifact: `sts2-protocol/runtime-v4-expert-rest-action`;
* profile: `expert-rest-action`;
* schema source after protocol adoption: `schemas/runtime-v4-expert-rest-action-v1.schema.json`;
* proposed schema SHA-256: `bb3555fae28eb1f79d08a15e9884696a579e4c20836f5016509f17e0f4c36fbd`;
* proposed route: `/v4/instances/{instance_id}/expert-rest-action`.

The action reference is either an opaque native `rest_option` identity or a typed follow-up
selection action. Every response repeats the submitted action. A settled response binds its
transition and effect witness to the operation, option, and resulting generation. Requests,
accepted responses, and rejected, unknown, or cancelled responses carry a null effect witness.

`rest_option_completed` is valid only with a versioned `rest-effect-witness-v1` object. The
witness contains the operation ID, option ID, resulting generation, and bounded option-specific
evidence: card, relic, HP, stat, or native completion data. The same witness is present in the
transition and root response field; a consumer must require the two objects to be equal. A
generation increment or button disappearance without this evidence remains `unknown`.

Selector options use `rest_option_selection_requested` as a completed first boundary. Its typed
selector object carries a stable `selection_id`, `selection_kind`, `required_count`, selected
choice IDs, remaining count, and a complete bounded follow-up catalog. A card selection can
return `rest_option_selection_progressed` after each typed `select_card`; Smith then requires
the advertised number of distinct cards before `confirm_selection`. The final confirmation
returns `rest_option_selection_completed` with `smith_applied` evidence. A player selector uses
the same contract with a typed `select_player` action, so Mend no longer depends on an action
arm in the shared observation: the shared observation carries the native `rest_option` action,
while stateful selector follow-ups remain in `transition.selector`. `cancel_selection` is typed
and operation-bound; its cancelled response carries no settlement witness.

The producer and consumer ownership, dispatch ordering, option evidence mapping, and adoption
gates are recorded in [implementation-path.md](implementation-path.md). The producer must
retain the native button or selector state on the host thread and must never synthesize an
effect from a changed generation. The consumer must reject the profile when its protocol,
digest, provenance, selection identity, count, action catalog, or witness bindings do not
match.

The proposal goldens cover:

* `golden/action-request.json`;
* `golden/action-accepted.json`;
* `golden/action-completed.json`;
* `golden/action-selection-requested.json`;
* `golden/action-selection-first-request.json`;
* `golden/action-selection-progressed.json`;
* `golden/action-selection-second-request.json`;
* `golden/action-selection-second-progressed.json`;
* `golden/action-selection-confirm-request.json`;
* `golden/action-selection-completed.json`;
* `golden/action-selection-early-confirm-rejected.json`;
* `golden/action-mend-selection-requested.json`;
* `golden/action-mend-selection-completed.json`;
* `golden/action-rejected.json`;
* `golden/action-unknown.json`.

The Smith request/response chain is indexed by
[`smith-two-pick-confirmation-sequence.json`](smith-two-pick-confirmation-sequence.json). It
binds the first pick to the generation-10 selector catalog, the second pick to the refreshed
generation-11 catalog, and confirmation to the refreshed generation-12 catalog. Each response
echoes the exact action reference submitted by its request; the final response settles at
generation 13 with both upgraded card IDs.

The digest is recomputed from the exact `schema.json` bytes. Protocol adoption must copy the
schema to its owned source path, recompute the digest over that committed file, add a manifest
and checksums, and preserve the old potion artifact and digest unchanged. These fixtures and
the schema are proposal evidence; they do not prove a licensed host build, native runtime
settlement, gateway/MCP wiring, or live gameplay.
