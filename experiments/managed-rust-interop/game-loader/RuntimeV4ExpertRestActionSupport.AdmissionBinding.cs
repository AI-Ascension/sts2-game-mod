// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV4ExpertRestActionSupport
{
    private static RuntimeV4ExpertRestSelector? SnapshotSelector(
        RuntimeV4ExpertRestSelector? selector) => selector is null ? null : selector with
        {
            SelectedChoiceIds = new List<string>(selector.SelectedChoiceIds).AsReadOnly(),
            LegalActions = new List<RuntimeV4ExpertRestActionReference>(selector.LegalActions)
                .AsReadOnly()
        };

    /// <summary>
    /// Binds a selector completion to the catalog captured at admission. The wire validator checks
    /// shape and mirrors; this receipt also checks native choices before reconciliation.
    /// </summary>
    private static bool CompletionMatchesAdmission(
        RuntimeV4ExpertRestReceipt receipt,
        RuntimeV4ExpertRestHostCompletion completion)
    {
        RuntimeV4ExpertRestSelector selector = receipt.AdmissionSelector!;
        if (!ContainsExact(selector.LegalActions, receipt.Action)) return false;
        if (completion.Status == "cancelled")
        {
            return receipt.Action.Action.Kind == "cancel_selection"
                && completion.ErrorCode == "sts2.game-mod/selection_cancelled"
                && completion.Observation is null && completion.Transition is null
                && completion.EffectWitness is null;
        }
        if (completion.Status != "settled" || completion.Transition is null
            || completion.Transition.BeforeGeneration != receipt.Generation
            || completion.Transition.RestOptionId
                != (selector.SelectionKind == "card" ? "smith" : "mend"))
            return false;

        return completion.Transition switch
        {
            RuntimeV4ExpertRestSelectionProgressedTransition progressed =>
                ProgressionMatches(selector, receipt.Action, progressed.Selector),
            RuntimeV4ExpertRestSelectionCompletedTransition completed =>
                CompletionMatches(selector, receipt.Action, completed),
            _ => false
        };
    }

    private static bool ProgressionMatches(
        RuntimeV4ExpertRestSelector prior,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestSelector next)
    {
        if (!SelectorIdentityMatches(prior, next)
            || action.Action.Kind is not ("select_card" or "select_player"))
            return false;
        string? selectedId = action.Action.Kind == "select_card"
            ? action.Action.CardId : action.Action.PlayerId;
        if (selectedId is null || prior.SelectedChoiceIds.Contains(selectedId)) return false;
        var expected = new List<string>(prior.SelectedChoiceIds) { selectedId };
        return ChoiceIdsMatch(next.SelectedChoiceIds, expected)
            && NextChoiceCatalogIsBound(prior, next)
            && next.RemainingCount == prior.RequiredCount - expected.Count;
    }

    private static bool CompletionMatches(
        RuntimeV4ExpertRestSelector prior,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestSelectionCompletedTransition completed)
    {
        if (completed.SelectionId != prior.SelectionId
            || completed.SelectionKind != prior.SelectionKind
            || completed.RequiredCount != prior.RequiredCount
            || completed.SelectedChoiceIds.Count != prior.RequiredCount)
            return false;
        var expected = new List<string>(prior.SelectedChoiceIds);
        if (action.Action.Kind == "select_player")
        {
            if (prior.SelectionKind != "player" || action.Action.PlayerId is null
                || expected.Contains(action.Action.PlayerId))
                return false;
            expected.Add(action.Action.PlayerId);
        }
        else if (action.Action.Kind != "confirm_selection"
            || prior.RemainingCount != 0)
        {
            return false;
        }
        return ContainsExact(prior.LegalActions, action)
            && ChoiceIdsMatch(completed.SelectedChoiceIds, expected);
    }

    private static bool NextChoiceCatalogIsBound(
        RuntimeV4ExpertRestSelector prior,
        RuntimeV4ExpertRestSelector next)
    {
        var priorChoiceIds = new HashSet<string>(
            prior.LegalActions
                .Where(action => action.Action.Kind is "select_card" or "select_player")
                .Select(action => action.Action.Kind == "select_card"
                    ? action.Action.CardId! : action.Action.PlayerId!),
            StringComparer.Ordinal);
        foreach (RuntimeV4ExpertRestActionReference action in next.LegalActions)
        {
            if (action.Action.Kind is not ("select_card" or "select_player")) continue;
            string? choiceId = action.Action.Kind == "select_card"
                ? action.Action.CardId : action.Action.PlayerId;
            if (choiceId is null || !priorChoiceIds.Contains(choiceId)
                || next.SelectedChoiceIds.Contains(choiceId)) return false;
        }
        return true;
    }

    private static bool SelectorIdentityMatches(
        RuntimeV4ExpertRestSelector prior,
        RuntimeV4ExpertRestSelector next) =>
        next.SelectionId == prior.SelectionId
            && next.SelectionKind == prior.SelectionKind
            && next.RequiredCount == prior.RequiredCount;

    private static bool ChoiceIdsMatch(
        IReadOnlyList<string> actual,
        List<string> expected)
    {
        if (actual.Count != expected.Count) return false;
        for (int index = 0; index < expected.Count; index++)
            if (actual[index] != expected[index]) return false;
        return true;
    }
}
