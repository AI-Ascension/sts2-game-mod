# ADR 0012: Configurable runtime listener settings

- Status: Accepted for the built-in settings/runtime-controls slice
- Date: 2026-09-02

## Context

The managed addon owns an authenticated runtime listener for its bounded host probe. The initial
runtime slice fixed that listener to `127.0.0.1:15526` and accepted configuration only through
environment variables. Users testing the addon need a discoverable in-game way to choose a local
port and a local bind address without installing a second settings-framework mod.

The bearer token is an authentication secret and must not be copied into or edited through the game
settings UI. Binding to all interfaces is also a deliberate network exposure decision and must be
clearly labeled.

## Decision

Add a Runtime connection section to the standalone AI-Ascension settings tab with:

| Setting | Behavior |
| --- | --- |
| Runtime API | Enables or disables the authenticated listener on the next launch; the default is enabled for backwards-compatible token-gated behavior. |
| Bind address | Dropdown containing loopback (`127.0.0.1`), all interfaces (`0.0.0.0`), the detected machine hostname, and detected local IPv4 addresses. |
| Network port | Validated numeric value from `1024` through `65535`, defaulting to `15526`. |

The values are staged in the panel, persisted by the Apply action in the addon-owned user-data
settings file, and take effect on the next game launch. Reset restores the enabled default,
loopback, and port `15526`. `STS2_RUNTIME_PORT` and `STS2_RUNTIME_BIND_ADDRESS` remain explicit
environment overrides for automation. `STS2_RUNTIME_TOKEN` remains environment-controlled and is
still required before the listener starts.

The managed/native ABI passes the selected bind address and port to the Rust listener. Rust resolves
the selected address locally, binds the first usable socket address, stores the actual bound socket
address for shutdown wake-up, and keeps the existing bounded HTTP, identity, and bearer-token
checks. An invalid address or port prevents listener startup without preventing the addon load-smoke
or settings tab from operating.

## Consequences

The default remains loopback-only behavior, while intentional testers can select a local network
interface or hostname from the same tab. Selecting all interfaces can expose the listener beyond
the local machine; the UI warns that a trusted network and firewall configuration are required.
Listener settings are configuration for startup, so the UI explicitly says that a restart is
required for a changed value to become active.

The runtime health response reports a configured listener rather than claiming that every listener
is loopback. Existing exact-host evidence remains evidence for the default loopback configuration;
it does not prove every address, hostname, firewall, or platform combination.

## Boundaries

This change does not add token editing, arbitrary free-form network input, new HTTP routes, MCP
actions, gameplay mutation, or a second settings framework. The native Installed Mods checkbox
remains the sole enablement control. Environment overrides are retained for scripts and controlled
runtime probes.
