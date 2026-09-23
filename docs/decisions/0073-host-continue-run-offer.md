# ADR 0073: Host-offered continuation beside `start_run`

Status: proposed; focused producer-boundary tests executed; native resume evidence pending.

## Requirement and owner

Refs sts2-game-mod#172. Game-mod owns the producer catalog and the existing runtime-v3 gameplay
admission/receipt lane. The harness is the consumer and already admits the two `continue_run`
shapes (AI-Ascension/sts2-harness#415, merge `551ec19d3e6b7dcf1634602f58a24ea5be9c9f4b`). No
ownership or execution-route change.

## Defect

`start_run` was the only way to enter a run, so an episode that found an existing compatible saved
run could not re-enter it; each new episode abandoned the previous one. No mod-owned action could
continue a saved run.

## Decision

Add `continue_run` to the runtime-v3 gameplay action catalogue as an additive producer extension.
The host offers it beside `start_run` only through `offer_continue_run`, which appends the
continuation when the current screen is `Setup`, the catalog already offers exactly one
`start_run`, and the native owner reported a compatible resumable run. An absent run and a present
but incompatible run add nothing and are not an error.

The producer shape is the harness-admitted envelope: `{"kind":"continue_run"}` or
`{"kind":"continue_run","run_id":...}`, with the host-generated identity `continue_run:<seq>` or
`continue_run:<seq>:<run_id>`. The optional discriminator is kept distinct from an omitted one: an
explicit JSON `null` is refused rather than folded into an absent `run_id`.

Detecting a saved run and reading its compatibility identity stay with the native owner. The mod
never opens a save, resolves a path, infers a run from a filename, or accepts a caller-supplied save
path. Dispatch reuses the existing admission, generation, catalog-membership and receipt paths
unchanged; the lane dispatches exactly the continuation the host offered and never substitutes
`start_run`.

## Compatibility and exclusions

This is a source-only, additive change. The neutral schema's `action_payload` oneOf is already
non-exhaustive (it omits `use_potion`), so no frozen artifact byte, digest, route or durable record
changes. `RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST` is unchanged.

This slice does not perform a native resume, prove that a resumed run continues the saved
progress, or replace the synthetic profile seam with a real save read. `continue_run` accepted and
settled through the fake host is not evidence that a native run resumed; that is the T3 evidence
gate and remains out of scope. The slice neither closes the referenced feature issue nor
establishes gameplay improvement.

## Verification

Focused component tests (`crates/game-mod/tests/runtime_v3_continue_run.rs`) exercise both admitted
shapes, the offer gate and the existing admission/receipt lane. Positive: a compatible run is
offered beside `start_run` and is never aliased onto it; the offered continuation dispatches
accepted-to-settled and the host is shown the continuation, not `start_run`. Negative: a `null`
discriminator, an extra field, a supplied `save_path`, a duplicate key, a blank/non-identity or
oversized discriminator, and a full malformed envelope are each refused before host dispatch; a
stale generation and an unoffered continuation are rejected with zero dispatch calls; and the
offer gate refuses a non-setup screen, a missing `start_run`, a duplicate identity, an invalid
continuation and a full catalog.

These establish source-linked producer behavior only. Native save detection, compatibility
classification and resumed-run progress remain unverified.

## Next boundary

T3: produce native resume evidence that a continued run actually re-enters the saved run under the
active content/version fence, and prove producer-to-harness equality of the offered shapes.
