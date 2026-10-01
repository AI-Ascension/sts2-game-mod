// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

namespace AiAscension.Sts2GameMod.GameplayTests;

/// <summary>
/// Checks that host-supplied offered-entry metadata actually reaches the serialized observation.
///
/// <para>
/// The point of <c>sts2-game-mod#171</c> is the producer, not the type. These checks drive
/// <c>RuntimeV3GameplayCodec</c> directly and read the emitted JSON, because an offered entry that
/// is accepted but never filled would leave the livelock in place while every type-level test still
/// passed.
/// </para>
/// </summary>
internal static partial class OfferedEntryChecks
{
    internal static void Run()
    {
        LegacyHostsKeepTheBareStringForm();
        HostSuppliedFieldsReachTheJson();
        DisclosedContentsReachTheJson();
        WithheldAttributesStayAbsent();
        DuplicateIdentitiesAreRefused();
        NegativeCostIsWithheldNotEmitted();
        BoundsAreRefusedBeforeSerialization();
        UndescribedHostFallsBackToBareIdentities();
        NoCatalogBackedRarityOrProbabilityExists();
    }

    /// <summary>
    /// A host that described nothing must still emit the exact bare-string array it emitted before
    /// the observation was widened. This is the compatibility hinge, and it is asserted on the JSON
    /// text rather than on a parsed value, because a byte-comparing consumer is what would break.
    /// </summary>
    private static void LegacyHostsKeepTheBareStringForm()
    {
        RuntimeV3GameplayObservation observation = Reward(
            new[]
            {
                RuntimeV3GameplayOfferedEntry.Identified("card:21:Setup-Strike"),
                RuntimeV3GameplayOfferedEntry.Identified("card:22:Tremble")
            });
        string json = Serialize(observation);
        Check(json.Contains("\"options\":[\"card:21:Setup-Strike\",\"card:22:Tremble\"]", StringComparison.Ordinal),
            "an undescribed offered set must keep the legacy bare-string bytes: " + json);
        Check(!json.Contains("choice_id", StringComparison.Ordinal),
            "an identity-only entry must not gain a choice_id wrapper: " + json);
    }

    /// <summary>
    /// The attributes the host read must appear, on the object form, with the host's own values.
    /// </summary>
    private static void HostSuppliedFieldsReachTheJson()
    {
        RuntimeV3GameplayObservation observation = Reward(new[]
        {
            RuntimeV3GameplayOfferedEntry.FromCard("card:21:Setup-Strike", "Setup Strike", 1, false),
            RuntimeV3GameplayOfferedEntry.FromCard("card:22:Tremble", "Tremble", 2, true)
        });
        using JsonDocument document = JsonDocument.Parse(Serialize(observation));
        JsonElement options = State(document).GetProperty("options");
        Check(options.GetArrayLength() == 2, "both described entries must be emitted");

        JsonElement first = options[0];
        Check(first.ValueKind == JsonValueKind.Object, "a described entry uses the object form");
        Check(first.GetProperty("choice_id").GetString() == "card:21:Setup-Strike",
            "the described entry keeps the identity the action catalog used");
        Check(first.GetProperty("name").GetString() == "Setup Strike", "the host title must reach the JSON");
        Check(first.GetProperty("cost").GetInt32() == 1, "the host cost must reach the JSON");
        Check(!first.GetProperty("upgraded").GetBoolean(), "the host upgraded flag must reach the JSON");

        JsonElement second = options[1];
        Check(second.GetProperty("cost").GetInt32() == 2
            && second.GetProperty("upgraded").GetBoolean(),
            "each entry carries its own host values");
    }

    /// <summary>
    /// An attribute the host did not supply must be absent from the JSON, not filled with a
    /// plausible value. A default here would read downstream as an observed fact about the game.
    /// </summary>
    private static void WithheldAttributesStayAbsent()
    {
        RuntimeV3GameplayObservation observation = Reward(new[]
        {
            // Only the identity and the upgraded flag were read by this host.
            RuntimeV3GameplayOfferedEntry.FromCard("card:21:Setup-Strike", null, null, true)
        });
        using JsonDocument document = JsonDocument.Parse(Serialize(observation));
        JsonElement entry = State(document).GetProperty("options")[0];
        Check(entry.GetProperty("upgraded").GetBoolean(), "the supplied flag is published");
        Check(!entry.TryGetProperty("name", out _), "an absent name must not be invented");
        Check(!entry.TryGetProperty("cost", out _), "an absent cost must not be invented");
        Check(!entry.TryGetProperty("description", out _), "an absent description must not be invented");
        Check(!entry.TryGetProperty("rarity", out _), "an absent rarity must not be invented");
    }

    /// <summary>
    /// Two entries sharing an identity are refused, because the catalog carries a single
    /// <c>reward_id</c> per action and could not act on the choice.
    /// </summary>
    private static void DuplicateIdentitiesAreRefused()
    {
        RuntimeV3GameplayObservation observation = Reward(new[]
        {
            RuntimeV3GameplayOfferedEntry.Identified("card:21:Setup-Strike"),
            RuntimeV3GameplayOfferedEntry.Identified("card:21:Setup-Strike")
        });
        Check(!RuntimeV3GameplayCodec.TrySerialize(observation, Array.Empty<LegalActionReference>(),
            out _, out string error),
            "a duplicate offered identity must be refused");
        Check(error.Contains("unique", StringComparison.Ordinal),
            "the refusal must name the duplicate identity as the cause: " + error);
    }

    /// <summary>
    /// The harness rejects a negative <c>cost</c> as <c>InvalidNumber</c>, so the producer withholds
    /// it. This must stay consistent with the Rust contract, which types <c>cost</c> as unsigned.
    /// </summary>
    private static void NegativeCostIsWithheldNotEmitted()
    {
        Check(!RuntimeV3GameplayContract.IsPublishableCost(-1),
            "a negative cost is never publishable");
        Check(RuntimeV3GameplayContract.IsPublishableCost(0), "a zero cost is publishable");
        Check(RuntimeV3GameplayContract.IsPublishableCost(255), "the harness maximum is publishable");
        Check(!RuntimeV3GameplayContract.IsPublishableCost(256), "above the harness maximum is withheld");

        RuntimeV3GameplayOfferedEntry entry = RuntimeV3GameplayOfferedEntry.FromCard(
            "card:21:Setup-Strike", "Setup Strike", -1, false);
        Check(entry.Cost is null, "a negative resolved cost is carried as absent");
        Check(entry.Name == "Setup Strike",
            "withholding the cost must not withhold the attributes the host did read");
    }

    /// <summary>
    /// A host that fills only <c>StateValues</c> keeps working: the serializer falls back to those
    /// bare identities rather than emitting an empty set.
    /// </summary>
    private static void UndescribedHostFallsBackToBareIdentities()
    {
        RuntimeV3GameplayObservation observation = RuntimeV3GameplayFixtures.CombatObservation(4) with
        {
            State = RuntimeV3GameplayState.Reward,
            StateValues = new[] { "reward:0:CardReward", "reward:1:PotionReward" }
        };
        string json = Serialize(observation);
        Check(json.Contains("\"options\":[\"reward:0:CardReward\",\"reward:1:PotionReward\"]",
            StringComparison.Ordinal),
            "an undescribed surface falls back to its bare identities: " + json);
    }

    /// <summary>
    /// Guards the disclosure rule against a future convenience: the loader holds no rarity or
    /// probability table, and the entry type exposes no way to fill one in.
    /// </summary>
    private static void NoCatalogBackedRarityOrProbabilityExists()
    {
        RuntimeV3GameplayOfferedEntry bare = RuntimeV3GameplayOfferedEntry.Identified("card:21:X");
        Check(bare.Rarity is null && bare.Description is null && bare.Cost is null,
            "the identity-only constructor leaves every disclosed attribute absent");

        // A caller can supply a rarity, but nothing in the loader computes one: the only source is
        // the host reading it off its own object, which is what FromCard takes as an argument.
        RuntimeV3GameplayOfferedEntry supplied = RuntimeV3GameplayOfferedEntry.FromCard(
            "card:21:X", "X", 1, false, null, "starter");
        Check(supplied.Rarity == "starter",
            "a rarity the host supplied is passed through verbatim");
        Check(RuntimeV3GameplayOfferedEntry.FromCard("card:21:X", "X", 1, false, null, null).Rarity is null,
            "a rarity the host did not supply is never guessed");
    }

    internal static RuntimeV3GameplayObservation Reward(IReadOnlyList<RuntimeV3GameplayOfferedEntry> entries) =>
        RuntimeV3GameplayFixtures.CombatObservation(3) with
        {
            State = RuntimeV3GameplayState.Reward,
            StateValues = Identities(entries),
            Offered = new RuntimeV3GameplayOfferedSet(entries)
        };

    private static string[] Identities(IReadOnlyList<RuntimeV3GameplayOfferedEntry> entries)
    {
        var identities = new string[entries.Count];
        for (int index = 0; index < identities.Length; index++)
        {
            identities[index] = entries[index].ChoiceId;
        }
        return identities;
    }

    internal static string Serialize(RuntimeV3GameplayObservation observation)
    {
        Check(RuntimeV3GameplayCodec.TrySerialize(observation, Array.Empty<LegalActionReference>(),
            out string json, out string error), "observation must serialize: " + error);
        return json;
    }

    internal static JsonElement State(JsonDocument document) => document.RootElement.GetProperty("state");

    internal static void Check(bool passed, string message)
    {
        if (!passed) { throw new InvalidOperationException(message); }
    }
}
