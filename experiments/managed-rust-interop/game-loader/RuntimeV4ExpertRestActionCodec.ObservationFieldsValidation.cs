// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateMapNode(JsonElement node, out string error)
    {
        error = string.Empty;
        return HasExactFields(node, "node_id", "act", "row", "col", "kind", "reachable")
            && IdentityField(node, "node_id") && ByteField(node, "act")
            && ByteField(node, "row") && ByteField(node, "col")
            && IdentityField(node, "kind") && BoolField(node, "reachable")
            || Fail(out error, "rest observation map node is invalid");
    }

    private static bool ValidateMapEdge(JsonElement edge, out string error)
    {
        error = string.Empty;
        return HasExactFields(edge, "from", "to") && IdentityField(edge, "from")
            && IdentityField(edge, "to") || Fail(out error, "rest observation map edge is invalid");
    }

    private static bool ValidateChoice(JsonElement choice, out string error)
    {
        error = string.Empty;
        return HasExactFields(choice, "choice_id", "label", "kind", "domain")
            && IdentityField(choice, "choice_id") && TextField(choice, "label")
            && IdentityField(choice, "kind")
            && ValidateOptionalIdentityArray(choice, "domain", 256, out error)
            || Fail(out error, "rest observation choice is invalid");
    }

    private static bool ValidateShopItem(JsonElement item, out string error)
    {
        error = string.Empty;
        return HasExactFields(item, "item_id", "name", "kind", "price")
            && IdentityField(item, "item_id") && TextField(item, "name")
            && IdentityField(item, "kind") && UInt32Field(item, "price")
            || Fail(out error, "rest observation shop item is invalid");
    }

    private static bool ValidateEnemy(JsonElement enemy, out string error)
    {
        error = string.Empty;
        return HasExactFields(enemy, "enemy_id", "name", "hp", "max_hp", "block", "powers",
                "statuses", "intent")
            && IdentityField(enemy, "enemy_id") && TextField(enemy, "name")
            && UInt16Field(enemy, "hp", out ushort hp)
            && UInt16Field(enemy, "max_hp", out ushort maxHp) && hp <= maxHp
            && NullableUInt16(enemy, "block")
            && ValidateCollection(enemy, "powers", ValidateStatus, out error)
            && ValidateCollection(enemy, "statuses", ValidateStatus, out error)
            && ValidateIntent(enemy.GetProperty("intent"), out error)
            || Fail(out error, "rest observation enemy is invalid");
    }

    private static bool ValidateIntent(JsonElement intent, out string error)
    {
        error = string.Empty;
        string? kind = StringField(intent, "kind");
        if (kind == "attack")
        {
            return HasExactFields(intent, "kind", "damage", "hits", "target_ids")
                && UInt16Field(intent, "damage") && ByteField(intent, "hits", 1)
                && ValidateOptionalIdentityArray(intent, "target_ids", 16, out error)
                || Fail(out error, "rest observation attack intent is invalid");
        }
        if (kind is "defend" or "buff" or "debuff" or "unknown")
        {
            return HasExactFields(intent, "kind", "target_ids")
                && ValidateOptionalIdentityArray(intent, "target_ids", 16, out error)
                || Fail(out error, "rest observation intent is invalid");
        }
        if (kind == "composite")
        {
            if (!HasExactFields(intent, "kind", "intents", "target_ids")
                || !ValidateOptionalIdentityArray(intent, "target_ids", 16, out error)
                || !intent.TryGetProperty("intents", out JsonElement components)
                || components.ValueKind != JsonValueKind.Array
                || components.GetArrayLength() is < 2 or > 16)
                return Fail(out error, "rest observation composite intent is invalid");
            foreach (JsonElement component in components.EnumerateArray())
                if (!ValidateIntentComponent(component, out error)) return false;
            return true;
        }
        return Fail(out error, "rest observation intent kind is unsupported");
    }

    private static bool ValidateIntentComponent(JsonElement intent, out string error)
    {
        error = string.Empty;
        string? kind = StringField(intent, "kind");
        if (kind == "attack")
            return HasExactFields(intent, "kind", "damage", "hits", "target_ids")
                && UInt16Field(intent, "damage") && ByteField(intent, "hits", 1)
                && ValidateOptionalIdentityArray(intent, "target_ids", 16, out error)
                || Fail(out error, "rest observation attack component is invalid");
        if (kind is "defend" or "buff" or "debuff" or "unknown")
            return HasExactFields(intent, "kind", "target_ids")
                && ValidateOptionalIdentityArray(intent, "target_ids", 16, out error)
                || Fail(out error, "rest observation intent component is invalid");
        return Fail(out error, "rest observation intent component kind is unsupported");
    }

    private static bool ValidateLegalActions(JsonElement actions, out string error)
    {
        error = string.Empty;
        if (actions.ValueKind != JsonValueKind.Array
            || actions.GetArrayLength() > RuntimeV3GameplayContract.MaxLegalActions)
            return Fail(out error, "rest observation legal actions are invalid");
        var ids = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement action in actions.EnumerateArray())
        {
            if (!HasExactFields(action, "action_id", "action")
                || !IdentityField(action, "action_id")
                || !ids.Add(StringField(action, "action_id")!)
                || !ValidateGameplayAction(action.GetProperty("action"), out error))
                return false;
        }
        return true;
    }

    private static bool ValidateGameplayAction(JsonElement action, out string error)
    {
        error = string.Empty;
        string? kind = StringField(action, "kind");
        if (kind is "end_turn" or "skip_reward" or "proceed" or "rest"
            or "confirm_victory" or "save_quit")
            return HasExactFields(action, "kind")
                || Fail(out error, "rest observation control action is invalid");
        if (kind is "start_run" or "select_character")
            return HasExactFields(action, "kind", "character_id")
                && IdentityField(action, "character_id")
                || Fail(out error, "rest observation character action is invalid");
        if (kind == "select_map_node")
            return HasExactFields(action, "kind", "node_id") && IdentityField(action, "node_id")
                || Fail(out error, "rest observation map action is invalid");
        if (kind is "play_card" or "use_potion")
        {
            string idField = kind == "play_card" ? "card_id" : "potion_id";
            return HasExactFields(action, "kind", idField, "target_id")
                && IdentityField(action, idField) && NullableIdentity(action, "target_id")
                || Fail(out error, "rest observation combat action is invalid");
        }
        if (kind == "rest_option")
            return HasExactFields(action, "kind", "rest_option_id")
                && IdentityField(action, "rest_option_id")
                || Fail(out error, "rest observation rest action is invalid");
        if (kind == "choose_reward")
            return HasExactFields(action, "kind", "reward_id") && IdentityField(action, "reward_id")
                || Fail(out error, "rest observation reward action is invalid");
        if (kind == "shop_purchase")
            return HasExactFields(action, "kind", "item_id") && IdentityField(action, "item_id")
                || Fail(out error, "rest observation purchase action is invalid");
        if (kind is "shop_remove" or "smith")
            return HasExactFields(action, "kind", "card_id") && IdentityField(action, "card_id")
                || Fail(out error, "rest observation card action is invalid");
        if (kind == "event_choice")
            return HasExactFields(action, "kind", "choice_id") && IdentityField(action, "choice_id")
                || Fail(out error, "rest observation event action is invalid");
        if (kind == "select_card")
            return HasExactFields(action, "kind", "selection_id", "card_id")
                && NullableIdentity(action, "selection_id") && IdentityField(action, "card_id")
                || Fail(out error, "rest observation card selection action is invalid");
        if (kind is "confirm_selection" or "cancel_selection")
            return HasExactFields(action, "kind", "selection_id")
                && NullableIdentity(action, "selection_id")
                || Fail(out error, "rest observation selection control is invalid");
        return Fail(out error, "rest observation action kind is unsupported");
    }

    private static bool UniqueObjectIds(
        JsonElement parent,
        string field,
        string idField,
        out string error)
    {
        error = string.Empty;
        JsonElement value = parent.GetProperty(field);
        if (value.ValueKind == JsonValueKind.Null) return true;
        var ids = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement item in value.EnumerateArray())
            if (!ids.Add(StringField(item, idField)!))
                return Fail(out error, "rest observation IDs are duplicated");
        return true;
    }

    private static bool ValidateIdentityArray(
        JsonElement parent,
        string field,
        int max,
        out string error) => ValidateOptionalIdentityArray(parent, field, max, out error, false);

    private static bool ValidateOptionalIdentityArray(
        JsonElement parent,
        string field,
        int max,
        out string error,
        bool nullable = true)
    {
        error = string.Empty;
        if (!parent.TryGetProperty(field, out JsonElement value))
            return Fail(out error, "rest observation ID array is missing");
        if (value.ValueKind == JsonValueKind.Null) return nullable;
        if (value.ValueKind != JsonValueKind.Array || value.GetArrayLength() > max)
            return Fail(out error, "rest observation ID array is invalid");
        var ids = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement item in value.EnumerateArray())
        {
            string? id = item.ValueKind == JsonValueKind.String ? item.GetString() : null;
            if (id is null || !RuntimeV4ExpertRestActionContract.IsIdentity(id)
                || !ids.Add(id))
                return Fail(out error, "rest observation ID array is invalid");
        }
        return true;
    }

}
