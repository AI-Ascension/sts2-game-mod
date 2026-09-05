// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplaySupport
{
    private static bool TryAction(
        JsonElement root,
        ulong generation,
        out LegalActionReference? action)
    {
        action = null;
        if (!root.TryGetProperty("action", out JsonElement value)
            || value.ValueKind != JsonValueKind.Object
            || !HasExactFields(value, "action_id", "action")
            || !TryString(value, "action_id", out string? actionId)
            || !value.TryGetProperty("action", out JsonElement payload)
            || payload.ValueKind != JsonValueKind.Object
            || !TryString(payload, "kind", out string? kind))
        {
            return false;
        }
        if (!TryActionPayload(payload, kind, out string? selectedValue, out string? targetId))
        {
            return false;
        }
        if (actionId is null || kind is null)
        {
            return false;
        }
        action = new LegalActionReference(actionId, kind, selectedValue, targetId, generation);
        return action.Validate(out _);
    }

    private static bool TryActionPayload(JsonElement payload, string? kind,
        out string? selectedValue, out string? targetId)
    {
        selectedValue = null;
        targetId = null;
        string? field = kind switch
        {
            "start_run" => "character_id",
            "select_map_node" => "node_id",
            "choose_reward" => "reward_id",
            "shop_purchase" => "item_id",
            "shop_remove" or "smith" or "select_card" => "card_id",
            "event_choice" => "choice_id",
            "play_card" => "card_id",
            "end_turn" or "skip_reward" or "rest" or "confirm_victory" or "save_quit" => null,
            _ => "invalid"
        };
        if (field == "invalid")
        {
            return false;
        }
        if (field is null)
        {
            if (!HasExactFields(payload, "kind"))
            {
                return false;
            }
        }
        else if (kind == "play_card")
        {
            if (!HasExactFields(payload, "kind", "card_id", "target_id")
                || !TryString(payload, "card_id", out selectedValue)
                || !TryOptionalString(payload, "target_id", out targetId))
            {
                return false;
            }
        }
        else if (!HasExactFields(payload, "kind", field)
            || !TryString(payload, field, out selectedValue))
        {
            return false;
        }
        return true;
    }

}
