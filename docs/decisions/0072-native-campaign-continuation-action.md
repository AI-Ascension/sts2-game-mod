# ADR 0072: Native campaign continuation action

- Status: Accepted; source-linked synthetic checks executed; native resume evidence pending
- Date: 2026-09-23 (corrected 2026-09-28)
- Owner: sts2-game-mod

This record and [ADR 0073](0073-host-continue-run-offer.md) originally described the same
decision in two files. [ADR 0073](0073-host-continue-run-offer.md) is now the single normative
record; this document is retained as the identifier `sts2-game-mod#172` was first written against
and holds only the schema-revision provenance that 0073 does not restate. Where the two disagree,
0073 governs.

## Requirement and owner

Refs sts2-game-mod#172. Game-mod owns the host-thread catalog and dispatch lane. The consumer-first
admission is already merged in `sts2-harness` (PR #415, merge
`551ec19d3e6b7dcf1634602f58a24ea5be9c9f4b`), so the harness accepts the host-offered kind and
refuses everything else. No ownership or execution route changes here.

## Defect

A resumable standard campaign could not be continued: the host catalog offered only `start_run`, so
every model-driven episode started a new run even when the native profile already held a
`current_run.save`. The reported symptom is exactly "every episode starts over".

## Decision

Offer `continue_run` beside `start_run`. Detection reuses only the native-owner guards the standard
resume path already relies on: an initialized profile, a current profile in `1..3`, `HasRunSave`,
and no run already in progress. A failed or incomplete read is not resumable, so a broken host can
never fabricate a continuation. Practice runs and the map-bound fixture never offer it.

The offered action is argument-free, `{"kind":"continue_run"}`, which is the accepted wire shape for
a single detected run. The catalog and codec also admit the discriminated shape
`{"kind":"continue_run","run_id":"..."}` with an identity-only `run_id`; the host never emits a null
discriminator, an extra field, or a save path. Naming a specific run is not offered and not
dispatched, because resolving a run discriminator to a save would need host authority this boundary
does not hold; such a request is refused rather than mapped onto another action.

Dispatch reuses the existing admission and receipt paths. The continuation is admitted only when it
is the exact generation-bound action the host currently offers, then invokes the existing
`LiveCampaignStart.ResumeAsync` native resume seam. Settlement requires the ordinary independent
witness: an advanced generation, a non-setup successor state, and the `campaign_resumed` effect. A
synchronous failure stays unknown, and a pending receipt fences overlapping operations, so a second
resume cannot run concurrently.

## Schema-revision provenance (corrected 2026-09-28)

An earlier revision of this record stated that the normative `runtime-v3-gameplay` schema, its
package copy, the `SHA256SUMS`/manifest digests and the Rust contract mirror were **not** changed
and that the schema arm was out of scope. That was wrong on the evidence now on `main`:

- `73979e6` ("mirror the runtime-v3-gameplay `continue_run` schema revision", PR #212) re-vendored
  the artifact from its producer (`sts2-protocol` PR #61) so the mod-side parser admits
  `continue_run` under a schema that actually defines the arm.
- That commit is an **explicit incompatible artifact revision** under ADR 0012. The schema digest
  moved from `8e99cea36b7ede97532348fd8efe302ca79260895265a7bf14ddf7e006d8ff63` to
  `daa216902d3211b9537924105b27e7718dd93dec82969a3c550131a27147c06b`.
- `continue_run` is now a defined arm in `protocol-artifact/runtime-v3-gameplay/schema.json` and
  `schemas/runtime-v3-gameplay.schema.json`, with the matching conformance case and golden.
- The current Rust constant `RUNTIME_V3_GAMEPLAY_SCHEMA_DIGEST` is
  `daa216902d3211b9537924105b27e7718dd93dec82969a3c550131a27147c06b`, and both
  `protocol-artifact/runtime-v3-gameplay/SHA256SUMS` entries agree.

The earlier "no frozen byte changed" claim also survived only because ADR 0073 was written first
and asserted the same false invariant; the two records had to be reconciled against the artifact
rather than against each other.

This record does not claim a resumed native game or any game effect. Native resumable-run detection
and native resume evidence remain `unverified` and need the authorized native host.

## Validation

The managed source-only probe drives the real catalog, codec, request validation, dispatch and
receipt composition: both accepted continuation shapes are admitted, a save path, a null, blank, or
non-identity `run_id`, an extra field, a continuation target, a duplicate identity and an unowned
kind are refused before any mutation, a continuation identity with a `start_run` payload is not
coerced, replay does not dispatch twice, and a read-only wait settles an independently completed
continuation. No native host, profile, save, or process is read by these checks.
