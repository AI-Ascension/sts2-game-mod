// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Builds the only projection permitted to cross into the harness/provider lane.</summary>
internal static class FairPlayProjection
{
    internal static bool TrySerialize(
        RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> legalActions,
        out string json,
        out string error)
    {
        if (!RuntimeV3GameplayCodec.TrySerialize(observation, legalActions, out json, out error))
        {
            return false;
        }
        using JsonDocument document = JsonDocument.Parse(json);
        if (!PrivilegedFieldGuard.IsSafeJson(document.RootElement, out error))
        {
            json = string.Empty;
            return false;
        }
        return true;
    }
}
