# Owned live fixed-card-cost source checks

The `live-facts.yml` CI job builds the real native library and runs these source-linked probes:

~~~sh
cargo build --locked --offline --package sts2-game-mod-interop
dotnet run --project experiments/managed-rust-interop/live-facts-tests/LiveFactsProbe.csproj --configuration Release -- target/debug/libai_ascension_sts2_game_mod_native.so
dotnet run --project experiments/managed-rust-interop/live-card-capture-tests/LiveCardCaptureProbe.csproj --configuration Release
~~~

Use Rust 1.97.1 and .NET 9.0.317; perform locked fetch before offline checks if dependencies are
not cached. For disk-constrained source runners, `CARGO_PROFILE_DEV_DEBUG=0`,
`CARGO_PROFILE_TEST_DEBUG=0` and `CARGO_INCREMENTAL=0` reduce rebuildable output without changing
the pinned dependency graph or enabling a different rule/host implementation.

The cross-language probe exercises production binding association, managed ABI, canonical manifest
producer and facts codec. The capture probe uses original controlled native shapes against actual
capture/retention source. Rust tests exercise fixed authenticated routes, closed envelopes,
source-derived input ordering, unavailable values and ABI bounds. Run the full Rust workspace gates
and existing source-only managed probes as well; these focused probes do not replace them.

These checks do not establish exact-host API compatibility, native parity, consumer adoption or
parent-issue closure. No game installation, profile, provider or proprietary assembly is needed.
See [ADR 0084](decisions/0084-owned-live-fixed-card-cost-facts.md) for the boundary decision.
