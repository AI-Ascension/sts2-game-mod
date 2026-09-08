// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateSelector(
        RuntimeV4ExpertRestSelector selector,
        string optionId,
        out string error)
    {
        error = string.Empty;
        if (!RuntimeV4ExpertRestActionContract.IsIdentity(selector.SelectionId)
            || !RuntimeV4ExpertRestActionContract.IsSelectionKind(selector.SelectionKind)
            || selector.RequiredCount is < 1 or > RuntimeV4ExpertRestActionContract.MaxChoices
            || selector.SelectedChoiceIds.Count > RuntimeV4ExpertRestActionContract.MaxChoices
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
        }
        bool hasConfirm = selector.LegalActions.Any(a => a.Action.Kind == "confirm_selection");
        bool hasCancel = selector.LegalActions.Any(a => a.Action.Kind == "cancel_selection");
        if (!hasCancel || selector.RemainingCount == 0 && !hasConfirm
            || selector.RemainingCount > 0 && hasConfirm)
        {
            error = selector.RemainingCount > 0
                ? "rest selector catalog advertises early confirmation"
                : "rest selector catalog lacks its required control actions";
            return false;
        }
        return true;
    }
}
