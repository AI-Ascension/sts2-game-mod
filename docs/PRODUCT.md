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

## Wave 1 and Wave 2 scope

Wave 1 established governance, policy-as-code, workflow gates, documentation, decision records,
and a target workspace for repository tooling plus the pre-existing interop experiment. Wave 2
adds non-empty target-owned Rust seams for a host port, bounded main-thread queue and dispatcher,
versioned ABI validation, a transport-free bounded HTTP adapter, and their composition. The minimal
POC additionally maps one copied artifact through a fake core port. Deterministic fake tests cover
those seams. This remains source-only preparation: it adds no game rules, public route catalog,
managed loader, host callback implementation, game package, or placeholder crate.

## Future scope

Later implementation may add managed loader composition, a real host adapter, owner-local HTTP
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

The current repository state is source-prepared and runtime-unverified. The initialized seams and
existing interop experiment are not a product package and do not prove loader discovery, game
start, main-thread execution, HTTP serving, mutation settlement, or fault isolation.
