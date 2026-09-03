# Product boundary

## Purpose

sts2-game-mod is the game-facing adapter for a future STS2 automation system. It will provide the
managed entry point, host/main-thread translation, authoritative local HTTP surface, and a narrow
Rust/native composition boundary.

## Consumers and authority

The STS2 host is authoritative for live state, legal actions, and mutation effects. The future
gateway is the direct service consumer of the mod's local HTTP API. The MCP server maps its own
protocol to gateway calls; the harness coordinates instances, model/provider experiments, and
artifacts. Neither may link to or bypass the mod's host boundary.

The host-independent core target owns semantic policy. The sts2-protocol target owns the shared
language- and transport-neutral `poc-v1` artifact. This target consumes a checked-in copy as inert
input and owns no MCP tool catalog, gateway lease, or model schema.

## Wave 1, Wave 2, and runtime load-smoke scope

Wave 1 established governance, policy-as-code, workflow gates, documentation, decision records,
and a target workspace for repository tooling plus the pre-existing interop experiment. Wave 2
adds non-empty target-owned Rust seams for a host port, bounded main-thread queue and dispatcher,
versioned ABI validation, a transport-free bounded HTTP adapter, and their composition. The minimal
POC additionally maps one copied artifact through a fake core port. Deterministic fake tests cover
those seams. A narrow runtime slice now adds the managed loader entry point, unique Rust companion
package, manifest, ABI smoke call, bounded runtime routes (default loopback), managed main-thread
queue, and manual staging path. It adds no game rules or gameplay mutation.

## Future scope

Later implementation may add broader host adapters, owner-local gameplay HTTP
routes, and host-specific lifecycle behavior when each has a project-owned requirement, consumer,
bounded module, and deterministic test seam. Host calls will remain explicit, serialized where
required, and reconciled with observed host state. The current ports and POC fake mapping do not
make those runtime claims.

## Non-goals

- Reimplementing game rules in the adapter.
- Putting HTTP, MCP, gateway lifecycle, or harness coordination into the managed loader.
- Remote-by-default exposure, discovery, fallback credentials, or unbounded payloads.
- Storing or distributing proprietary game files, assets, saves, profiles, or credentials.
- Compatibility with another harness implementation or unsupported game/platform versions.

## Evidence boundary

The current repository has confirmed load-smoke and focused runtime evidence for one exact installed
host. The initialized Rust seams and POC fake mapping do not prove gameplay mutation, settlement of
game rules, or fault isolation. See the dated runtime evidence reports for the precise boundaries.

## Runtime vertical slice

The first executable host-facing slice is intentionally a probe rather than a gameplay action. It
accepts the canonical `runtime-v1` state request and the fixed `show_runtime_probe` action through a
bearer-authenticated local adapter whose default bind address is loopback. The native listener owns
bounded HTTP decoding; the managed bridge owns host access and queues work to the Godot main thread.
An accepted action is reported only after a `CanvasLayer` status overlay is observed, and a stale
generation is rejected with `sts2.game-mod/stale_generation`. The built-in settings tab immediately
applies and persists the listener's enablement, local bind address, and port for future launches;
the bearer token
remains an environment-controlled secret.

This slice does not claim to change game rules, advance combat, or settle a gameplay mutation. The
source, build, and exact-host probe gates are `confirmed`; gameplay mutation, process lifecycle, and
broader compatibility remain outside the evidence.
