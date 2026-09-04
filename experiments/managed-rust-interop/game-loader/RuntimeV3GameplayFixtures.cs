// SPDX-License-Identifier: MIT

using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class RuntimeV3GameplayFixtures
{
    internal static RuntimeV3GameplayObservation CombatObservation(ulong generation) =>
        new(
            "combat-1",
            generation,
            "visible-seed-text",
            new RuntimeV3GameplayPlayer(
                50,
                50,
                3,
                99,
                new List<RuntimeV3GameplayCard>(),
                new List<RuntimeV3GameplayCard>(),
                new List<RuntimeV3GameplayCard>(),
                new List<RuntimeV3GameplayCard>()),
            RuntimeV3GameplayState.Combat,
            new List<string>(),
            new List<RuntimeV3GameplayEnemy>())
        {
            IsActionable = true,
            InputEnabled = true,
            ModalBlocking = false,
            TurnIndex = 1
        };

    internal static LegalActionReference EndTurn(ulong generation) =>
        new("combat.end-turn", "end_turn", null, null, generation);
}
