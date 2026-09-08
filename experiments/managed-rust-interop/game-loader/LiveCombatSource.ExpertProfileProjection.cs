// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Godot;
using MegaCrit.Sts2.Core.Combat;
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
            enemy.EnemyId, enemy.Name, enemy.Hp, enemy.MaxHp, PublicU16(host, "Block"),
            PublicStatuses(host, "Powers"), PublicStatuses(host, "Statuses"),
            new RuntimeV4ExpertGameplayIntent(
                enemy.Intent.ToString().ToLowerInvariant(),
                enemy.Intent == RuntimeV3GameplayIntent.Attack ? enemy.IntentDamage : null,
                enemy.Intent == RuntimeV3GameplayIntent.Attack ? enemy.IntentHits : null, null));
    }

    private static RuntimeV4ExpertGameplayAction ProjectAction(LegalActionReference action) =>
        new(action.ActionId, action.Kind, action.Value, action.TargetId, null);

    private List<RuntimeV4ExpertGameplayAction> ExpertActions(
        RuntimeV3GameplayObservation observation, Player? player, PlayerCombatState? combat)
    {
        var actions = LegalActions(observation).Select(ProjectAction).ToList();
        if (observation.State != RuntimeV3GameplayState.Combat || !observation.InputEnabled
            || player is null || combat is null) return actions;
        Creature[] enemies = CombatManager.Instance.DebugOnlyGetState()?.Enemies.ToArray()
            ?? Array.Empty<Creature>();
        foreach (PotionModel potion in player.Potions)
        {
            int slot = player.GetPotionSlotIndex(potion);
            if (slot < 0 || slot > byte.MaxValue || PotionUsable(player, potion, slot) != true)
                continue;
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

    private static string PotionTargetMode(PotionModel potion) => PublicText(potion, "TargetType") switch
    {
        "Self" => "self", "AnyEnemy" => "any_enemy", "AllEnemies" => "all_enemies",
        "None" => "none", _ => "unknown"
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
                    if (string.Equals(node.GetType().Name, "NPotionHolder", StringComparison.Ordinal)
                        && ReferenceEquals(PublicValue(node, "Potion"), potion))
                        return PublicBool(node, "IsPotionUsable");
            }
        }
        catch (Exception)
        {
            // A transition can invalidate the visual holder. The model check below fails closed.
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

    private static byte? MaxPotionCount(Player? player) =>
        player is null || player.MaxPotionCount is < 0 or > byte.MaxValue
            ? null : (byte)player.MaxPotionCount;

    /// <summary>Projects map points currently rendered by the ordinary map screen.</summary>
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
                .OrderBy(node => node.Row).ThenBy(node => node.Col)
                .ThenBy(node => node.NodeId, StringComparer.Ordinal).ToArray();
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
            return visible.Values.SelectMany(point => point.Point.Children
                    .Where(child => child is not null)
                    .Select(child => new { From = MapId(point, run), Child = child }))
                .Where(edge => visible.ContainsKey(MapId(edge.Child, run)))
                .Select(edge => new RuntimeV4ExpertGameplayMapEdge(edge.From,
                    MapId(edge.Child, run))).Distinct()
                .OrderBy(edge => edge.From, StringComparer.Ordinal)
                .ThenBy(edge => edge.To, StringComparer.Ordinal).ToArray();
        }
        catch (Exception)
        {
            return null;
        }
    }

    private static byte? ByteCoordinate(int value) =>
        value is >= 0 and <= byte.MaxValue ? (byte)value : null;

}
