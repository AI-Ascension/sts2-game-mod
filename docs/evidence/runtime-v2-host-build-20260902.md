# Runtime-v2 host adapter build evidence

> Historical author-reported build evidence, not validation of the repaired PR head. The
> 2026-09-04 review found incorrect settlement inference and incomplete idempotency/freshness
> protection. Current code retains unknown outcomes pending independent completion evidence;
> these package hashes must not be used to claim validation of the corrected source. The v2
> symbol narrative also predates the committed `EndPlayerTurnAction` enqueue implementation.

- Date: 2026-09-02
- Evidence level: `L1` target-local source/build/package evidence
- Status: `confirmed` for the named build oracles; live host behavior remains `unverified`
- Source base: game-mod remote `main` `f5cc07b6d0f0ef89bd06ea9378e39aa93e82a405` plus the
  uncommitted isolated candidate changes
- Runtime profile: `sts2-protocol/runtime-v2`
- Schema digest: `f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2`

## Host reference used for build

| Field | Value |
| --- | --- |
| Game | STS2 v0.107.1 |
| Host release label | `59260271` |
| `sts2.dll` SHA-256 | `a1f9e653f1e28e4076558fee1e60d218619cb7e057b887c6417f62c62c6d7a52` |
| Platform | Windows x86-64 |
| Managed runtime | .NET 9 host; .NET SDK `10.0.204` used for build |
| Native target | `x86_64-pc-windows-gnu`; Rust `1.97.1` |

The proprietary host assembly was used only as an operator-local build and reflection input. It is
not stored, copied, or packaged by this repository.

## Verified common host symbols

Read-only reflection and compiler resolution confirmed the v2 candidate uses only symbols available
in the recorded host:

```text
MegaCrit.Sts2.Core.Combat.CombatManager.Instance
MegaCrit.Sts2.Core.Combat.CombatManager.IsInProgress
MegaCrit.Sts2.Core.Combat.CombatManager.IsEnemyTurnStarted
MegaCrit.Sts2.Core.Combat.CombatManager.PlayerActionsDisabled
MegaCrit.Sts2.Core.Combat.CombatManager.DebugOnlyGetState()
MegaCrit.Sts2.Core.Runs.RunManager.Instance
MegaCrit.Sts2.Core.Runs.RunManager.DebugOnlyGetState()
MegaCrit.Sts2.Core.Context.LocalContext.GetMe(...)
MegaCrit.Sts2.Core.Commands.PlayerCmd.EndTurn(Player, Boolean, Func<Task>)
```

`CombatManager.IsPlayPhase` was not used because it is absent from the recorded host even though it
is present in a newer installed host.

## Commands and results

| Command | Result |
| --- | --- |
| `cargo run --locked --offline --package repo-policy -- --strict` | exit `0`; 121 files, 0 warnings, 0 errors |
| `cargo fmt --all --check` | exit `0` |
| `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` | exit `0` |
| `cargo test --locked --offline --workspace --all-targets --all-features` | exit `0`; all workspace tests passed |
| Windows .NET build against the recorded host `sts2.dll` and `GodotSharp.dll` | exit `0`; 0 warnings, 0 errors |
| `git diff --check` | exit `0` |
| `package-runtime-addon.sh` against the recorded host | exit `0`; exactly 3 staged files |

Continuation validation on 2026-09-03 reran the target-local metadata, strict policy, formatting,
warnings-denied Clippy, locked/offline workspace tests, and diff checks successfully. The native
test suite reported six passing tests; the exact-host managed build reported zero warnings and zero
errors. The package hashes below are from that continuation build.

The native runtime test suite contains six passing tests, including the exact constant-work bearer
token comparison. This is a source/build oracle only; it does not establish network timing security
against a live adversary.

## Candidate package hashes

The package was staged outside the repository and contained only these files:

| File | SHA-256 |
| --- | --- |
| `AIAscensionSTS2GameMod.dll` | `504b8adc89f77b2976409930cb30ca05b30e5975e5471e8bf19e39ab88c5189c` |
| `AIAscensionSTS2GameModNative.dll` | `f2500e9e7195cdc8ae1df3676488822b86895740ea31140c1b19a09defd25498` |
| `AIAscensionSTS2GameMod.json` | `559e177f0b6e5d82fc44f6b086b1e728353b2f6e437f5e8fae989083d85659984` |

The candidate includes v2 state/action/operation routes, exact context validation, main-thread
host translation, bounded operation retention, settlement polling, bounded mod-side queue
admission, and pre-claim timeout cancellation.
No game was installed or launched by this build evidence, no profile or save was changed, and no
provider or credential was used.

## Remaining gate

The next required oracle is an explicitly authorized disposable-host trace: load this exact
candidate, read a live player-turn state, submit one `end_turn`, observe a fresh next-turn state and
`turn_end_settled` witness, then verify duplicate/stale/identity/timeout behavior and restore all
disposable state. Until that trace is recorded, the v2 host path is not promoted beyond `L1`.
