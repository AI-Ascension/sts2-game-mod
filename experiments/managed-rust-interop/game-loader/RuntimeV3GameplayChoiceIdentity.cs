// SPDX-License-Identifier: MIT

using System;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class RuntimeV3GameplayChoiceIdentity
{
    // Leave space for the action kind and maximum generation in the catalog's action ID.
    private const int MaxChoiceIdentityBytes = RuntimeV3GameplayContract.MaxTextBytes - 64;

    internal static string Card(string cardId, string title)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(cardId) || cardId.Length > MaxChoiceIdentityBytes)
            throw new ArgumentException("invalid card identity", nameof(cardId));
        int remaining = MaxChoiceIdentityBytes - cardId.Length - 1;
        if (remaining <= 0) return cardId;
        string label = new(title.Select(character => char.IsAsciiLetterOrDigit(character)
            ? character : '-').Take(remaining).ToArray());
        label = label.Trim('-');
        return label.Length == 0 ? cardId : cardId + ":" + label;
    }
}
