# ADR 0002: Initial game compatibility baseline

- Status: Accepted as the initial planning target; target runtime evidence outstanding
- Date: 2026-09-02

## Context

Host compatibility must be stated for an exact game and platform. A prior implementation or a
moving beta build is not a support promise for this greenfield target. The project planning
baseline identifies one stable target for the first compatibility work.

## Decision

The initial target is:

| Dimension | Value |
| --- | --- |
| Game | STS2 v0.107.1 |
| Game commit | 59260271 |
| Platform | Windows x86-64 |
| Host assembly | Operator-supplied and kept outside this repository |
| Behavioral contract | Project-owned requirements and fixtures |

Earlier versions, beta versions, Linux, macOS, and other architectures are unverified until they
receive exact matrix rows and evidence.

## Promotion evidence

Support moves through build-only, load smoke, focused runtime, and full conformance levels in
[COMPATIBILITY.md](../COMPATIBILITY.md). A promotion records game and assembly identity, platform,
runtime, source and artifact hashes, disposable profile, setup, observed results, cleanup, and
date. CI must not download or redistribute proprietary host assemblies.

## Consequences

Host adaptation can focus on one explicit baseline while HTTP compatibility stays independent.
The absence of a local host file or runtime test is an unverified gate, not evidence of support.
