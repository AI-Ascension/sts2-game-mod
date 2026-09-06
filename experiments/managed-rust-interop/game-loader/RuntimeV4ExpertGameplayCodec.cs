// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Managed constants and closed JSON projection for runtime-v4-expert.</summary>
internal static class RuntimeV4ExpertGameplayCodec
{
    internal const string ProtocolVersion = "runtime-v4-expert";
    internal const string Artifact = "sts2-protocol/runtime-v4-expert";
    internal const string SchemaSource = "schemas/runtime-v4-expert.schema.json";
    internal const string Generator = "hand-authored";
    internal const string Profile = "expert-state";
    internal const string SchemaDigest = "f0786b039396043a441323447ac44f7cc4c218071bc477722f3ec992ab295a8a";

    internal static bool TrySerialize(
        RuntimeV4ExpertGameplayObservation observation,
        out string json,
        out string error)
    {
        json = string.Empty;
        error = string.Empty;
        if (!RuntimeV3GameplayContract.IsIdentity(observation.StateId)
            || observation.Generation > RuntimeV3GameplayContract.MaxGeneration
            || observation.VisibleSeed is { } seed && !RuntimeV3GameplayContract.IsText(seed)
            || observation.LegalActions.Count > RuntimeV3GameplayContract.MaxLegalActions
            || !RuntimeV3GameplayContract.IsIdentity(observation.State.Kind))
        {
            error = "runtime-v4-expert observation identity, generation, or catalog is invalid";
            return false;
        }

        var actions = new List<Dictionary<string, object?>>(observation.LegalActions.Count);
        var actionIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (RuntimeV4ExpertGameplayAction action in observation.LegalActions)
        {
            if (!RuntimeV3GameplayContract.IsIdentity(action.ActionId)
                || !actionIds.Add(action.ActionId)
                || action.Value is { } value && !RuntimeV3GameplayContract.IsIdentity(value)
                || action.TargetId is { } target && !RuntimeV3GameplayContract.IsIdentity(target)
                || action.SelectionId is { } selection && !RuntimeV3GameplayContract.IsIdentity(selection))
            {
                error = "runtime-v4-expert action identity is invalid or duplicated";
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
            ["protocol_version"] = ProtocolVersion,
            ["schema_digest"] = SchemaDigest,
            ["provenance"] = new Dictionary<string, object?>
            {
                ["artifact"] = Artifact,
                ["source"] = SchemaSource,
                ["generator"] = Generator
            },
            ["profile"] = Profile,
            ["state_id"] = observation.StateId,
            ["generation"] = observation.Generation,
            ["visible_seed"] = observation.VisibleSeed,
            ["run"] = RunObject(observation.Run),
            ["player"] = PlayerObject(observation.Player),
            ["state"] = StateObject(observation.State),
            ["legal_actions"] = actions
        };
        json = JsonSerializer.Serialize(root);
        return true;
    }

    private static Dictionary<string, object?> RunObject(RuntimeV4ExpertGameplayRun run) => new()
    {
        ["character_id"] = run.CharacterId,
        ["act"] = run.Act,
        ["location"] = run.Location
    };

    private static Dictionary<string, object?> PlayerObject(RuntimeV4ExpertGameplayPlayer player) => new()
    {
        ["hp"] = player.Hp,
        ["max_hp"] = player.MaxHp,
        ["block"] = player.Block,
        ["energy"] = player.Energy,
        ["gold"] = player.Gold,
        ["hand"] = Cards(player.Hand),
        ["deck"] = Cards(player.Deck),
        ["discard"] = Cards(player.Discard),
        ["exhaust"] = Cards(player.Exhaust),
        ["powers"] = Statuses(player.Powers),
        ["statuses"] = Statuses(player.Statuses),
        ["relics"] = Relics(player.Relics),
        ["potions"] = Potions(player.Potions),
        ["potion_slots"] = player.PotionSlots,
        ["max_potion_slots"] = player.MaxPotionSlots
    };

    private static Dictionary<string, object?>[]? Cards(IReadOnlyList<RuntimeV4ExpertGameplayCard>? cards) =>
        cards is null ? null : cards.Select(card => new Dictionary<string, object?>
        {
            ["card_id"] = card.CardId,
            ["name"] = card.Name,
            ["cost"] = card.Cost,
            ["upgraded"] = card.Upgraded,
            ["type"] = card.Type,
            ["rarity"] = card.Rarity,
            ["target"] = card.Target,
            ["description"] = card.Description
        }).ToArray();

    private static Dictionary<string, object?>[]? Statuses(IReadOnlyList<RuntimeV4ExpertGameplayStatus>? statuses) =>
        statuses is null ? null : statuses.Select(status => new Dictionary<string, object?>
        {
            ["status_id"] = status.StatusId,
            ["name"] = status.Name,
            ["amount"] = status.Amount
        }).ToArray();

    private static Dictionary<string, object?>[]? Relics(IReadOnlyList<RuntimeV4ExpertGameplayRelic>? relics) =>
        relics is null ? null : relics.Select(relic => new Dictionary<string, object?>
        {
            ["relic_id"] = relic.RelicId,
            ["name"] = relic.Name
        }).ToArray();

    private static Dictionary<string, object?>[]? Potions(IReadOnlyList<RuntimeV4ExpertGameplayPotion>? potions) =>
        potions is null ? null : potions.Select(potion => new Dictionary<string, object?>
        {
            ["potion_id"] = potion.PotionId,
            ["name"] = potion.Name,
            ["slot"] = potion.Slot,
            ["usable"] = potion.Usable,
            ["target_mode"] = potion.TargetMode
        }).ToArray();

    private static Dictionary<string, object?> StateObject(RuntimeV4ExpertGameplayState state)
    {
        var result = new Dictionary<string, object?> { ["state"] = state.Kind };
        switch (state.Kind)
        {
            case "setup":
                result["characters"] = state.Characters;
                break;
            case "map":
                result["current_node_id"] = state.CurrentNodeId;
                result["nodes"] = state.Nodes?.Select(MapNodeObject).ToArray();
                result["edges"] = state.Edges?.Select(MapEdgeObject).ToArray();
                result["options"] = state.Options;
                break;
            case "combat":
                result["turn_index"] = state.TurnIndex;
                result["enemies"] = state.Enemies?.Select(EnemyObject).ToArray();
                break;
            case "reward":
            case "event":
            case "rest":
            case "selection":
                result["choices"] = state.Choices?.Select(ChoiceObject).ToArray();
                break;
            case "shop":
                result["items"] = state.Items?.Select(ShopItemObject).ToArray();
                break;
            case "defeat":
                result["reason"] = state.Reason;
                break;
            case "recovery":
                result["code"] = state.Code;
                break;
            case "victory":
                break;
            default:
                throw new InvalidOperationException("runtime-v4-expert state kind is not supported");
        }
        return result;
    }

    private static Dictionary<string, object?> MapNodeObject(RuntimeV4ExpertGameplayMapNode node) => new()
    {
        ["node_id"] = node.NodeId, ["act"] = node.Act, ["row"] = node.Row,
        ["col"] = node.Col, ["kind"] = node.Kind, ["reachable"] = node.Reachable
    };

    private static Dictionary<string, object?> MapEdgeObject(RuntimeV4ExpertGameplayMapEdge edge) => new()
    {
        ["from"] = edge.From, ["to"] = edge.To
    };

    private static Dictionary<string, object?> EnemyObject(RuntimeV4ExpertGameplayEnemy enemy) => new()
    {
        ["enemy_id"] = enemy.EnemyId, ["name"] = enemy.Name, ["hp"] = enemy.Hp,
        ["max_hp"] = enemy.MaxHp, ["block"] = enemy.Block, ["powers"] = Statuses(enemy.Powers),
        ["statuses"] = Statuses(enemy.Statuses), ["intent"] = IntentObject(enemy.Intent)
    };

    private static Dictionary<string, object?> IntentObject(RuntimeV4ExpertGameplayIntent intent)
    {
        var result = new Dictionary<string, object?>
        {
            ["kind"] = intent.Kind,
            ["target_ids"] = intent.TargetIds
        };
        if (intent.Kind == "attack")
        {
            result["damage"] = intent.Damage;
            result["hits"] = intent.Hits;
        }
        return result;
    }

    private static Dictionary<string, object?> ChoiceObject(RuntimeV4ExpertGameplayChoice choice) => new()
    {
        ["choice_id"] = choice.ChoiceId, ["label"] = choice.Label,
        ["kind"] = choice.Kind, ["domain"] = choice.Domain
    };

    private static Dictionary<string, object?> ShopItemObject(RuntimeV4ExpertGameplayShopItem item) => new()
    {
        ["item_id"] = item.ItemId, ["name"] = item.Name,
        ["kind"] = item.Kind, ["price"] = item.Price
    };

    private static Dictionary<string, object?> ActionObject(RuntimeV4ExpertGameplayAction action)
    {
        var result = new Dictionary<string, object?> { ["kind"] = action.Kind };
        switch (action.Kind)
        {
            case "start_run":
            case "select_character": result["character_id"] = action.Value; break;
            case "select_map_node": result["node_id"] = action.Value; break;
            case "play_card": result["card_id"] = action.Value; result["target_id"] = action.TargetId; break;
            case "use_potion": result["potion_id"] = action.Value; result["target_id"] = action.TargetId; break;
            case "rest_option": result["rest_option_id"] = action.Value; break;
            case "choose_reward": result["reward_id"] = action.Value; break;
            case "shop_purchase": result["item_id"] = action.Value; break;
            case "shop_remove":
            case "smith": result["card_id"] = action.Value; break;
            case "event_choice": result["choice_id"] = action.Value; break;
            case "select_card": result["selection_id"] = action.SelectionId; result["card_id"] = action.Value; break;
            case "confirm_selection":
            case "cancel_selection": result["selection_id"] = action.SelectionId; break;
            case "end_turn":
            case "skip_reward":
            case "proceed":
            case "rest":
            case "confirm_victory":
            case "save_quit": break;
            default: throw new InvalidOperationException("runtime-v4-expert action kind is not supported");
        }
        return result;
    }
}
