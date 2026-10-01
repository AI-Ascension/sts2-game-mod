// SPDX-License-Identifier: MIT

using System;
using System.Text;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Managed-side names for the neutral Runtime-v3 gameplay contract.</summary>
internal static class RuntimeV3GameplayContract
{
    internal const string ProtocolVersion = "runtime-v3-gameplay";
    internal const string Artifact = "sts2-protocol/runtime-v3-gameplay";
    internal const string SchemaSource = "schemas/runtime-v3-gameplay.schema.json";
    internal const string Generator = "hand-authored";
    internal const string SchemaDigest = "843e2e546116c8011f378d271406ac2fb4ec0e4c2dedd32dee46cc1500315ad5";
    internal const ulong MaxGeneration = 9_007_199_254_740_991;
    internal const int MaxLegalActions = 256;
    internal const int MaxEntities = 256;
    internal const int MaxTextBytes = 512;
    // Bounds transcribed from the harness observation validator (`collection_bound` /
    // `validate_number_bound` / `valid_text`) so a value this loader can read is not published
    // only for the runtime to refuse. Three of these are *tighter* than the runtime-v3 schema and
    // that is deliberate: the producer must satisfy the runtime, which is the stricter reader.
    //   - `state.{options,choices}` -> MAX_TEXT_ITEMS (256), also the schema's `offered_set` bound.
    //   - `Choice.contents`         -> MAX_CHOICE_CONTENTS (32). The schema's `disclosed_set` says
    //     `maxItems: 256`, so the schema admits a set the runtime refuses; this loader follows the
    //     runtime, because emitting 33 contents would be a value the harness rejects as
    //     `CollectionBounds`. Recorded as a live schema/runtime divergence, not silently widened.
    //   - `cost`                    -> 255, the schema's `maximum` and the harness's number bound.
    // Offered **text** reuses MaxTextBytes: the harness applies `valid_text` (a 512-byte bound) to
    // every offered attribute, including the ones the schema measures in characters.
    internal const int MaxOfferedEntries = 256;
    internal const int MaxChoiceContents = 32;
    internal const int MaxCost = 255;

    internal static bool IsIdentity(string value) =>
        !string.IsNullOrEmpty(value)
        && value.Length <= MaxTextBytes
        && AllAscii(value, static character =>
            char.IsAsciiLetterOrDigit(character) || ".:/-_".Contains(character));

    internal static bool IsText(string value)
    {
        if (string.IsNullOrEmpty(value) || Encoding.UTF8.GetByteCount(value) > MaxTextBytes)
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

    /// <summary>
    /// Whether the host resolved a cost the observation can carry.
    ///
    /// The harness validates `cost` with `value.as_u64().is_some_and(|n| n &lt;= 255)`, so a negative
    /// cost is rejected as `InvalidNumber` rather than read as "unfixed". The producer therefore
    /// emits **no** `cost` key for a negative or out-of-range value instead of emitting one the
    /// harness refuses. Absence is the host-owned answer; a negative number is not a fact about the
    /// game that this boundary is willing to publish.
    /// </summary>
    internal static bool IsPublishableCost(int value) => value is >= 0 and <= MaxCost;

    /// <summary>Whether supplied text may be published verbatim, or must be withheld as absent.</summary>
    internal static bool IsPublishableText(string? value) =>
        value is not null && IsText(value);

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
