// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

/// <summary>
/// The <c>contents</c> and bounds halves of the offered-entry producer checks, split beside
/// <see cref="OfferedEntryChecks"/> to stay inside the repository's handwritten size policy.
/// </summary>
internal static partial class OfferedEntryChecks
{
    /// <summary>
    /// What an option would present next must reach the JSON, or the first reward choice stays
    /// blind: a card reward is only an identifier until it has already been taken.
    ///
    /// <para>
    /// Both content forms are checked, because the bare form is what a host that read only the
    /// identity emits, and the object form is what one that read the card emits. A content entry
    /// never carries a <c>contents</c> key of its own.
    /// </para>
    /// </summary>
    private static void DisclosedContentsReachTheJson()
    {
        RuntimeV3GameplayOfferedEntry entry = RuntimeV3GameplayOfferedEntry.FromCard(
            "reward:0:CardReward", "Card Reward", null, null, contents: new[]
            {
                RuntimeV3GameplayOfferedContent.FromCard("card:21:Setup-Strike", "Setup Strike", 1, false),
                RuntimeV3GameplayOfferedContent.Identified("card:22:Tremble")
            });
        using JsonDocument document = JsonDocument.Parse(Serialize(Reward(new[] { entry })));
        JsonElement contents = State(document).GetProperty("options")[0].GetProperty("contents");
        Check(contents.GetArrayLength() == 2, "both disclosed contents must be emitted");

        JsonElement described = contents[0];
        Check(described.ValueKind == JsonValueKind.Object, "a described content uses the object form");
        Check(described.GetProperty("choice_id").GetString() == "card:21:Setup-Strike",
            "a described content keeps the identity the action catalog used");
        Check(described.GetProperty("name").GetString() == "Setup Strike"
            && described.GetProperty("cost").GetInt32() == 1
            && !described.GetProperty("upgraded").GetBoolean(),
            "the host's card attributes must reach the nested JSON");
        Check(!described.TryGetProperty("contents", out _),
            "a content entry must not carry a contents key of its own");

        Check(contents[1].ValueKind == JsonValueKind.String
            && contents[1].GetString() == "card:22:Tremble",
            "a content the host described no further detail keeps the bare string form");
    }

    /// <summary>
    /// The producer refuses an entry that breaks a bound rather than emitting JSON the harness
    /// would reject, so the refusal happens at observation validation instead.
    /// </summary>
    private static void BoundsAreRefusedBeforeSerialization()
    {
        RuntimeV3GameplayOfferedEntry overText = RuntimeV3GameplayOfferedEntry.FromCard(
            "card:21:X", new string('n', RuntimeV3GameplayContract.MaxTextBytes + 1), 1, false);
        // FromCard withholds unpublishable text, so an over-long host title is absent, not refused.
        Check(overText.Name is null,
            "text past the 512-byte bound is withheld rather than published");

        RuntimeV3GameplayOfferedEntry overCost = new("card:21:X", "X", RuntimeV3GameplayContract.MaxCost + 1);
        Check(!Reward(new[] { overCost }).Validate(out string error)
            && error.Contains("cost", StringComparison.Ordinal),
            "a cost above the harness maximum must be refused: " + error);

        var tooManyContents = new RuntimeV3GameplayOfferedContent[
            RuntimeV3GameplayContract.MaxChoiceContents + 1];
        for (int index = 0; index < tooManyContents.Length; index++)
        {
            tooManyContents[index] = RuntimeV3GameplayOfferedContent.Identified($"card:{index}:X");
        }
        RuntimeV3GameplayOfferedEntry wide = RuntimeV3GameplayOfferedEntry.FromCard(
            "reward:0:CardReward", "Card Reward", null, null, contents: tooManyContents);
        Check(!Reward(new[] { wide }).Validate(out error)
            && error.Contains("contents", StringComparison.Ordinal),
            "contents past the harness bound of 32 must be refused: " + error);

        var tooManyEntries = new RuntimeV3GameplayOfferedEntry[
            RuntimeV3GameplayContract.MaxOfferedEntries + 1];
        for (int index = 0; index < tooManyEntries.Length; index++)
        {
            tooManyEntries[index] = RuntimeV3GameplayOfferedEntry.Identified($"card:{index}:X");
        }
        Check(!Reward(tooManyEntries).Validate(out error),
            "an offered set past the harness bound of 256 must be refused: " + error);
    }
}
