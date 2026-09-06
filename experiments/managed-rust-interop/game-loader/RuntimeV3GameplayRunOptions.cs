// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV3GameplayRunOptions(bool Practice, string? Seed)
{
    internal static RuntimeV3GameplayRunOptions Parse(string? mode, string? seed) => mode switch
    {
        null or "standard" when seed is null => new(false, null),
        "practice" when seed is not null && RuntimeV3GameplayContract.IsIdentity(seed) => new(true, seed),
        _ => throw new InvalidOperationException(
            "campaign requires standard mode without a supplied seed, or practice mode with an explicit seed")
    };
}
