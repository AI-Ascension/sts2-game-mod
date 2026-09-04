// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Managed-side names for the neutral Runtime-v3 gameplay contract.</summary>
internal static class RuntimeV3GameplayContract
{
    internal const string ProtocolVersion = "runtime-v3-gameplay";
    internal const string Artifact = "sts2-protocol/runtime-v3-gameplay";
    internal const string SchemaSource = "schemas/runtime-v3-gameplay.schema.json";
    internal const string Generator = "hand-authored";
    internal const string SchemaDigest = "fbfb18279b0c7ebb350ef0ce0d56547fa11e83985b13380cb2b0f1dba4cb56e9";
    internal const ulong MaxGeneration = 9_007_199_254_740_991;
    internal const int MaxLegalActions = 256;
    internal const int MaxEntities = 256;
    internal const int MaxTextBytes = 512;

    internal static bool IsIdentity(string value) =>
        !string.IsNullOrEmpty(value)
        && value.Length <= MaxTextBytes
        && AllAscii(value, static character =>
            char.IsAsciiLetterOrDigit(character) || ".:/-_".Contains(character));

    internal static bool IsText(string value)
    {
        if (string.IsNullOrEmpty(value) || value.Length > MaxTextBytes)
        {
            return false;
        }
        foreach (char character in value)
        {
            if (char.IsControl(character))
            {
                return false;
            }
        }
        return true;
    }

    private static bool AllAscii(string value, Func<char, bool> predicate)
    {
        foreach (char character in value)
        {
            if (character > 0x7f || !predicate(character))
            {
                return false;
            }
        }

        return true;
    }
}

internal enum RuntimeV3GameplayState
{
    Setup,
    Map,
    Combat,
    Reward,
    Shop,
    Event,
    Rest,
    Selection,
    Victory,
    Defeat,
    Recovery,
    Unknown
}

internal enum RuntimeV3GameplayIntent
{
    Attack,
    Defend,
    Buff,
    Debuff,
    Unknown
}
