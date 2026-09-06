// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    /// <summary>Opt-in projection entry point; the existing Runtime-v3 route is unchanged.</summary>
    internal RuntimeV4ExpertGameplayObservation ObserveExpert()
    {
        RequireThread();
        RuntimeV3GameplayObservation observation = Observe();
        Player? player = CurrentPlayer();
        PlayerCombatState? combat = player?.PlayerCombatState;
        IReadOnlyList<RuntimeV4ExpertGameplayAction> actions = ExpertActions(observation, player, combat);
        return new RuntimeV4ExpertGameplayObservation(
            observation.StateId,
            observation.Generation,
            observation.VisibleSeed,
            ProjectRun(observation, player),
            ProjectPlayerExpert(observation.Player, player, combat),
            ProjectStateExpert(observation),
            actions);
    }

    internal bool TrySerializeExpert(out string json, out string error)
    {
        RuntimeV4ExpertGameplayObservation observation = ObserveExpert();
        return RuntimeV4ExpertGameplayCodec.TrySerialize(observation, out json, out error);
    }

    private static RuntimeV4ExpertGameplayRun ProjectRun(
        RuntimeV3GameplayObservation observation,
        Player? player)
    {
        RunState? run = MegaCrit.Sts2.Core.Runs.RunManager.Instance.DebugOnlyGetState();
        byte? act = run is null ? null : (byte)Math.Clamp(run.CurrentActIndex, 0, 255);
        string? character = ReadNestedIdentity(player, "Character", "Id");
        return new RuntimeV4ExpertGameplayRun(character, act, observation.NodeId);
    }

    private RuntimeV4ExpertGameplayPlayer ProjectPlayerExpert(
        RuntimeV3GameplayPlayer baseline,
        Player? player,
        PlayerCombatState? combat)
    {
        RuntimeV4ExpertGameplayPlayer result = new(
            player is null ? baseline.Hp : U16(player.Creature.CurrentHp),
            player is null ? baseline.MaxHp : U16(player.Creature.MaxHp),
            PublicU16(player?.Creature, "Block"),
            player is null ? baseline.Energy : (byte)Math.Clamp(combat?.Energy ?? 0, 0, 255),
            player is null ? baseline.Gold : (uint)Math.Max(player.Gold, 0))
        {
            Hand = combat is null ? V3Cards(baseline.Hand) : Cards(combat.Hand.Cards),
            Deck = player is null ? V3Cards(baseline.Deck) : Cards(player.Deck.Cards),
            Discard = combat is null ? V3Cards(baseline.Discard) : Cards(combat.DiscardPile.Cards),
            Exhaust = combat is null ? V3Cards(baseline.Exhaust) : Cards(combat.ExhaustPile.Cards),
            Powers = PublicStatuses(player?.Creature, "Powers"),
            Statuses = PublicStatuses(player?.Creature, "Statuses"),
            Relics = Relics(player),
            Potions = Potions(player),
            PotionSlots = OccupiedPotionCount(player),
            MaxPotionSlots = MaxPotionCount(player)
        };
        return result;
    }

    private RuntimeV4ExpertGameplayCard[]? Cards(
        IEnumerable<CardModel>? cards) => cards?.Select(ProjectCard).ToArray();

    private RuntimeV4ExpertGameplayCard ProjectCard(CardModel card) => new(
        CardId(card),
        card.Title,
        Cost(card),
        card.IsUpgraded,
        PublicText(card, "Type"),
        PublicText(card, "Rarity"),
        card.TargetType.ToString(),
        PublicText(card, "Description"));

    private static RuntimeV4ExpertGameplayCard[] V3Cards(
        IReadOnlyList<RuntimeV3GameplayCard> cards) => cards.Select(card =>
            new RuntimeV4ExpertGameplayCard(
                card.CardId,
                card.Name,
                card.Cost,
                card.Upgraded,
                null,
                null,
                null,
                null)).ToArray();

    private static byte? Cost(CardModel card)
    {
        int cost = card.EnergyCost.GetResolved();
        return cost is >= 0 and <= byte.MaxValue ? (byte)cost : null;
    }

    private static RuntimeV4ExpertGameplayRelic[]? Relics(Player? player)
    {
        if (player is null) return null;
        try
        {
            return player.Relics.Select(relic => new RuntimeV4ExpertGameplayRelic(
                relic.Id.Entry, relic.Title.GetFormattedText())).ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static RuntimeV4ExpertGameplayPotion[]? Potions(Player? player)
    {
        if (player is null) return null;
        try
        {
            var result = new List<RuntimeV4ExpertGameplayPotion>();
            foreach (PotionModel potion in player.Potions)
            {
                int slot = player.GetPotionSlotIndex(potion);
                if (slot < 0 || slot > byte.MaxValue) return null;
                result.Add(new RuntimeV4ExpertGameplayPotion(
                    PotionId(potion, slot),
                    potion.Title.GetFormattedText(),
                    (byte)slot,
                    PotionUsable(player, potion, slot),
                    PotionTargetMode(potion)));
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static RuntimeV4ExpertGameplayState ProjectStateExpert(
        RuntimeV3GameplayObservation observation)
    {
        RuntimeV4ExpertGameplayState state = new(observation.State switch
        {
            RuntimeV3GameplayState.Setup => "setup",
            RuntimeV3GameplayState.Map => "map",
            RuntimeV3GameplayState.Combat => "combat",
            RuntimeV3GameplayState.Reward => "reward",
            RuntimeV3GameplayState.Shop => "shop",
            RuntimeV3GameplayState.Event => "event",
            RuntimeV3GameplayState.Rest => "rest",
            RuntimeV3GameplayState.Selection => "selection",
            RuntimeV3GameplayState.Victory => "victory",
            RuntimeV3GameplayState.Defeat => "defeat",
            RuntimeV3GameplayState.Recovery => "recovery",
            _ => "recovery"
        });
        return observation.State switch
        {
            RuntimeV3GameplayState.Setup => state with { Characters = observation.StateValues },
            RuntimeV3GameplayState.Map => state with
            {
                CurrentNodeId = CurrentMapNodeId() ?? observation.NodeId,
                Nodes = VisibleMapNodes(),
                Edges = VisibleMapEdges(),
                Options = observation.StateValues
            },
            RuntimeV3GameplayState.Combat => state with
            {
                TurnIndex = observation.TurnIndex,
                Enemies = observation.Enemies.Select(ProjectEnemy).ToArray()
            },
            RuntimeV3GameplayState.Reward or RuntimeV3GameplayState.Event
                or RuntimeV3GameplayState.Rest or RuntimeV3GameplayState.Selection => state with
            {
                Choices = observation.StateValues.Select(value =>
                    new RuntimeV4ExpertGameplayChoice(value, value, state.Kind, null)).ToArray()
            },
            RuntimeV3GameplayState.Shop => state with
            {
                Items = observation.ShopItems.Select(item =>
                    new RuntimeV4ExpertGameplayShopItem(item.ItemId, item.Name, "unknown", item.Price)).ToArray()
            },
            RuntimeV3GameplayState.Defeat => state with
            {
                Reason = observation.StateValues.Count == 0 ? null : observation.StateValues[0]
            },
            RuntimeV3GameplayState.Recovery => state with
            {
                Code = observation.StateValues.Count == 0 ? "recovery" : observation.StateValues[0]
            },
            _ => state
        };
    }

    private static string? CurrentMapNodeId()
    {
        try
        {
            RunState? run = RunManager.Instance.DebugOnlyGetState();
            return run?.CurrentMapPoint is { } point ? MapId(point, run) : null;
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static RuntimeV4ExpertGameplayEnemy ProjectEnemy(RuntimeV3GameplayEnemy enemy)
    {
        Creature? host = CombatManager.Instance.DebugOnlyGetState()?.Enemies
            .FirstOrDefault(candidate => EnemyId(candidate) == enemy.EnemyId);
        return new RuntimeV4ExpertGameplayEnemy(
            enemy.EnemyId,
            enemy.Name,
            enemy.Hp,
            enemy.MaxHp,
            PublicU16(host, "Block"),
            PublicStatuses(host, "Powers"),
            PublicStatuses(host, "Statuses"),
            new RuntimeV4ExpertGameplayIntent(
                enemy.Intent.ToString().ToLowerInvariant(),
                enemy.Intent == RuntimeV3GameplayIntent.Attack ? enemy.IntentDamage : null,
                enemy.Intent == RuntimeV3GameplayIntent.Attack ? enemy.IntentHits : null,
                null));
    }

    private static RuntimeV4ExpertGameplayAction ProjectAction(LegalActionReference action) =>
        new(action.ActionId, action.Kind, action.Value, action.TargetId, null);

    private List<RuntimeV4ExpertGameplayAction> ExpertActions(
        RuntimeV3GameplayObservation observation,
        Player? player,
        PlayerCombatState? combat)
    {
        var actions = LegalActions(observation).Select(ProjectAction).ToList();
        if (observation.State != RuntimeV3GameplayState.Combat
            || !observation.InputEnabled
            || player is null
            || combat is null)
        {
            return actions;
        }

        Creature[] enemies = CombatManager.Instance.DebugOnlyGetState()?.Enemies.ToArray()
            ?? Array.Empty<Creature>();
        foreach (PotionModel potion in player.Potions)
        {
            int slot = player.GetPotionSlotIndex(potion);
            if (slot < 0 || slot > byte.MaxValue || PotionUsable(player, potion, slot) != true)
            {
                continue;
            }
            string potionId = PotionId(potion, slot);
            string targetMode = PotionTargetMode(potion);
            if (targetMode == "any_enemy")
            {
                foreach (Creature enemy in enemies)
                {
                    if (!PotionAcceptsTarget(potion, enemy)) continue;
                    string enemyId = EnemyId(enemy);
                    actions.Add(new RuntimeV4ExpertGameplayAction(
                        $"potion:{observation.Generation}:{potionId}:{enemyId}",
                        "use_potion", potionId, enemyId, null));
                }
            }
            else if (targetMode == "self" && PotionAcceptsTarget(potion, player.Creature))
            {
                actions.Add(new RuntimeV4ExpertGameplayAction(
                    $"potion:{observation.Generation}:{potionId}:self",
                    "use_potion", potionId, "player:local", null));
            }
            else if (targetMode is "all_enemies" or "none")
            {
                actions.Add(new RuntimeV4ExpertGameplayAction(
                    $"potion:{observation.Generation}:{potionId}:none",
                    "use_potion", potionId, null, null));
            }
        }
        return actions;
    }

    private static string PotionId(PotionModel potion, int slot) =>
        $"potion:{slot}:{potion.Id.Entry}";

    private static string PotionTargetMode(PotionModel potion) =>
        PublicText(potion, "TargetType") switch
        {
            "Self" => "self",
            "AnyEnemy" => "any_enemy",
            "AllEnemies" => "all_enemies",
            "None" => "none",
            _ => "unknown"
        };

    private static bool PotionAcceptsTarget(PotionModel potion, Creature? target)
    {
        try
        {
            MethodInfo? method = potion.GetType().GetMethod(
                "IsValidTarget", BindingFlags.Instance | BindingFlags.Public,
                binder: null, new[] { typeof(Creature) }, modifiers: null);
            return method?.Invoke(potion, new object?[] { target }) as bool? == true;
        }
        catch (Exception)
        {
            return false;
        }
    }

    private static bool? PotionUsable(Player player, PotionModel potion, int slot)
    {
        try
        {
            if (NCombatRoom.Instance is { } room)
            {
                foreach (Node node in Descendants(room))
                {
                    if (!string.Equals(node.GetType().Name, "NPotionHolder", StringComparison.Ordinal)
                        || !ReferenceEquals(PublicValue(node, "Potion"), potion)) continue;
                    return PublicBool(node, "IsPotionUsable");
                }
            }
        }
        catch (Exception)
        {
            // A transition can invalidate the visual holder. Fall through to the model's
            // public usability check and keep the action catalog fail-closed.
        }
        return PublicBool(potion, "PassesCustomUsabilityCheck");
    }

    private static byte? OccupiedPotionCount(Player? player)
    {
        if (player is null) return null;
        try
        {
            int occupied = player.PotionSlots.Count(potion => potion is not null);
            return occupied is >= 0 and <= byte.MaxValue ? (byte)occupied : null;
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static byte? MaxPotionCount(Player? player)
    {
        if (player is null || player.MaxPotionCount is < 0 or > byte.MaxValue) return null;
        return (byte)player.MaxPotionCount;
    }

    /// <summary>
    /// Projects only map points currently rendered by the ordinary map screen. The run model also
    /// contains future map data, so reading its complete grid would disclose unrevealed content.
    /// </summary>
    private static RuntimeV4ExpertGameplayMapNode[]? VisibleMapNodes()
    {
        if (NMapScreen.Instance is not { IsOpen: true } map || !map.IsVisibleInTree()) return null;
        try
        {
            RunState? run = RunManager.Instance.DebugOnlyGetState();
            if (run is null) return null;
            return Descendants(map).OfType<NMapPoint>()
                .Where(point => point.IsVisibleInTree() && point.Point is not null)
                .Select(point => new RuntimeV4ExpertGameplayMapNode(
                    MapId(point, run),
                    ByteCoordinate(run.CurrentActIndex)
                        ?? throw new InvalidOperationException("map act is outside the wire bound"),
                    ByteCoordinate(point.Point.coord.row)
                        ?? throw new InvalidOperationException("map row is outside the wire bound"),
                    ByteCoordinate(point.Point.coord.col)
                        ?? throw new InvalidOperationException("map column is outside the wire bound"),
                    point.Point.PointType.ToString().ToLowerInvariant(),
                    point.State == MapPointState.Travelable))
                .OrderBy(node => node.Row)
                .ThenBy(node => node.Col)
                .ThenBy(node => node.NodeId, StringComparer.Ordinal)
                .ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static RuntimeV4ExpertGameplayMapEdge[]? VisibleMapEdges()
    {
        if (NMapScreen.Instance is not { IsOpen: true } map || !map.IsVisibleInTree()) return null;
        try
        {
            RunState? run = RunManager.Instance.DebugOnlyGetState();
            if (run is null) return null;
            var visible = Descendants(map).OfType<NMapPoint>()
                .Where(point => point.IsVisibleInTree() && point.Point is not null)
                .ToDictionary(point => MapId(point, run), StringComparer.Ordinal);
            return visible.Values
                .SelectMany(point => point.Point.Children
                    .Where(child => child is not null)
                    .Select(child => new { From = MapId(point, run), Child = child }))
                .Where(edge => visible.ContainsKey(MapId(edge.Child, run)))
                .Select(edge => new RuntimeV4ExpertGameplayMapEdge(
                    edge.From, MapId(edge.Child, run)))
                .Distinct()
                .OrderBy(edge => edge.From, StringComparer.Ordinal)
                .ThenBy(edge => edge.To, StringComparer.Ordinal)
                .ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static byte? ByteCoordinate(int value) =>
        value is >= 0 and <= byte.MaxValue ? (byte)value : null;

    private static RuntimeV4ExpertGameplayStatus[]? PublicStatuses(
        object? host,
        string propertyName)
    {
        object? raw = PublicValue(host, propertyName);
        if (raw is not IEnumerable values || raw is string) return null;
        var result = new List<RuntimeV4ExpertGameplayStatus>();
        try
        {
            foreach (object? value in values)
            {
                string? id = ReadNestedIdentity(value, "Id") ?? PublicText(value, "Name");
                string? name = PublicText(value, "Name") ?? id;
                if (id is null || name is null) return null;
                result.Add(new RuntimeV4ExpertGameplayStatus(id, name, PublicInt(value, "Amount")));
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static object? PublicValue(object? host, string propertyName) => host?.GetType()
        .GetProperty(propertyName, BindingFlags.Instance | BindingFlags.Public)?.GetValue(host);

    private static bool? PublicBool(object? host, string propertyName) => PublicValue(host, propertyName) switch
    {
        bool value => value,
        _ => null
    };

    private static ushort? PublicU16(object? host, string propertyName)
    {
        int? value = PublicInt(host, propertyName);
        return value is >= 0 and <= ushort.MaxValue ? (ushort)value.Value : null;
    }

    private static byte? PublicCount(object? host, string propertyName) => PublicValue(host, propertyName) switch
    {
        ICollection collection when collection.Count <= byte.MaxValue => (byte)collection.Count,
        _ => null
    };

    private static int? PublicInt(object? host, string propertyName) => PublicValue(host, propertyName) switch
    {
        int value => value,
        short value => value,
        byte value => value,
        long value when value is >= int.MinValue and <= int.MaxValue => (int)value,
        _ => null
    };

    private static string? PublicText(object? host, string propertyName) =>
        VisibleText(PublicValue(host, propertyName));

    private static string? VisibleText(object? value)
    {
        if (value is string text && RuntimeV3GameplayContract.IsText(text)) return text;
        if (value is Enum enumValue && RuntimeV3GameplayContract.IsIdentity(enumValue.ToString()))
            return enumValue.ToString();
        if (value is null) return null;
        foreach (string methodName in new[] { "GetFormattedText", "GetText" })
        {
            try
            {
                MethodInfo? method = value.GetType().GetMethod(
                    methodName, BindingFlags.Instance | BindingFlags.Public,
                    binder: null, Type.EmptyTypes, modifiers: null);
                if (method?.Invoke(value, null) is string formatted
                    && RuntimeV3GameplayContract.IsText(formatted)) return formatted;
            }
            catch (Exception)
            {
                // A missing/invalid localizer must leave the field unavailable.
            }
        }
        return null;
    }

    private static string? ReadNestedIdentity(object? host, params string[] properties)
    {
        object? current = host;
        foreach (string property in properties) current = PublicValue(current, property);
        return current switch
        {
            string value when RuntimeV3GameplayContract.IsIdentity(value) => value,
            _ => null
        };
    }
}
