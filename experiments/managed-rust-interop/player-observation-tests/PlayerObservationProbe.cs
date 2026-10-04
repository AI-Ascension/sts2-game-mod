// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private readonly Dictionary<CardModel, string> _ids = new();

    internal static RuntimeV3GameplayPlayer Capture(Player? player, PlayerCombatState? combat = null) =>
        new LiveCombatSource().ProjectPlayer(player, combat);

    private static ushort U16(int value) => (ushort)Math.Clamp(value, 0, ushort.MaxValue);

    private string CardId(CardModel card)
    {
        if (!_ids.TryGetValue(card, out string? id))
        {
            id = $"card:{_ids.Count + 1}";
            _ids.Add(card, id);
        }
        return id;
    }

    private static string PotionId(PotionModel potion, int slot) =>
        $"potion:{slot}:{potion.Id.Entry}";

    private static bool? PotionUsable(Player player, PotionModel potion, int slot)
    {
        _ = player;
        _ = slot;
        return potion.Usable;
    }

    private static byte? MaxPotionCount(Player? player)
    {
        if (player is null) return null;
        try
        {
            int count = player.MaxPotionCount;
            return count is >= 0 and <= byte.MaxValue ? (byte)count : null;
        }
        catch (Exception)
        {
            return null;
        }
    }
}

internal static class PlayerObservationProbe
{
    private static int Main()
    {
        (string Name, Action Run)[] cases =
        {
            ("held card, relic and potion host descriptions reach the runtime-v3 codec", DescriptionsAndInventory),
            ("unavailable host inventories and counts stay omitted", UnavailableInventory),
            ("observed empty inventory differs from unavailable", ObservedEmpty),
            ("unrecognized and missing potion values stay explicit or absent", UnknownAndMissingOptionalValues),
            ("host collections stop at the shipped entity bound", CollectionBounds)
        };
        int passed = 0;
        try
        {
            foreach ((string name, Action run) in cases)
            {
                run();
                passed++;
                Console.WriteLine($"PASS {name}");
            }
            Console.WriteLine($"Passed {passed} source-linked synthetic producer checks; native behavior unverified.");
            return 0;
        }
        catch (Exception error)
        {
            Console.Error.WriteLine($"FAIL after {passed} checks: {error.Message}");
            return 1;
        }
    }

    private static void DescriptionsAndInventory()
    {
        CardModel card = new("card:wall", "Blood Wall", "Gain 8 Block.", 2) { IsUpgraded = true };
        RelicModel relic = new("relic:anchor", "Anchor", "Start each combat with 10 Block.");
        PotionModel fire = new("fire", 0, "Fire Potion", "Deal 20 damage to one enemy.", TargetType.AnyEnemy, true);
        PotionModel unknown = new("mystery", 2, "Murky Draught", null, TargetType.Mystery, false);
        RuntimeV3GameplayPlayer player = LiveCombatSource.Capture(HostPlayer(
            relics: new[] { relic }, potions: new[] { fire, unknown },
            slots: new PotionModel?[] { fire, null, unknown }, maxPotionCount: 3),
            new PlayerCombatState { Hand = new CardPile { Cards = new[] { card } } });

        Equal("Gain 8 Block.", player.Hand.Single().Description);
        Equal("Start each combat with 10 Block.", player.Relics?.Single().Description);
        Equal((byte)3, player.PotionSlots);
        Equal((byte)3, player.MaxPotionSlots);
        Equal("Deal 20 damage to one enemy.", player.Potions?.Single(potion => potion.PotionId.Contains("fire", StringComparison.Ordinal)).Description);
        Equal((byte)2, player.Potions?.Single(potion => potion.PotionId.Contains("mystery", StringComparison.Ordinal)).Slot);

        using JsonDocument json = Serialize(player);
        JsonElement encodedPlayer = json.RootElement.GetProperty("player");
        Equal("Gain 8 Block.", encodedPlayer.GetProperty("hand")[0].GetProperty("description").GetString());
        Equal("Start each combat with 10 Block.", encodedPlayer.GetProperty("relics")[0].GetProperty("description").GetString());
        Equal(3, encodedPlayer.GetProperty("potion_slots").GetInt32());
        Equal(2, encodedPlayer.GetProperty("potions").GetArrayLength());
        Equal("Deal 20 damage to one enemy.", encodedPlayer.GetProperty("potions")[0].GetProperty("description").GetString());
    }

    private static void UnavailableInventory()
    {
        Player host = HostPlayer(
            readRelics: () => throw new InvalidOperationException("private sentinel"),
            readPotions: () => throw new InvalidOperationException("private sentinel"),
            readSlots: () => throw new InvalidOperationException("private sentinel"),
            readMaxPotionCount: () => throw new InvalidOperationException("private sentinel"));
        RuntimeV3GameplayPlayer player = LiveCombatSource.Capture(host);
        Equal<IReadOnlyList<RuntimeV3GameplayRelic>?>(null, player.Relics);
        Equal<IReadOnlyList<RuntimeV3GameplayPotion>?>(null, player.Potions);
        Equal<byte?>(null, player.PotionSlots);
        Equal<byte?>(null, player.MaxPotionSlots);

        using JsonDocument json = Serialize(player);
        JsonElement encodedPlayer = json.RootElement.GetProperty("player");
        False(encodedPlayer.TryGetProperty("relics", out _));
        False(encodedPlayer.TryGetProperty("potions", out _));
        False(encodedPlayer.TryGetProperty("potion_slots", out _));
        False(encodedPlayer.TryGetProperty("max_potion_slots", out _));
    }

    private static void ObservedEmpty()
    {
        RuntimeV3GameplayPlayer player = LiveCombatSource.Capture(HostPlayer(
            slots: Array.Empty<PotionModel?>(), maxPotionCount: 0));
        Equal(0, player.Relics?.Count);
        Equal(0, player.Potions?.Count);
        Equal((byte)0, player.PotionSlots);
        Equal((byte)0, player.MaxPotionSlots);

        using JsonDocument json = Serialize(player);
        JsonElement encodedPlayer = json.RootElement.GetProperty("player");
        Equal(0, encodedPlayer.GetProperty("relics").GetArrayLength());
        Equal(0, encodedPlayer.GetProperty("potions").GetArrayLength());
        Equal(0, encodedPlayer.GetProperty("potion_slots").GetInt32());
        Equal(0, encodedPlayer.GetProperty("max_potion_slots").GetInt32());
    }

    private static void UnknownAndMissingOptionalValues()
    {
        PotionModel unknown = new("other", 0, "Odd Potion", null, TargetType.Mystery, null);
        PotionModel missing = new("missing", 1, "Unnamed Mode", null, null, null);
        RuntimeV3GameplayPlayer player = LiveCombatSource.Capture(HostPlayer(
            potions: new[] { unknown, missing }, slots: new PotionModel?[] { unknown, missing }, maxPotionCount: 2));
        using JsonDocument json = Serialize(player);
        JsonElement potions = json.RootElement.GetProperty("player").GetProperty("potions");
        Equal("unknown", potions[0].GetProperty("target_mode").GetString());
        False(potions[0].TryGetProperty("usable", out _));
        False(potions[0].TryGetProperty("description", out _));
        False(potions[1].TryGetProperty("target_mode", out _));
        False(potions[1].TryGetProperty("usable", out _));
    }

    private static void CollectionBounds()
    {
        RelicModel[] tooManyRelics = Enumerable.Range(0, RuntimeV3GameplayContract.MaxEntities + 1)
            .Select(index => new RelicModel($"relic:{index}", "Anchor", null)).ToArray();
        PotionModel[] tooManyPotions = Enumerable.Range(0, RuntimeV3GameplayContract.MaxEntities + 1)
            .Select(index => new PotionModel($"item:{index}", index, "Potion", null, TargetType.None, false)).ToArray();
        RuntimeV3GameplayPlayer player = LiveCombatSource.Capture(HostPlayer(
            relics: tooManyRelics, potions: tooManyPotions));
        Equal<IReadOnlyList<RuntimeV3GameplayRelic>?>(null, player.Relics);
        Equal<IReadOnlyList<RuntimeV3GameplayPotion>?>(null, player.Potions);

        CardModel[] tooManyCards = Enumerable.Range(0, RuntimeV3GameplayContract.MaxEntities + 1)
            .Select(index => new CardModel($"card:{index}", "Strike", null)).ToArray();
        RuntimeV3GameplayPlayer cards = LiveCombatSource.Capture(HostPlayer(cards: tooManyCards));
        False(CreateObservation(cards).Validate(out _));
    }

    private static Player HostPlayer(
        IEnumerable<CardModel>? cards = null,
        IEnumerable<RelicModel>? relics = null,
        IEnumerable<PotionModel>? potions = null,
        IReadOnlyList<PotionModel?>? slots = null,
        int maxPotionCount = 0,
        Func<IEnumerable<RelicModel>>? readRelics = null,
        Func<IEnumerable<PotionModel>>? readPotions = null,
        Func<IReadOnlyList<PotionModel?>>? readSlots = null,
        Func<int>? readMaxPotionCount = null) => new()
    {
        Deck = new PlayerDeck { Cards = cards ?? Array.Empty<CardModel>() },
        ReadRelics = readRelics ?? (() => relics ?? Array.Empty<RelicModel>()),
        ReadPotions = readPotions ?? (() => potions ?? Array.Empty<PotionModel>()),
        ReadPotionSlots = readSlots ?? (() => slots ?? Array.Empty<PotionModel?>()),
        ReadMaxPotionCount = readMaxPotionCount ?? (() => maxPotionCount)
    };

    private static JsonDocument Serialize(RuntimeV3GameplayPlayer player)
    {
        RuntimeV3GameplayObservation observation = CreateObservation(player);
        if (!RuntimeV3GameplayCodec.TrySerialize(observation, Array.Empty<LegalActionReference>(),
            out string json, out string error))
            throw new InvalidOperationException($"runtime-v3 codec rejected producer output: {error}");
        return JsonDocument.Parse(json);
    }

    private static RuntimeV3GameplayObservation CreateObservation(RuntimeV3GameplayPlayer player) => new(
        "live:producer-probe", 0, null, player, RuntimeV3GameplayState.Combat,
        Array.Empty<string>(), Array.Empty<RuntimeV3GameplayEnemy>());

    private static void Equal<T>(T expected, T actual)
    {
        if (!EqualityComparer<T>.Default.Equals(expected, actual))
            throw new InvalidOperationException($"expected {expected}, received {actual}");
    }

    private static void False(bool value)
    {
        if (value) throw new InvalidOperationException("expected false");
    }
}
