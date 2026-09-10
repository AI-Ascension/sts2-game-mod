// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateTransition(
        JsonElement transition,
        ulong responseGeneration,
        string operationId,
        RuntimeV4ExpertRestActionReference action,
        JsonElement observation,
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
            || optionId != action.Action.RestOptionId
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
            if (action.Action.Kind != "rest_option"
                || RuntimeV4ExpertRestActionContract.SelectorOptionKinds.Contains(optionId))
                return Fail(out error, "rest option completion action is not bound");
            return HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "completed", "effect_witness")
                && transition.GetProperty("completed").ValueKind == JsonValueKind.True
                && transition.GetProperty("effect_witness").ValueKind == JsonValueKind.Object
                || Fail(out error, "rest option completion transition is invalid");
        }
        if (kind == "rest_option_selection_requested")
        {
            if (action.Action.Kind != "rest_option"
                || !RuntimeV4ExpertRestActionContract.SelectorOptionKinds.Contains(optionId))
                return Fail(out error, "rest selector request action is not bound");
            if (!HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "selector", "effect_witness")
                || transition.GetProperty("selector").ValueKind != JsonValueKind.Object
                || transition.GetProperty("effect_witness").ValueKind != JsonValueKind.Null)
                return Fail(out error, "rest selector request transition is invalid");
            return ValidateSelector(transition.GetProperty("selector"), optionId, observation,
                out _, out _, out _, out _, out _, out error);
        }
        if (kind == "rest_option_selection_progressed")
        {
            if (action.Action.Kind is not ("select_card" or "select_player"))
                return Fail(out error, "rest selector progress action is not bound");
            if (!HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "selection_id", "selection_kind", "required_count",
                    "selected_choice_ids", "remaining_count", "selector", "effect_witness")
                || transition.GetProperty("selector").ValueKind != JsonValueKind.Object
                || transition.GetProperty("effect_witness").ValueKind != JsonValueKind.Null)
                return Fail(out error, "rest selector progress transition is invalid");
            if (!ValidateSelector(transition.GetProperty("selector"), optionId, observation,
                    out string? selectorId, out string? selectorKind, out _, out _,
                    out _, out error))
                return false;
            if (StringField(transition, "selection_id") != selectorId
                || StringField(transition, "selection_kind") != selectorKind
                || !Int32Field(transition, "required_count", 1,
                    RuntimeV4ExpertRestActionContract.MaxChoices, out int required)
                || !BoundedIds(transition, "selected_choice_ids")
                || !Int32Field(transition, "remaining_count", 0,
                    RuntimeV4ExpertRestActionContract.MaxChoices, out int remaining)
                || !JsonElementDeepEquals(transition.GetProperty("selected_choice_ids"),
                    transition.GetProperty("selector").GetProperty("selected_choice_ids"))
                || required != transition.GetProperty("selector").GetProperty("required_count").GetInt32()
                || remaining != transition.GetProperty("selector").GetProperty("remaining_count").GetInt32()
                )
                return Fail(out error, "rest selector progress fields do not mirror selector");
            if (action.Action.SelectionId != selectorId
                || (selectorKind == "card" && action.Action.Kind != "select_card")
                || (selectorKind == "player" && action.Action.Kind != "select_player"))
                return Fail(out error, "rest selector progress action identity is invalid");
            string selectedId = selectorKind == "card"
                ? action.Action.CardId! : action.Action.PlayerId!;
            if (!transition.GetProperty("selected_choice_ids").EnumerateArray()
                    .Any(value => value.GetString() == selectedId))
                return Fail(out error, "rest selector progress action is not selected");
            return true;
        }
        if (kind == "rest_option_selection_completed")
        {
            hasWitness = true;
            if (action.Action.Kind is not ("confirm_selection" or "select_player")
                || !RuntimeV4ExpertRestActionContract.SelectorOptionKinds.Contains(optionId))
                return Fail(out error, "rest selector completion action is not bound");
            if (!HasExactFields(transition, "kind", "before_generation", "after_generation",
                    "rest_option_id", "selection_id", "selection_kind", "required_count",
                    "selected_choice_ids", "remaining_count", "completed", "effect_witness")
                || transition.GetProperty("completed").ValueKind != JsonValueKind.True
                || transition.GetProperty("remaining_count").ValueKind != JsonValueKind.Number
                || !transition.GetProperty("remaining_count").TryGetInt32(out int remainingCount)
                || remainingCount != 0
                || transition.GetProperty("effect_witness").ValueKind != JsonValueKind.Object)
                return Fail(out error, "rest selector completion transition is invalid");
            if (!IdentityField(transition, "selection_id")
                || StringField(transition, "selection_kind") is not ("card" or "player")
                || !Int32Field(transition, "required_count", 1,
                    RuntimeV4ExpertRestActionContract.MaxChoices, out int completedRequired)
                || !BoundedIds(transition, "selected_choice_ids")
                || transition.GetProperty("selected_choice_ids").GetArrayLength()
                    != completedRequired)
                return Fail(out error, "rest selector completion fields are invalid");
            string completedKind = StringField(transition, "selection_kind")!;
            if (optionId == "smith" && completedKind != "card"
                || optionId == "mend" && completedKind != "player"
                || action.Action.SelectionId != StringField(transition, "selection_id"))
                return Fail(out error, "rest selector completion identity is invalid");
            if (completedKind == "card" && action.Action.Kind != "confirm_selection"
                || completedKind == "player"
                    && action.Action.Kind is not ("confirm_selection" or "select_player"))
                return Fail(out error, "rest selector completion action kind is invalid");
            if (completedKind == "player" && action.Action.Kind == "select_player"
                && !transition.GetProperty("selected_choice_ids").EnumerateArray()
                    .Any(value => value.GetString() == action.Action.PlayerId))
                return Fail(out error, "rest selector completion target is not selected");
            return true;
        }
        error = "rest transition kind is unsupported";
        return false;
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
