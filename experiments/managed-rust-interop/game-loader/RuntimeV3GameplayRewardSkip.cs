// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV3GameplayRewardAlternative(string Id, bool IsPlainSkip);

/// <summary>Owned selection facts; native controls remain on the host thread.</summary>
internal static class RuntimeV3GameplayRewardSkip
{
    internal static bool TrySelection(IReadOnlyList<RuntimeV3GameplayRewardAlternative> alternatives,
        IReadOnlyList<string> clickableIds, int cardCount, out int selection)
    {
        selection = -1;
        if (cardCount is < 1 or > 64 || alternatives.Count is < 1 or > 2
            || clickableIds.Count > 2
            || alternatives.Any(option => option is null || string.IsNullOrEmpty(option.Id))
            || alternatives.Select(option => option.Id).Distinct(StringComparer.Ordinal).Count()
                != alternatives.Count
            || clickableIds.Count(id => id == "Skip") != 1) return false;
        for (int index = 0; index < alternatives.Count; index++)
        {
            if (alternatives[index] is not { Id: "Skip", IsPlainSkip: true }) continue;
            // The native screen completes alternatives after its card-option indices.
            selection = cardCount + index;
            return true;
        }
        return false;
    }

    internal static bool Selected(Task<int?> task, int expectedSelection) =>
        expectedSelection >= 1 && task.IsCompletedSuccessfully && task.Result == expectedSelection;
}
