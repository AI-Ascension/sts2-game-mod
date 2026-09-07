// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class CombatSettlementChecks
{
    internal static void Run()
    {
        var before = RuntimeV3GameplayFixtures.CombatObservation(1) with { TurnIndex = 10 };
        // Reproduce the observed sequence: playable cards survive the last enemy briefly,
        // then the combat disappears before the reward controls become actionable.
        var dead = before with { Enemies = before.Enemies.Select(enemy => enemy with { Hp = 0 }).ToArray() };
        var recovery = dead with { State = RuntimeV3GameplayState.Recovery };
        var reward = before with { State = RuntimeV3GameplayState.Reward, TurnIndex = 0 };
        if (RuntimeV3GameplayCombatSettlement.Ready(dead, true)
            || RuntimeV3GameplayCombatSettlement.Ready(recovery, true)
            || RuntimeV3GameplayCombatSettlement.Ready(reward, false))
            throw new InvalidOperationException("intermediate combat completion must retain the operation");
        if (!RuntimeV3GameplayCombatSettlement.Ready(reward, true)
            || !RuntimeV3GameplayCombatSettlement.TurnEnded(before, reward))
            throw new InvalidOperationException("actionable rewards complete an end-turn despite reset turn index");
        var demoReward = reward with { IsActionable = false, InputEnabled = false, ModalBlocking = true };
        if (!RuntimeV3GameplayCombatSettlement.Ready(demoReward, false, combatOnly: true)
            || RuntimeV3GameplayCombatSettlement.Ready(demoReward, false)
            || RuntimeV3GameplayCombatSettlement.Ready(recovery, false, combatOnly: true)
            || RuntimeV3GameplayCombatSettlement.Ready(dead, true, combatOnly: true))
            throw new InvalidOperationException("only a combat-only reward is terminal without next controls");
        if (RuntimeV3GameplayCombatSettlement.TurnEnded(before, before)
            || !RuntimeV3GameplayCombatSettlement.TurnEnded(before, before with { TurnIndex = 11 }))
            throw new InvalidOperationException("continuing combat requires the next turn");
        foreach (var state in new[] { RuntimeV3GameplayState.Victory, RuntimeV3GameplayState.Defeat })
            if (!RuntimeV3GameplayCombatSettlement.Ready(before with
            { State = state, IsActionable = false, InputEnabled = false, ModalBlocking = true }, false))
                throw new InvalidOperationException("terminal outcomes require no next action");
    }
}
