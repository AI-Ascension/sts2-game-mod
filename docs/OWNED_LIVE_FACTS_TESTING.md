# Owned live facts source checks

[ADR 0084](decisions/0084-owned-live-fixed-card-cost-facts.md) defines the production boundary and
evidence limits. Run from the root with pinned Rust 1.97.1 and .NET 9.0.317:

```sh
cargo test --locked --offline --package sts2-game-mod --test owned_live_facts
cargo test --locked --offline --package sts2-game-mod-interop game_facts
cargo build --locked --offline --package sts2-game-mod-interop --lib
dotnet build experiments/managed-rust-interop/game-facts-tests/GameFactsProbe.csproj --configuration Release
DOTNET_SYSTEM_GLOBALIZATION_INVARIANT=1 dotnet run --project experiments/managed-rust-interop/game-facts-tests/GameFactsProbe.csproj \
  --configuration Release --no-build -- "$PWD/target/debug/libai_ascension_sts2_game_mod_native.so"
```

The route test requires an owned ephemeral loopback listener; refusal to bind is an environment
failure, not a passing test. On Windows supply the compiled native DLL path. The host-independent
probe does not load a game or proprietary assembly.

| Boundary | Observable checks |
| --- | --- |
| Production capture | Raw energy/cost above 255; negative values stay unknown; absent fixedness; getter failure; generation changes during capture |
| Retention | No epoch advancement; invalid identity, queue failure, incomplete callback, failed invalidation and failed recheck cannot expose older values |
| Native ownership | Same-seed run replacement, same-definition card replacement, profile-slot change and same-slot profile replacement invalidate |
| Actual managed/native seam | Canonical codec and manifest producer; Core input order; owner-derived build/mode; known unsourced rules unsupported; wrong correlation/duplicate keys rejected before reads |
| Authorization/content | Different lease, revoked authorization, wrong Harness run/visibility, changed catalog generation/semantics/locale and state changes during catalog reads refuse |
| Rust response | Zero and signed-32-bit values preserved; unavailable/X costs never defaulted; query order, all Core members, unknown IDs and binding mismatches |
| ABI/listener | Input length/output capacity refusals; callback 26; bearer authentication, exact methods, JSON requirement and no arbitrary route |

The controlled host and catalog are original synthetic values. Linking the actual capture methods
does not qualify an installed host API. An unchanged controlled RNG draw counter proves only that
this path did not call that fixture's draw API; native getter side effects, cost parity and all
lifecycle transitions remain unverified.

Run the complete build, format, strict policy, Clippy and tests from [AGENTS.md](../AGENTS.md),
strict Rust documentation, existing managed source/Workshop checks, and affected host-candidate,
contract, live-card-source and game-information probes. The game-information conformance workflow
builds/runs the facts probe against the same native library. Protocol checksum discovery remains
unchanged.

Host-loader packaging needs the authorized exact `sts2.dll`, `GodotSharp.dll` and Harmony
prerequisite documented by the loader project. Without them, packaging and controlled native
acceptance remain unverified. Gateway/MCP/Harness adoption and replay are separate required
engineering work for parent #207.
