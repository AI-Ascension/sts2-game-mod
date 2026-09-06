// SPDX-License-Identifier: MIT

using System;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

internal static class RunOptionsChecks
{
    internal static void Run()
    {
        if (RuntimeV3GameplayRunOptions.Parse(null, null).Practice
            || RuntimeV3GameplayRunOptions.Parse("standard", null).Seed is not null
            || RuntimeV3GameplayRunOptions.Parse("practice", "SYNTHETIC1") != new RuntimeV3GameplayRunOptions(true, "SYNTHETIC1"))
            throw new InvalidOperationException("standard and practice configuration must remain distinct");
        foreach (var (mode, seed) in new (string?, string?)[]
        {
            (null, "SYNTHETIC1"), ("standard", "SYNTHETIC1"), ("practice", null),
            ("practice", ""), ("practice", "invalid seed"), ("unknown", null)
        })
        {
            bool rejected = false;
            try { RuntimeV3GameplayRunOptions.Parse(mode, seed); }
            catch (InvalidOperationException) { rejected = true; }
            if (!rejected) throw new InvalidOperationException("invalid or ambiguous run configuration was admitted");
        }
    }
}
