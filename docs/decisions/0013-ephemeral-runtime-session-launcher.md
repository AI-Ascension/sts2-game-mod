# ADR 0013: Ephemeral authenticated runtime-session launcher

- Status: Accepted for the managed runtime proof lane
- Date: 2026-09-02

## Context

The addon listener is intentionally token-gated, but a WSL shell export does not automatically
become the environment of a Windows game launched through the normal desktop boundary. Starting
the game, gateway, harness, and MCP independently also makes it easy to reuse a token, leave a
listener running, kill an unrelated process, or accidentally put a secret in an argument or log.
The saved Runtime API setting must remain user-owned and the provider implementations remain in
their separate targets.

## Decision

Add `experiments/managed-rust-interop/session-launcher.sh` as a development/test orchestrator.
For each launch it generates a 48-byte hex credential with the operating system CSPRNG. The
runtime credential is used as `STS2_RUNTIME_TOKEN` in the game and `STS2_MOD_TOKEN` at the gateway;
a different credential is used as `STS2_GATEWAY_TOKEN` at the gateway and harness/MCP chain. The
credentials exist only in launcher memory, child environments, and the bridge stdin pipe.

The launcher refuses an existing `SlayTheSpire2.exe` before building, installing, or generating a
session. It uses a checked-in .NET bridge with `UseShellExecute=false`; the bridge receives the
runtime credential from stdin and sets the game's inherited `STS2_RUNTIME_TOKEN`, bind address,
port, and non-secret `STS2_RUNTIME_SESSION=1`. That flag is an ephemeral launch override for a
saved-off Runtime API setting and does not write settings. Provider binaries are explicit inputs or
are built from explicit source directories; this target does not edit gateway, harness, or MCP.

The default endpoint is loopback and the launcher bounds provider/game readiness and harness
completion. It first verifies unauthenticated rejection, then authenticated game and gateway
readiness, runs the existing harness/MCP trace, and terminates only recorded child process groups
and the recorded Windows game PID. A successful one-shot run verifies both listeners are closed.
`--keep-alive` is available for manual inspection and remains subject to the same owned cleanup.

## Consequences

The runtime proof can be launched as one disposable session without a second settings-framework
mod or a persisted credential. The output surface is boolean-only. Synthetic tests cover credential
properties, role mapping, authorization outcomes, process ownership, timeout behavior, argument
leakage, the already-running guard, and the stdin bridge contract.

This remains a development/test lane, not a production supervisor or a replacement for gateway,
harness, or MCP lifecycle ownership. A live result is valid only when the exact external provider
revisions and disposable game host are recorded separately. The launcher cannot prove provider
implementation correctness or gameplay compatibility by itself.

## Security boundary

No credential is accepted as a command-line value or written to a file, `.env`, URL, Steam option,
log, screenshot, or CI artifact. Build and provider output is suppressed by the launcher. The
launcher does not inspect or copy saves, profiles, host assemblies, or private provider logs.
