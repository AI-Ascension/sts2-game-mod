// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateSelector(
        JsonElement selector,
        string optionId,
        JsonElement observation,
        out string? selectionId,
        out string? selectionKind,
        out int requiredCount,
        out string[] selectedChoiceIds,
        out int remainingCount,
        out string error)
    {
        selectionId = null;
        selectionKind = null;
        requiredCount = 0;
        selectedChoiceIds = Array.Empty<string>();
        remainingCount = 0;
        error = string.Empty;
        if (!HasExactFields(selector, "selection_id", "selection_kind", "required_count",
                "selected_choice_ids", "remaining_count", "legal_actions")
            || !RuntimeV4ExpertRestActionContract.IsIdentity(
                selectionId = StringField(selector, "selection_id"))
            || !RuntimeV4ExpertRestActionContract.IsSelectionKind(
                selectionKind = StringField(selector, "selection_kind"))
            || !Int32Field(selector, "required_count", 1,
                RuntimeV4ExpertRestActionContract.MaxChoices, out requiredCount)
            || !BoundedIds(selector, "selected_choice_ids", out selectedChoiceIds)
            || !Int32Field(selector, "remaining_count", 0,
                RuntimeV4ExpertRestActionContract.MaxChoices, out remainingCount)
            || remainingCount != requiredCount - selectedChoiceIds.Length
            || !selector.TryGetProperty("legal_actions", out JsonElement legalActions)
            || legalActions.ValueKind != JsonValueKind.Array
            || legalActions.GetArrayLength() == 0
            || legalActions.GetArrayLength() > RuntimeV4ExpertRestActionContract.MaxChoices)
        {
            return Fail(out error, "rest selector counts or identity are invalid");
        }
        if (selectionKind == "card" && optionId != "smith"
            || selectionKind == "player" && optionId != "mend")
            return Fail(out error, "rest selector kind does not match its option");

        JsonElement choices = observation.GetProperty("state").GetProperty("choices");
        if (observation.GetProperty("state").GetProperty("state").GetString() != "selection"
            || choices.ValueKind != JsonValueKind.Array || choices.GetArrayLength() == 0)
            return Fail(out error, "rest selector has no visible choice surface");
        var choiceIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement choice in choices.EnumerateArray())
            if (!IdentityField(choice, "choice_id")
                || !choiceIds.Add(StringField(choice, "choice_id")!))
                return Fail(out error, "rest selector choice IDs are invalid");

        var actionIds = new HashSet<string>(StringComparer.Ordinal);
        bool hasConfirm = false;
        bool hasCancel = false;
        bool hasChoice = false;
        foreach (JsonElement actionValue in legalActions.EnumerateArray())
        {
            if (!TryParseAction(actionValue,
                    out RuntimeV4ExpertRestActionReference? action, out error)
                || action is null || !actionIds.Add(action.ActionId)
                || action.Action.SelectionId != selectionId
                || action.Action.RestOptionId != optionId)
                return false;

            if (action.Action.Kind == "confirm_selection") hasConfirm = true;
            if (action.Action.Kind == "cancel_selection") hasCancel = true;
            if (selectionKind == "card" && action.Action.Kind == "select_card")
            {
                hasChoice = true;
                if (!choiceIds.Contains(action.Action.CardId!))
                    return Fail(out error, "rest card selector action is not visible");
            }
            else if (selectionKind == "player" && action.Action.Kind == "select_player")
            {
                hasChoice = true;
                if (!choiceIds.Contains(action.Action.PlayerId!))
                    return Fail(out error, "rest player selector action is not visible");
            }
            else if (action.Action.Kind is not ("confirm_selection" or "cancel_selection"))
                return Fail(out error, "rest selector catalog contains an unrelated action");
        }
        if (!hasCancel || remainingCount == 0 && !hasConfirm
            || remainingCount > 0 && hasConfirm
            || remainingCount > 0 && !hasChoice)
        {
            error = remainingCount > 0
                ? "rest selector catalog is not actionable"
                : "rest selector catalog lacks its required control actions";
            return false;
        }
        return true;
    }

    private static bool ValidateSelector(
        RuntimeV4ExpertRestSelector selector,
        string optionId,
        RuntimeV4ExpertGameplayObservation observation,
        out string error)
    {
        error = string.Empty;
        if (!RuntimeV4ExpertRestActionContract.IsIdentity(selector.SelectionId)
            || !RuntimeV4ExpertRestActionContract.IsSelectionKind(selector.SelectionKind)
            || selector.RequiredCount is < 1 or > RuntimeV4ExpertRestActionContract.MaxChoices
            || selector.SelectedChoiceIds.Count > RuntimeV4ExpertRestActionContract.MaxChoices
            || selector.SelectedChoiceIds.Any(id =>
                !RuntimeV4ExpertRestActionContract.IsIdentity(id))
            || selector.SelectedChoiceIds.Distinct(StringComparer.Ordinal).Count()
                != selector.SelectedChoiceIds.Count
            || selector.RemainingCount != selector.RequiredCount - selector.SelectedChoiceIds.Count
            || selector.RemainingCount < 0
            || selector.LegalActions.Count > RuntimeV4ExpertRestActionContract.MaxChoices)
        {
            error = "rest selector counts or identity are invalid";
            return false;
        }
        if (selector.SelectionKind == "card" && optionId != "smith"
            || selector.SelectionKind == "player" && optionId != "mend")
        {
            error = "rest selector kind does not match its option";
            return false;
        }
        if (observation.State.Kind != "selection" || observation.State.Choices is not { Count: > 0 })
        {
            error = "rest selector has no visible choice surface";
            return false;
        }
        var visibleChoiceIds = new HashSet<string>(
            observation.State.Choices.Select(choice => choice.ChoiceId), StringComparer.Ordinal);
        var actionIds = new HashSet<string>(StringComparer.Ordinal);
        foreach (RuntimeV4ExpertRestActionReference action in selector.LegalActions)
        {
            if (!actionIds.Add(action.ActionId)
                || !RuntimeV4ExpertRestActionContract.TryValidateAction(action, out error)
                || action.Action.SelectionId != selector.SelectionId
                || action.Action.RestOptionId != optionId)
                return false;
            if (selector.SelectionKind == "card"
                && action.Action.Kind is not ("select_card" or "confirm_selection" or "cancel_selection"))
            {
                error = "card selector catalog contains an unrelated action";
                return false;
            }
            if (selector.SelectionKind == "player"
                && action.Action.Kind is not ("select_player" or "confirm_selection" or "cancel_selection"))
            {
                error = "player selector catalog contains an unrelated action";
                return false;
            }
            if (action.Action.Kind == "select_card"
                && !visibleChoiceIds.Contains(action.Action.CardId!))
            {
                error = "rest card selector action is not visible";
                return false;
            }
            if (action.Action.Kind == "select_player"
                && !visibleChoiceIds.Contains(action.Action.PlayerId!))
            {
                error = "rest player selector action is not visible";
                return false;
            }
        }
        bool hasConfirm = selector.LegalActions.Any(a => a.Action.Kind == "confirm_selection");
        bool hasCancel = selector.LegalActions.Any(a => a.Action.Kind == "cancel_selection");
        bool hasChoice = selector.LegalActions.Any(a =>
            selector.SelectionKind == "card" && a.Action.Kind == "select_card"
            || selector.SelectionKind == "player" && a.Action.Kind == "select_player");
        if (!hasCancel || selector.RemainingCount == 0 && !hasConfirm
            || selector.RemainingCount > 0 && hasConfirm)
        {
            error = selector.RemainingCount > 0
                ? "rest selector catalog advertises early confirmation"
                : "rest selector catalog lacks its required control actions";
            return false;
        }
        if (selector.RemainingCount > 0 && !hasChoice)
        {
            error = "rest selector catalog is not actionable";
            return false;
        }
        return true;
    }
}
