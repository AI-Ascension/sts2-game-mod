# Repository layout

## Current target tree

~~~text
sts2-game-mod/
├── managed/loader                    future host-specific managed integration surface
├── crates/host                       host port, bounded queue, dispatcher, ABI gate
├── crates/http-adapter               bounded transport-free HTTP adapter seam
├── crates/game-mod                   target-local composition root
├── protocol-artifact/poc-v1           offline copy consumed by the POC mapping
├── protocol-artifact/runtime-v2       offline copy consumed by the Runtime-v2 fake seam
├── schemas/poc-v1.schema.json         copied protocol source for artifact checksums
├── schemas/runtime-v2.schema.json     copied Runtime-v2 source schema for artifact checksums
├── conformance/cases/poc-v1.json      copied protocol conformance evidence
├── conformance/cases/runtime-v2.json  copied Runtime-v2 conformance evidence
├── schemas/game-http-v1              future owner-local HTTP schema
├── tests                             future component and host-test seams
├── experiments/managed-rust-interop/ managed load-smoke package and Rust companion source
├── tools/workshop/                  deterministic Workshop staging and fixture checks
├── tools/repo-policy/                target-local governance checker
├── docs/                             architecture, policy, testing, and decisions
└── .github/                          bounded read-only automation
~~~

The initialized crates contain only boundary ports, bounded admission, and deterministic fake
tests. `managed/loader`, `schemas/game-http-v1`, and `tests` remain future responsibility markers;
they must not receive empty manifests or speculative source. The `poc-v1` schema and conformance
case are inert, protocol-owner bytes copied at the repository-relative paths required by the
artifact's exact checksum inventory.

## Workspace

The root Cargo workspace contains the target-local repo-policy tool, the three initialized seam
crates, and the native companion of the managed-rust-interop runtime addon. The product crates use
only target-local path dependencies. The managed loader remains an ordinary .NET project under
the existing experiment directory and is packaged only by the explicit load-smoke command.

## Ownership map

| Path | Owner | Consumer or use |
| --- | --- | --- |
| managed/loader | mod | STS2 loader and host callbacks |
| crates/host | mod | mod composition and managed/host integration seam |
| crates/http-adapter | mod | mod composition; future owner-local HTTP listener |
| crates/game-mod | mod | target-local composition and admission boundary |
| protocol-artifact/poc-v1 | protocol release consumer | inert copied POC artifact and fixtures |
| protocol-artifact/runtime-v2 | protocol release consumer | inert copied Runtime-v2 artifact and fixtures |
| schemas/poc-v1.schema.json | protocol release consumer | inert copied source schema for checksums |
| schemas/runtime-v2.schema.json | protocol release consumer | inert copied Runtime-v2 source schema for checksums |
| conformance/cases/poc-v1.json | protocol release consumer | inert copied conformance evidence |
| conformance/cases/runtime-v2.json | protocol release consumer | inert copied Runtime-v2 conformance evidence |
| schemas/game-http-v1 | mod | HTTP adapter and conformance |
| tools/repo-policy | repository maintainers | local and CI policy gates |
| tools/workshop | mod and release maintainers | first-party Workshop package staging and checks |
| experiments/managed-rust-interop | mod | managed loader package, native companion, and load-smoke staging |

The protocol target, core target, gateway, MCP server, and harness remain separate trees. Do not
move their implementation or evidence into this target.

## Generated and private material

target, bin, obj, artifacts, coverage, host assemblies, saves, profiles, credentials, and local
environment files are ignored or prohibited. Existing generated experiment output is preserved as
local output and is not source or release material.

## Naming authority

Shared naming is defined by the aggregate NAMING_CONVENTIONS.md and naming-registry.yaml. The
package, crate-import, managed namespace, and native export names in this target use the
`sts2-game-mod` owner prefix; host-defined names and consumed contracts remain exact exceptions.
