// SPDX-License-Identifier: MIT

using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Identifies an observation that is ready for a caller to select a gameplay action.
/// Run startup can expose a valid native run before its first visible surface and catalog exist;
/// that transient recovery projection must not be used as a seeded-run settlement witness.
/// </summary>
internal static class RuntimeV3GameplayReadiness
{
    internal static bool IsReady(
        RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> actions)
    {
        if (observation.State is RuntimeV3GameplayState.Setup
                or RuntimeV3GameplayState.Recovery
                or RuntimeV3GameplayState.Victory
                or RuntimeV3GameplayState.Defeat
                or RuntimeV3GameplayState.Unknown
            || !observation.IsActionable
            || !observation.InputEnabled
            || observation.ModalBlocking
            || !LegalActionCatalog.TryCreate(observation.Generation, actions, out _))
        {
            return false;
        }

        return actions.Count > 0;
    }
}
