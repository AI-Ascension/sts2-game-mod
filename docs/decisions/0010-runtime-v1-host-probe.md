# ADR 0010: `runtime-v1` host-visible probe

- Status: Accepted for the bounded vertical slice; exact-host execution confirmed by dated probe
- Date: 2026-09-02

## Context

The load-smoke package proves managed discovery and a native ABI call, but no external request has
reached host code. The next useful seam must prove authenticated local intake, managed/native
translation, main-thread dispatch, and an observable host effect without inventing a game-rule
action or bypassing gateway authority.

## Decision

Add a loopback-only native HTTP listener with bounded headers, bodies, responses, and identity values.
It requires `STS2_RUNTIME_TOKEN` and admits only `/health/ready`,
`/api/v1/runtime/state`, and `/api/v1/runtime/action`. It calls a versioned C ABI callback using
borrowed pointer/length values whose lifetime ends with the callback.

The managed bridge copies request data into owned values, queues at most the bounded pump workload,
and processes it on Godot `SceneTree.ProcessFrame`. State returns the current `runtime-v1`
observation. The only action is `show_runtime_probe`: it creates the existing status overlay,
requires the overlay to be observable, advances generation, and returns a
`status_overlay_visible` witness. A stale generation returns HTTP 409 and
`sts2.game-mod/stale_generation` without creating a second witness.

The game host remains authoritative. Gateway authentication, leases, and fencing remain outside this
adapter; the listener's bearer token and identity checks are defense-in-depth. The action does not
advance combat or mutate gameplay state.

## Consequences and evidence

The managed action parser validates the complete closed request shape, provenance/action closure,
duplicate-property absence, required null fields, bounded identities/numbers, and body/header
epoch agreement before effects. Invalid callback context is an owner-local HTTP 400 transport
error; state errors are not mislabeled canonical state responses (which require null error_code).
The canonical observation limit of 1024 probe actions is enforced by pre-effect rejection.

Safety correction: managed admission holds at most 64 pending requests and processes at most 16
per frame. A five-second wait removes pending work atomically before execution can claim it.
If execution has already begun, the timeout remains an unknown outcome; late completion cannot
overwrite the published response. Admission failures use HTTP 503 and owner-local JSON errors;
timeouts use HTTP 504 with `main_thread_timeout_before_dispatch` or `main_thread_outcome_unknown`.
An execution exception also remains unknown because a host effect may already have happened.
These transport failures are not canonical `runtime-v1` rejected action envelopes. The immutable
artifact and native ABI are unchanged. Consumers must not retry mutations solely from a timeout.
The source-linked managed queue probe tests synthetic races and does not extend dated host evidence.

The native listener and managed bridge are source/build-confirmed, and consumers use the copied
`protocol-artifact/runtime-v1/` artifact. The authorized probe confirmed a real listener request,
host callback, main-thread execution, visible effect, disposable profile, and reversible cleanup for
STS2 v0.107.1 on Windows x86-64. Gameplay mutation, process supervision, and other compatibility
axes remain outside this evidence. See
[`../evidence/runtime-v1-host-live-20260902.md`](../evidence/runtime-v1-host-live-20260902.md).
