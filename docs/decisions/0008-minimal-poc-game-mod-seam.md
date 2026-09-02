# ADR 0008: Minimal POC game-mod seam

- Status: Accepted for the deterministic POC
- Date: 2026-09-02

## Context

The six-target proof of concept needs the game-facing boundary to consume a built, release-like
protocol artifact while keeping target ownership explicit. The mod target must not gain a protocol
implementation dependency or pretend that a fake core is a live STS2 host.

## Decision

Copy `sts2-protocol/poc-v1` into `protocol-artifact/poc-v1` and verify its manifest, schema
identity, and fixtures locally. Map state reads and the typed `use_budget` action in a small
`PocMod<C>` through a `PocCorePort`. Preserve protocol metadata, correlation, instance, and
generation fields; return a bounded observation, stable core error identity, and one effect witness
only after an accepted fake-core transition.

The mapping uses no cross-repository path dependency, network, credential, game file, or host
assembly. It is exercised by deterministic tests with a fake core.

## Consequences

The mod boundary has a reviewable source/test seam for the requested fake vertical slice. The
artifact copy makes lineage and offline validation explicit. Host loading, main-thread execution,
HTTP serving, actual game mutation, and runtime settlement remain proposed or unverified and need
separate authorized evidence.
