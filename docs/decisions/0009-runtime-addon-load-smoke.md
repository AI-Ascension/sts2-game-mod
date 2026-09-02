# ADR 0009: Runtime addon load-smoke package

- Status: Accepted for the managed load-smoke slice
- Date: 2026-09-02

## Context

The earlier interop experiment proved only source-level ABI shape. A game addon is not implemented
until the host can discover its manifest, invoke its managed initializer, and load the paired native
library in the exact installed game. The host assembly remains proprietary and must stay outside the
repository and package.

## Decision

Promote the narrow loader seam into the `AIAscensionSTS2Poc` package. Its managed .NET 9 entry point
uses the host's `ModInitializer` metadata, loads the adjacent uniquely named
`ai_ascension_sts2_poc.dll`, verifies ABI version `1`, calls the checked-add export with `19` and
`23`, and logs a bounded success marker only when the result is `42`. It then adds a top-layer Godot
status banner with the same verified values, giving an operator a visible in-game confirmation.
The package script stages exactly the managed DLL, native DLL, and manifest for an authorized
Windows x86-64 game test.

The package does not expose HTTP, inspect or mutate game state, own game rules, or replace the
gateway/MCP/harness boundaries. Load-smoke evidence is recorded separately from Rust and fake-POC
tests and is never promoted to gameplay or broad compatibility evidence.

## Consequences

The target now has a real loader-facing artifact and a reproducible package path while retaining the
Rust-first boundary. The exact host assembly is used only as a local build reference; it is neither
stored nor redistributed. Gameplay, main-thread dispatch, HTTP serving, action effects, and host
lifecycle remain separate implementation and test work.
