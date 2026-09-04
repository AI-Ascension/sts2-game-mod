// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Explicit snake-case codec for the neutral fair-play projection.</summary>
internal static class RuntimeV3GameplayCodec
{
    internal static bool TrySerialize(
        RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> legalActions,
        out string json,
        out string error)
    {
        json = string.Empty;
        error = string.Empty;
        if (!observation.Validate(out error)
            || legalActions.Count > RuntimeV3GameplayContract.MaxLegalActions)
        {
            error = string.IsNullOrEmpty(error)
                ? "legal-action catalog exceeds its bound"
                : error;
            return false;
        }
        var actions = new List<Dictionary<string, object?>>(legalActions.Count);
        var actionIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (LegalActionReference action in legalActions)
        {
            if (!action.Validate(out error)
                || action.Generation != observation.Generation
                || !actionIds.Add(action.ActionId))
            {
                error = string.IsNullOrEmpty(error)
                    ? "legal-action catalog identity or generation is invalid"
                    : error;
                return false;
            }
            actions.Add(new Dictionary<string, object?>
            {
                ["action_id"] = action.ActionId,
                ["action"] = ActionObject(action)
            });
        }

        var root = new Dictionary<string, object?>
        {
            ["state_id"] = observation.StateId,
            ["generation"] = observation.Generation,
            ["visible_seed"] = observation.VisibleSeed,
            ["player"] = PlayerObject(observation.Player),
            ["state"] = StateObject(observation),
            ["legal_actions"] = actions
        };
        json = JsonSerializer.Serialize(root);
        using JsonDocument document = JsonDocument.Parse(json);
        if (!PrivilegedFieldGuard.IsSafeJson(document.RootElement, out error))
        {
            json = string.Empty;
            return false;
        }
        return true;
    }

    private static Dictionary<string, object?> PlayerObject(RuntimeV3GameplayPlayer player) =>
        new()
        {
            ["hp"] = player.Hp,
            ["max_hp"] = player.MaxHp,
            ["energy"] = player.Energy,
            ["gold"] = player.Gold,
            ["hand"] = Cards(player.Hand),
            ["deck"] = Cards(player.Deck),
            ["discard"] = Cards(player.Discard),
            ["exhaust"] = Cards(player.Exhaust)
        };

    private static List<Dictionary<string, object?>> Cards(IReadOnlyList<RuntimeV3GameplayCard> cards)
    {
        var result = new List<Dictionary<string, object?>>(cards.Count);
        foreach (RuntimeV3GameplayCard card in cards)
        {
            result.Add(new Dictionary<string, object?>
            {
                ["card_id"] = card.CardId,
                ["name"] = card.Name,
                ["cost"] = card.Cost,
                ["upgraded"] = card.Upgraded
            });
        }
        return result;
    }

    private static Dictionary<string, object?> StateObject(RuntimeV3GameplayObservation observation)
    {
        return observation.State switch
        {
            RuntimeV3GameplayState.Setup => new() { ["state"] = "setup", ["characters"] = observation.StateValues },
            RuntimeV3GameplayState.Map => new() { ["state"] = "map", ["node_id"] = observation.NodeId, ["options"] = observation.StateValues },
            RuntimeV3GameplayState.Combat => new() { ["state"] = "combat", ["turn_index"] = observation.TurnIndex, ["enemies"] = Enemies(observation.Enemies) },
            RuntimeV3GameplayState.Reward => new() { ["state"] = "reward", ["options"] = observation.StateValues },
            RuntimeV3GameplayState.Shop => new() { ["state"] = "shop", ["items"] = ShopItems(observation.ShopItems) },
            RuntimeV3GameplayState.Event => new() { ["state"] = "event", ["choices"] = observation.StateValues },
            RuntimeV3GameplayState.Rest => new() { ["state"] = "rest", ["options"] = observation.StateValues },
            RuntimeV3GameplayState.Selection => new() { ["state"] = "selection", ["choices"] = observation.StateValues },
            RuntimeV3GameplayState.Victory => new() { ["state"] = "victory" },
            RuntimeV3GameplayState.Defeat => new() { ["state"] = "defeat", ["reason"] = observation.StateValues.Count == 0 ? null : observation.StateValues[0] },
            RuntimeV3GameplayState.Recovery => new() { ["state"] = "recovery", ["code"] = observation.StateValues.Count == 0 ? "recovery" : observation.StateValues[0] },
            _ => new() { ["state"] = "recovery", ["code"] = "unknown_state" }
        };
    }

    private static List<Dictionary<string, object?>> Enemies(IReadOnlyList<RuntimeV3GameplayEnemy> enemies)
    {
        var result = new List<Dictionary<string, object?>>(enemies.Count);
        foreach (RuntimeV3GameplayEnemy enemy in enemies)
        {
            result.Add(new Dictionary<string, object?>
            {
                ["enemy_id"] = enemy.EnemyId,
                ["name"] = enemy.Name,
                ["hp"] = enemy.Hp,
                ["max_hp"] = enemy.MaxHp,
                ["intent"] = IntentObject(enemy)
            });
        }
        return result;
    }

    private static Dictionary<string, object?> IntentObject(RuntimeV3GameplayEnemy enemy) =>
        enemy.Intent == RuntimeV3GameplayIntent.Attack
            ? new() { ["kind"] = "attack", ["damage"] = enemy.IntentDamage, ["hits"] = enemy.IntentHits }
            : new() { ["kind"] = enemy.Intent.ToString().ToLowerInvariant() };

    private static List<Dictionary<string, object?>> ShopItems(IReadOnlyList<RuntimeV3GameplayShopItem> items)
    {
        var result = new List<Dictionary<string, object?>>(items.Count);
        foreach (RuntimeV3GameplayShopItem item in items)
        {
            result.Add(new Dictionary<string, object?>
            {
                ["item_id"] = item.ItemId,
                ["name"] = item.Name,
                ["price"] = item.Price
            });
        }
        return result;
    }

    private static Dictionary<string, object?> ActionObject(LegalActionReference action)
    {
        var value = new Dictionary<string, object?> { ["kind"] = action.Kind };
        string? field = action.Kind switch
        {
            "start_run" => "character_id",
            "select_map_node" => "node_id",
            "choose_reward" => "reward_id",
            "shop_purchase" => "item_id",
            "shop_remove" or "smith" or "select_card" => "card_id",
            "event_choice" => "choice_id",
            "play_card" => "card_id",
            _ => null
        };
        if (field is not null)
        {
            value[field] = action.Value;
        }
        if (action.Kind == "play_card")
        {
            value["target_id"] = action.TargetId;
        }
        return value;
    }
}
