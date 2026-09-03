# Runtime-v3 gameplay host adapter build evidence

- Date: 2026-09-02
- Evidence level: `L1` target-local source/build/package evidence
- Status: `confirmed` for the named build and reflection oracles; live host behavior remains
  `unverified`
- Source base: game-mod remote `main` `f5cc07b6d0f0ef89bd06ea9378e39aa93e82a405` plus the
  isolated candidate changes
- Runtime profile: `sts2-protocol/runtime-v3-gameplay`
- Action: `play_card`
- Schema digest: `c961bbde893f0422f80233d14ea9ae8b648ee9032136e5370aa5f6b949f6575e`

## Host reference used for build

| Field | Value |
| --- | --- |
| Game | STS2 v0.107.1 |
| Host release label | `59260271` |
| Platform | Windows x86-64 |
| Managed runtime | .NET 9 host; .NET SDK `10.0.204` used for build |
| Native target | `x86_64-pc-windows-gnu`; Rust `1.97.1` |

The proprietary host assembly was used only as an operator-local build and reflection input. It is
not stored, copied, or packaged by this repository.

## Verified host symbols used by the candidate

```text
CombatManager.Instance
CombatManager.IsInProgress
CombatManager.DebugOnlyGetState()
CombatManager.PlayerActionsDisabled
CombatState.RoundNumber
CombatState.Enemies
RunManager.Instance
RunManager.DebugOnlyGetState()
RunManager.ActionQueueSynchronizer
ActionQueueSynchronizer.RequestEnqueue(GameAction)
LocalContext.GetMe(RunState)
Player.PlayerCombatState
PlayerCombatState.Hand
PlayerCombatState.DrawPile
PlayerCombatState.DiscardPile
PlayerCombatState.ExhaustPile
PlayerCombatState.Energy
CardPile.Cards
CardModel.CanPlay()
CardModel.CanPlayTargeting(Creature)
PlayCardAction(CardModel, Creature)
Creature.CombatId
Creature.IsAlive
Creature.IsHittable
```

The candidate does not depend on the newer-only `CombatManager.IsPlayPhase` property. Explicit
numeric enemy combat IDs are the live target namespace; the protocol's bounded `target_id` remains
an opaque string at the transport boundary.

## Commands and results

| Command | Result |
| --- | --- |
| `cargo fmt --all` | exit `0` |
| `cargo check --locked --offline --workspace --all-targets --all-features` | exit `0` |
| `cargo test --locked --offline --workspace --all-targets --all-features` | exit `0`; all workspace tests passed |
| `cargo clippy --locked --offline --workspace --all-targets --all-features -- -D warnings` | exit `0` |
| `cargo run --locked --offline --package repo-policy -- --strict` | exit `0`; 121 sized files, 0 warnings, 0 errors |
| Windows .NET Release build against the recorded host | exit `0`; 0 warnings, 0 errors |
| `package-runtime-addon.sh` against the recorded host | exit `0`; exactly 3 staged files |
| `git diff --check` | exit `0` |

Continuation validation on 2026-09-03 reran the game-mod target's metadata, strict policy,
formatting, warnings-denied Clippy, locked/offline workspace tests, and diff checks successfully.
The package hashes below include the native authentication hardening shared by the v2 and v3 paths.

## Candidate package hashes

The package was staged outside the repository and contained only these files:

| File | SHA-256 |
| --- | --- |
| `AIAscensionSTS2GameMod.dll` | `504b8adc89f77b2976409930cb30ca05b30e5975e5471e8bf19e39ab88c5189c` |
| `AIAscensionSTS2GameModNative.dll` | `f2500e9e7195cdc8ae1df3676488822b86895740ea31140c1b19a09defd25498` |
| `AIAscensionSTS2GameMod.json` | `559e177f0b6e5d82fc44f6b086b1e728353b2f6e437f5e8fae989083d85659984` |

The candidate contains the v2 `end_turn` path and the separately versioned v3 `play_card` state,
action, and operation routes. No game was installed or launched by this build evidence, no profile
or save was changed, and no provider or credential was used.

## Remaining gate

The required next oracle is an explicitly authorized disposable-host trace: read a live
`combat/player_turn` state, submit one legal `play_card`, observe a fresh authoritative state with
the bounded collection or energy effect and `play_card_settled` witness, then verify duplicate,
stale, wrong-target, timeout, and cleanup behavior. Until that trace is recorded, this profile is
not promoted beyond `L1`.
