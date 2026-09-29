# ADR 0075: A non-consuming reward skip is not offered

- Status: Accepted; source-only producer gates executed; native consumption evidence pending
- Date: 2026-09-29
- Owner: sts2-game-mod

Refs [sts2-game-mod#171](https://github.com/AI-Ascension/sts2-game-mod/issues/171) defect 1.
Defect 2 of that issue (the offered set carries no information) is a modelling decision
deferred to [sts2-game-core#20](https://github.com/AI-Ascension/sts2-game-core/issues/20) and
is untouched here.

## Defect

A live run cycled on one card reward for dozens of generations: the model was offered
`skip_reward` on the card-reward selection screen, took it, and the same
`reward:5:CardReward` was offered again. The skip was not a real skip.

`LiveCombatSource.RewardSkip.cs` admitted exactly one alternative, matched by comparing the host's
own `_extraOptions` entry against a locally constructed reference:
`new CardRewardAlternative("Skip", PostAlternateCardRewardAction.EndSelectionAndDoNotCompleteReward).OnSelect`,
plus `option.AfterSelected == PostAlternateCardRewardAction.EndSelectionAndDoNotCompleteReward`.
The mod never *adds* that alternative; it only recognizes the one the host already owns. The
decisive fact is therefore the enum's own name and declared semantics: the host's skip
**ends the selection and explicitly does not complete the reward**. The reward is never
consumed, so the screen returns it to the reward set and the cycle is unbounded.

The skip path had no consumption assertion at all. Its `postcondition` checked only that the
selection screen went away (`!ReferenceEquals(RewardOverlay(), screen)`) and that the deck was
unchanged. The `choose_reward` path, by contrast, settles on
`reward.SuccessfullySelected` (`LiveCombatSource.RewardDispatch.cs`), and
`LiveCombatSource.Rewards.cs` itself filters unclaimed reward buttons on
`SuccessfullySelected: false`. Consumption was therefore observable and asserted everywhere
except on the skip.

## Decision

Do not offer `skip_reward` on the card-reward selection screen, and remove the non-consuming
skip machinery rather than leave it dormant. The host offers no consuming alternative on that
screen: the only way to complete a card reward is to take a card.

This is admission, not a workaround. Three options were available and only one is supported by
the host's own types:

1. *Claim the reward through a native completion path.* Rejected. The host's only skip callback
   is `EndSelectionAndDoNotCompleteReward`; there is no completing skip to bind to, and the mod
   adds no alternatives of its own.
2. *Route the skip through a host action that consumes a card reward.* Rejected. Consuming a
   card reward means selecting a specific card, which is not a skip and is already offered as
   `select_card`. Treating "take a specific card" as a skip would destroy the model's ability to
   decline.
3. *Fail closed on admission.* Adopted.

The screen is not left inert by this. `RewardActions` still offers `select_card` for every
visible card on `NCardRewardSelectionScreen`, and that path *does* consume the reward. A model
facing a card reward must therefore choose a card, which is a decision that can make progress.
The livelock is removed without introducing a stall.

`skip_reward` is also refused in `LegalActionReference.Validate`, so a stale or hand-built
catalog entry naming that kind is rejected with the catalog's own "not part of the Runtime-v3
catalog" error rather than reaching the host. Removing the offer and removing the dispatch arm
together means the action has no producer and no executor.

## Compatibility and exclusions

This is a source-only, producer-side narrowing. The mod's producer can no longer emit an action
the harness already accepts, so no consumer migrates.

The frozen protocol still declares `skip_reward` as a wire arm in
`protocol-artifact/runtime-v3-gameplay/schema.json`,
`protocol-artifact/runtime-v4-expert/schema.json`, the mirrored `schemas/` copies, and the Rust
decoder (`RuntimeV3GameplayAction::SkipReward`). Those are decoder and artifact concerns owned by
`sts2-protocol`; their `SHA256SUMS` must keep verifying, so the schema bytes and the conformance
case's `typed_action_kinds` list are deliberately unchanged. A decoder that stops accepting a
published kind would be an incompatible artifact revision, not a bug fix. The protocol keeps the
arm; this repository stops producing it.

Removing `RuntimeV3GameplayRewardSkip`, `LiveCombatSource.RewardSkip` and
`RewardSkipChecks` deletes code that only ever encoded the non-consuming skip. Leaving it in
place would leave a second, unreviewed path by which `skip_reward` could be re-admitted.

## Verification

`experiments/managed-rust-interop/gameplay-tests/RewardSkipChecks.cs` previously pinned the
non-consuming behavior: it asserted that the retained native alternate index was observable and
that the selection task settled. Those assertions described the defect, so they are removed
rather than inverted; a green test that pins a livelock is worse than no test.

The deleted probe was a **source-only probe over pure decision logic**. It never loaded the
proprietary host, and the gameplay probe's project file links only
`RuntimeV3Gameplay*.cs` plus other managed boundary files -- not
`LiveCombatSource.RewardSkip.cs`, which needs `sts2.dll`. The same limit applies to the fix: the
removed offer gate and the removed dispatch arm live entirely in host-dependent managed code
that no CI job compiles. This repository can show that the non-consuming skip is no longer
offered, admitted or executed; it cannot show what the game does when a human presses Skip.

## Next boundary

Native acceptance (the issue's T3) still owes: on an authorized host, press the real Skip on a
card-reward screen and record whether the host re-offers the same reward. That evidence is the
only thing that could reopen the question of a *consuming* skip, and it is exactly what is
unavailable on this Linux box with no Steam and no proprietary `sts2.dll`. If a future host
version does expose a completing skip, options 1 and 2 become available and this record should
be revisited rather than treated as settled.
