// SPDX-License-Identifier: MIT

using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class RuntimeV3GameplayCombatSettlement
{
    internal static bool Ready(RuntimeV3GameplayObservation after, bool hasLegalActions) =>
        after.State is RuntimeV3GameplayState.Victory or RuntimeV3GameplayState.Defeat
        || after.State != RuntimeV3GameplayState.Recovery && after.IsActionable
            && after.InputEnabled && !after.ModalBlocking && hasLegalActions
            && (after.State != RuntimeV3GameplayState.Combat || after.Enemies.Any(enemy => enemy.Hp > 0));

    internal static bool TurnEnded(RuntimeV3GameplayObservation before, RuntimeV3GameplayObservation after) =>
        after.State != RuntimeV3GameplayState.Combat || after.TurnIndex > before.TurnIndex;
}
