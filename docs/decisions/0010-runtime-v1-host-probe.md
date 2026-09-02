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

The native listener and managed bridge are source/build-confirmed, and consumers use the copied
`protocol-artifact/runtime-v1/` artifact. The authorized probe confirmed a real listener request,
host callback, main-thread execution, visible effect, disposable profile, and reversible cleanup for
STS2 v0.107.1 on Windows x86-64. Gameplay mutation, process supervision, and other compatibility
axes remain outside this evidence. See
[`../evidence/runtime-v1-host-live-20260902.md`](../evidence/runtime-v1-host-live-20260902.md).
