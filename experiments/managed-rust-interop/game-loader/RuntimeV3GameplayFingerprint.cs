// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class RuntimeV3GameplayFingerprint
{
    internal static string Create(RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> actions) => JsonSerializer.Serialize(new
        {
            Observation = observation, observation.TurnIndex, observation.IsActionable,
            observation.InputEnabled, observation.ModalBlocking, observation.NodeId,
            observation.ShopItems, Actions = actions
        });
}
