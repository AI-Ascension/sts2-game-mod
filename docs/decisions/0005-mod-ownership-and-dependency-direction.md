# ADR 0005: Mod ownership and dependency direction

- Status: Accepted for foundation and future implementation
- Date: 2026-09-02

## Context

The mod boundary can become a catch-all when host access, HTTP, lifecycle, and protocol concerns
are introduced together. That would make thread affinity, authority, and independent testing
unclear.

## Decision

sts2-game-mod owns managed loader metadata, host translation, main-thread dispatch, authoritative
host state/mutation observation, the owner-local HTTP adapter, and composition of those pieces.
The host is the authority for live state and effects. The gateway owns lifecycle, leases, routing,
isolation, and control-plane policy. The MCP server is a thin gateway adapter. The harness owns
coordination and artifacts. The core target owns host-independent semantic policy.

Dependencies point inward toward stable contracts:

~~~text
protocol (if accepted) <- core <- mod host boundary
                              ^        ^
                              |        +-- owner-local HTTP adapter
                              +-- gateway control plane
                                      ^
                                 MCP server
                                 harness
~~~

The mod must not depend on gateway internals, MCP implementation, harness model code, or
proprietary host source. A shared dependency requires an explicit owner and compatibility decision.

## Consequences

Main-thread and host authority remain visible at one boundary. HTTP and host tests can use fakes,
and the future gateway can consume a stable adapter without reaching through game objects. Moving
ownership or reversing dependency direction requires a superseding ADR.
