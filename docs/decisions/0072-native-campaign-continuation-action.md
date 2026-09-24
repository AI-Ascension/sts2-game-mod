# ADR 0072: Native campaign continuation action

- Status: Proposed; source-linked synthetic checks executed; native resume pending
- Date: 2026-09-23
- Owner: sts2-game-mod

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

## Compatibility and exclusions

The change is additive. The normative `runtime-v3-gameplay` schema and its package copy, the
`SHA256SUMS`/manifest digests, and the Rust contract mirror are **not** changed here: those bytes are
the protocol-owned shared contract that `sts2-harness` and the watchdog fault-fixture also vendor,
and the merged consumer admitted the kind without an artifact revision. Adding the `continue_run`
arm to the schema and repinning the digest is a coordinated protocol-owner action that must land
with every consumer copy and is out of scope for this lane.

This record does not claim a resumed native game or any game effect. Native resumable-run detection
and native resume evidence remain `unverified` and need the authorized native host.

## Validation

The managed source-only probe drives the real catalog, codec, request validation, dispatch and
receipt composition: both accepted continuation shapes are admitted, a save path, a null, blank, or
non-identity `run_id`, an extra field, a continuation target, a duplicate identity and an unowned
kind are refused before any mutation, a continuation identity with a `start_run` payload is not
coerced, replay does not dispatch twice, and a read-only wait settles an independently completed
continuation. No native host, profile, save, or process is read by these checks.
