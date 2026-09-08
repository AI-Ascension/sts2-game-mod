// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateTransition(
        JsonElement transition,
        ulong responseGeneration,
        string operationId,
        out string? optionId,
        out bool hasWitness,
        out string error)
    {
        optionId = null;
        hasWitness = false;
        error = string.Empty;
        string? kind = StringField(transition, "kind");
        optionId = StringField(transition, "rest_option_id");
        if (!RuntimeV4ExpertRestActionContract.IsOptionKind(optionId)
            || !UInt64Field(transition, "before_generation", out ulong before)
            || !UInt64Field(transition, "after_generation", responseGeneration)
            || responseGeneration <= before)
        {
            error = "rest transition identity or generation is invalid";
            return false;
        }
        if (kind == "rest_option_completed")
        {
            hasWitness = true;
            return HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "completed", "effect_witness")
                && transition.GetProperty("completed").ValueKind == JsonValueKind.True
                && transition.GetProperty("effect_witness").ValueKind == JsonValueKind.Object;
        }
        if (kind == "rest_option_selection_requested")
        {
            return HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "selector", "effect_witness")
                && transition.GetProperty("selector").ValueKind == JsonValueKind.Object
                && transition.GetProperty("effect_witness").ValueKind == JsonValueKind.Null;
        }
        if (kind == "rest_option_selection_progressed")
        {
            return HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "selection_id", "selection_kind", "required_count",
                    "selected_choice_ids", "remaining_count", "selector", "effect_witness")
                && transition.GetProperty("selector").ValueKind == JsonValueKind.Object
                && transition.GetProperty("effect_witness").ValueKind == JsonValueKind.Null;
        }
        if (kind == "rest_option_selection_completed")
        {
            hasWitness = true;
            return HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "selection_id", "selection_kind", "required_count",
                    "selected_choice_ids", "remaining_count", "completed", "effect_witness")
                && transition.GetProperty("completed").ValueKind == JsonValueKind.True
                && transition.GetProperty("remaining_count").GetInt32() == 0
                && transition.GetProperty("effect_witness").ValueKind == JsonValueKind.Object;
        }
        error = "rest transition kind is unsupported";
        return false;
    }

    private static bool ValidateObservation(JsonElement observation, ulong generation, out string error)
    {
        error = string.Empty;
        if (observation.ValueKind != JsonValueKind.Object
            || !observation.TryGetProperty("generation", out JsonElement value)
            || value.ValueKind != JsonValueKind.Number
            || !value.TryGetUInt64(out ulong observed)
            || observed != generation)
        {
            error = "rest response observation generation is invalid";
            return false;
        }
        return true;
    }

    private static bool ValidateActionReference(JsonElement value, out string error)
    {
        error = string.Empty;
        if (!TryParseAction(value, out RuntimeV4ExpertRestActionReference? action, out error)) return false;
        return RuntimeV4ExpertRestActionContract.TryValidateAction(action, out error);
    }

    private static bool TryParseAction(
        JsonElement value,
        out RuntimeV4ExpertRestActionReference? action,
        out string error)
    {
        action = null;
        error = string.Empty;
        if (!HasExactFields(value, "action_id", "action")
            || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(value, "action_id")))
            return Fail(out error, "rest action reference fields are invalid");
        JsonElement payload = value.GetProperty("action");
        if (!TryParseActionPayload(payload, out RuntimeV4ExpertRestAction? parsed, out error)) return false;
        action = new RuntimeV4ExpertRestActionReference(value.GetProperty("action_id").GetString()!, parsed!);
        return true;
    }

    private static bool TryParseActionPayload(
        JsonElement value,
        out RuntimeV4ExpertRestAction? action,
        out string error)
    {
        action = null;
        error = string.Empty;
        string? kind = StringField(value, "kind");
        if (kind == "rest_option"
            && HasExactFields(value, "kind", "rest_option_id")
            && RuntimeV4ExpertRestActionContract.IsIdentity(StringField(value, "rest_option_id")))
        {
            action = new RuntimeV4ExpertRestAction(kind, value.GetProperty("rest_option_id").GetString()!);
            return RuntimeV4ExpertRestActionContract.TryValidateAction(
                new RuntimeV4ExpertRestActionReference("action", action), out error);
        }
        if (kind == "select_card"
            && HasExactFields(value, "kind", "selection_id", "rest_option_id", "card_id"))
        {
            action = new RuntimeV4ExpertRestAction(kind,
                value.GetProperty("rest_option_id").GetString()!,
                value.GetProperty("selection_id").GetString()!,
                value.GetProperty("card_id").GetString()!);
            return RuntimeV4ExpertRestActionContract.TryValidateAction(
                new RuntimeV4ExpertRestActionReference("action", action), out error);
        }
        if (kind == "select_player"
            && HasExactFields(value, "kind", "selection_id", "rest_option_id", "player_id"))
        {
            action = new RuntimeV4ExpertRestAction(kind,
                value.GetProperty("rest_option_id").GetString()!,
                value.GetProperty("selection_id").GetString()!,
                PlayerId: value.GetProperty("player_id").GetString());
            return RuntimeV4ExpertRestActionContract.TryValidateAction(
                new RuntimeV4ExpertRestActionReference("action", action), out error);
        }
        if (kind is "confirm_selection" or "cancel_selection"
            && HasExactFields(value, "kind", "selection_id", "rest_option_id"))
        {
            action = new RuntimeV4ExpertRestAction(kind,
                value.GetProperty("rest_option_id").GetString()!,
                value.GetProperty("selection_id").GetString()!);
            return RuntimeV4ExpertRestActionContract.TryValidateAction(
                new RuntimeV4ExpertRestActionReference("action", action), out error);
        }
        return Fail(out error, "rest action payload is not a typed closed arm");
    }
}
