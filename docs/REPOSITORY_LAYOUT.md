# Repository layout

## Current target tree

~~~text
sts2-game-mod/
├── managed/loader                    future managed loader boundary
├── crates/host                       host port, bounded queue, dispatcher, ABI gate
├── crates/http-adapter               bounded transport-free HTTP adapter seam
├── crates/game-mod                   target-local composition root
├── protocol-artifact/poc-v1           offline copy consumed by the POC mapping
├── schemas/game-http-v1              future owner-local HTTP schema
├── conformance                       future deterministic boundary fixtures
├── tests                             future component and host-test seams
├── experiments/managed-rust-interop/ preserved source-only experiment
├── tools/repo-policy/                target-local governance checker
├── docs/                             architecture, policy, testing, and decisions
└── .github/                          bounded read-only automation
~~~

The initialized crates contain only boundary ports, bounded admission, and deterministic fake
tests. `managed/loader`, `schemas`, `conformance`, and `tests` remain future responsibility
markers; they must not receive empty manifests or speculative source.

## Workspace

The root Cargo workspace contains the target-local repo-policy tool, the three initialized seam
crates, and the already-existing native member of the managed-rust-interop experiment. The product
crates use only target-local path dependencies. The managed projects remain ordinary .NET projects
under the experiment until a later implementation decision gives them a product home.

## Ownership map

| Path | Owner | Consumer or use |
| --- | --- | --- |
| managed/loader | mod | STS2 loader and host callbacks |
| crates/host | mod | mod composition and managed/host integration seam |
| crates/http-adapter | mod | mod composition; future owner-local HTTP listener |
| crates/game-mod | mod | target-local composition and admission boundary |
| protocol-artifact/poc-v1 | protocol release consumer | inert copied POC artifact and fixtures |
| schemas/game-http-v1 | mod | HTTP adapter and conformance |
| tools/repo-policy | repository maintainers | local and CI policy gates |
| experiments/managed-rust-interop | mod research | build-level ABI investigation only |

The protocol target, core target, gateway, MCP server, and harness remain separate trees. Do not
move their implementation or evidence into this target.

## Generated and private material

target, bin, obj, artifacts, coverage, host assemblies, saves, profiles, credentials, and local
environment files are ignored or prohibited. Existing generated experiment output is preserved as
local output and is not source or release material.

## Naming authority

Shared naming is defined by the aggregate NAMING_CONVENTIONS.md and naming-registry.yaml. The
package,
crate-import, managed namespace, and native export names in this target use the `sts2-game-mod`
owner prefix; host-defined names and consumed contracts remain exact exceptions.
