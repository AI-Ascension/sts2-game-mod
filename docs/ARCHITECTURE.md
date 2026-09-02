# Architecture

## Responsibility

sts2-game-mod is the game-facing translation boundary. It adapts the managed loader and host
callbacks into owned Rust values, schedules host work on the game main thread, exposes the
authoritative local HTTP surface, and composes the narrow native seam.

This document records boundaries, the initialized source-level seams, and dependency direction.
The current Rust source proves deterministic port composition plus one local fake `poc-v1` mapping;
it does not claim a managed loader or game runtime path.

## Initialized source seam

`crates/host` contains the target-owned `HostPort`, owned request and snapshot projections, a
bounded FIFO main-thread queue, a deterministic dispatcher, and a versioned ABI descriptor gate.
`crates/http-adapter` accepts an already-framed request view and enforces a body bound without
opening a socket or defining a route catalog. `crates/game-mod` composes those ports so an HTTP
admission is queued before a host pump can submit it. Fake host, queue, ABI, adapter, and
composition tests make those source-level invariants deterministic.

This seam is intentionally opaque and preparatory. It is not the managed loader, a public HTTP
schema, a game-rule implementation, or a native implementation copied from another repository.

## Minimal POC mapping

`protocol-artifact/poc-v1/` is a checked-in, release-like copy produced by the protocol owner. The
mod verifies its manifest and schema identity locally, then maps the four message shapes without a
cross-repository dependency. `PocMod` accepts only a state request or an action request, forwards
the typed `use_budget` action through `PocCorePort`, and returns the bounded observation with the
same protocol version, schema digest, correlation ID, instance ID, and generation. An accepted
action creates one `EffectWitness`; a core rejection returns its stable error identity and creates
no witness.

The test's core implementation is a fake. The mapping is source/test evidence for the boundary,
not evidence of host callbacks, a local listener, game mutation, or settlement in STS2.

## System boundaries

The target crosses four distinct boundaries:

1. The managed loader boundary supplies the host-required metadata and keeps native-library
   lifetime explicit.
2. The host boundary translates host objects, callbacks, and thread affinity into owned values
   and commands.
3. The HTTP boundary decodes bounded requests, maps them to the host/application ports, and
   returns sanitized responses and errors.
4. The native seam carries only reviewed, versioned, transport-neutral ABI values.

The host remains authoritative for legal game state and mutation effects. An accepted request is
not reported as completed until the host reaches the corresponding result.

## Component ownership

| Component | Owns | Must not own |
| --- | --- | --- |
| managed loader | load metadata, callback entry, native lifetime, ABI translation | domain rules, HTTP, MCP, persistence |
| host boundary | host adaptation, main-thread dispatch, lifecycle observation | public MCP or gateway policy |
| HTTP adapter | local route decoding, response mapping, bounded errors | host object internals or gateway leases |
| game-mod composition | wiring of target-local components | a second domain or transport implementation |
| core target | host-independent semantics and policy | HTTP, host APIs, processes, filesystems |
| gateway target | lifecycle, routing, leases, isolation, control plane | game rules or host ABI |
| MCP target | thin MCP-to-gateway translation | host access, game rules, lifecycle ownership |
| harness target | coordination, explicit instance context, experiments, artifacts | game authority or wire reinterpretation |
| protocol target | only approved shared language/transport-neutral contracts | target-specific host or transport behavior |

## Dependency direction

The intended direction is:

~~~text
protocol (if accepted) <- core <- mod host boundary
                              ^        ^
                              |        +-- HTTP adapter
                              +-- gateway
                                    ^
                               MCP server
                               harness
~~~

The diagram is a boundary map, not permission to add dependencies now. The mod must not depend
on MCP, gateway control-plane internals, harness model code, or proprietary host source. A future
protocol dependency must be justified by a genuinely shared contract and a recorded owner.

## Main-thread and lifecycle invariants

- Host reads and mutations execute on the game main thread.
- Network intake may be off-thread, but queued host work has bounded capacity and explicit overload.
- Accepted work resolves to success, failure, or cancellation; it is never silently dropped.
- FIFO ordering, cancellation before and after acceptance, timeout meaning, and shutdown behavior
  are explicit contract decisions.
- Host calls do not occur while holding unrelated locks.
- Native unload is not assumed safe until a host lifecycle test proves it.
- Thread, callback, pointer, lifetime, allocator, and panic assumptions are documented at the ABI.

## Protocol and data rules

The owner-local HTTP contract remains separate from the copied POC artifact. The current adapter
owns only a bounded request view and admission status; its schema, wire names, route catalog,
errors, ordering, and versioning require later project-owned requirements and fixtures. MCP
envelopes and gateway leases remain outside this repository.

Host objects never cross the HTTP or native boundary. Convert them to owned, validated values;
never expose debug strings, panic text, private paths, save contents, or raw host references.

## Evidence status

The existing experiment provides source-level shape for managed-to-native calls. It does not prove
game discovery, loader start, host API compatibility, main-thread behavior, HTTP serving, or safe
fault isolation. Those require later tests in an authorized disposable environment.
