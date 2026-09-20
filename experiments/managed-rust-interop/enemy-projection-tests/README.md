# Native enemy projection regression probe

This is a source-linked, host-free test, not a native gameplay fixture. It links the real
`../game-loader/LiveCombatSource.IntentProjection.cs`, `RuntimeV3GameplayObservation.cs`
and `RuntimeV3GameplayContract.cs`. It supplies only original external API-shape doubles
and the unrelated native partial's identity/clamping helpers. No game assembly, provider,
HTTP service, private prompt, RNG state, save, profile or hidden game data is used.

## Run

From the repository root with the pinned .NET SDK:

```sh
dotnet run --project experiments/managed-rust-interop/enemy-projection-tests/EnemyProjectionProbe.csproj --configuration Release
```

The 32 cases include genuine empty/zero observations, owned values and ordering, repeated
reads, required enumeration/getter failures, sanitized exceptions, collection bounds,
duplicate identities, invalid required values, unclamped health bounds, and preserved
unknown/visible intent behavior. The oversized source is finite so testing the pre-fix
implementation cannot allocate forever.

The pre-fix source compiled and failed the overlarge-enumeration assertion; a build failure
would not count as that red-test result. With the correction, all 32 checks passed on Linux
using .NET SDK 9.0.317 during integration on 2026-09-20.

Full managed/Rust/policy gates remain required. This probe does not exercise a real host,
thread scheduling, v4 HTTP errors, generation fencing, cross-repository propagation, or
gameplay quality. See ADR 0071 for the exact scope and remaining gates.
