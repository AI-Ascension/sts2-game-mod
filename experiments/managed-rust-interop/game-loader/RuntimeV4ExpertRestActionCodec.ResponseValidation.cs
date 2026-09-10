// SPDX-License-Identifier: MIT

using System;
using System.Linq;

namespace AiAscension.Sts2GameMod.Runtime;

internal static partial class RuntimeV4ExpertRestActionCodec
{
    private static bool ValidateRequest(RuntimeV4ExpertRestRequest request, out string error)
    {
        error = string.Empty;
        if (request.Context is null || !RuntimeV4ExpertRestActionContract.IsIdentity(
                request.Context.InstanceId)
            || !RuntimeV4ExpertRestActionContract.IsIdentity(request.Context.SessionId)
            || !RuntimeV4ExpertRestActionContract.IsIdentity(request.Context.LeaseId)
            || !RuntimeV4ExpertRestActionContract.IsIdentity(request.Context.CorrelationId)
            || request.Context.LeaseEpoch > RuntimeV3GameplayContract.MaxGeneration
            || request.Generation > RuntimeV3GameplayContract.MaxGeneration
            || !RuntimeV4ExpertRestActionContract.IsIdentity(request.StateId)
            || request.Operation is null
            || request.Operation.InstanceId != request.Context.InstanceId
            || request.Operation.SessionId != request.Context.SessionId
            || request.Operation.LeaseId != request.Context.LeaseId
            || request.Operation.LeaseEpoch != request.Context.LeaseEpoch
            || !RuntimeV4ExpertRestActionContract.IsIdentity(request.Operation.OperationId)
            || !RuntimeV4ExpertRestActionContract.TryValidateAction(request.Action, out error))
        {
            if (error.Length == 0) error = "rest action request values are invalid";
            return false;
        }
        return true;
    }

    private static bool ValidateResponse(RuntimeV4ExpertRestResponse response, out string error)
    {
        error = string.Empty;
        if (response.Context is null || !RuntimeV4ExpertRestActionContract.IsIdentity(
                response.Context.InstanceId)
            || !RuntimeV4ExpertRestActionContract.IsIdentity(response.Context.SessionId)
            || !RuntimeV4ExpertRestActionContract.IsIdentity(response.Context.LeaseId)
            || !RuntimeV4ExpertRestActionContract.IsIdentity(response.Context.CorrelationId)
            || response.Context.LeaseEpoch > RuntimeV3GameplayContract.MaxGeneration
            || response.Generation > RuntimeV3GameplayContract.MaxGeneration
            || !RuntimeV4ExpertRestActionContract.IsIdentity(response.StateId)
            || response.Operation is null
            || response.Operation.InstanceId != response.Context.InstanceId
            || response.Operation.SessionId != response.Context.SessionId
            || response.Operation.LeaseId != response.Context.LeaseId
            || response.Operation.LeaseEpoch != response.Context.LeaseEpoch
            || !RuntimeV4ExpertRestActionContract.IsIdentity(response.Operation.OperationId))
        {
            error = "rest action response envelope identity is invalid";
            return false;
        }
        if (response.Status is not ("accepted" or "settled" or "rejected" or "unknown" or "cancelled"))
        {
            error = "rest action response status is invalid";
            return false;
        }
        if (response.Action is not null
            && !RuntimeV4ExpertRestActionContract.TryValidateAction(response.Action, out error))
            return false;
        if (response.Status == "accepted")
        {
            if (response.Action is null || response.Observation is not null
                || response.Transition is not null || response.EffectWitness is not null
                || response.ErrorCode is not null)
            {
                error = "accepted rest response values are invalid";
                return false;
            }
            return true;
        }
        if (response.Status is "rejected" or "unknown" or "cancelled")
        {
            if (response.Action is null || response.Observation is not null
                || response.Transition is not null || response.EffectWitness is not null
                || !RuntimeV4ExpertRestActionContract.IsIdentity(response.ErrorCode))
            {
                error = "non-settled rest response values are invalid";
                return false;
            }
            return true;
        }
        if (response.Action is null || response.Observation is null
            || response.Transition is null || response.ErrorCode is not null)
        {
            error = "settled rest response values are invalid";
            return false;
        }
        if (!ValidateObservationValue(response.Observation, response.StateId,
                response.Generation, out error)) return false;
        if (response.Transition.RestOptionId != response.Action.Action.RestOptionId
            || response.Transition.AfterGeneration != response.Generation
            || response.Transition.AfterGeneration <= response.Transition.BeforeGeneration)
        {
            error = "rest transition generation is not fenced";
            return false;
        }
        if (response.Transition is RuntimeV4ExpertRestSelectionRequestedTransition requested)
        {
            if (response.Action.Action.Kind != "rest_option"
                || !RuntimeV4ExpertRestActionContract.SelectorOptionKinds.Contains(
                    response.Transition.RestOptionId)
                || response.EffectWitness is not null || requested.Selector is null)
            {
                error = "selection request cannot carry an effect witness";
                return false;
            }
            return ValidateSelector(requested.Selector, requested.RestOptionId,
                response.Observation, out error);
        }
        if (response.Transition is RuntimeV4ExpertRestSelectionProgressedTransition progressed)
        {
            if (response.Action.Action.Kind is not ("select_card" or "select_player")
                || response.EffectWitness is not null || progressed.Selector is null)
            {
                error = "selection progress cannot carry an effect witness";
                return false;
            }
            return ValidateSelector(progressed.Selector, progressed.RestOptionId,
                response.Observation, out error);
        }
        if (response.Transition is RuntimeV4ExpertRestSelectionCompletedTransition completed)
        {
            if (response.EffectWitness is null
                || completed.EffectWitness != response.EffectWitness
                || completed.SelectionId != response.Action.Action.SelectionId
                || completed.RestOptionId != response.Action.Action.RestOptionId)
            {
                error = "selection completion witness or selection identity is invalid";
                return false;
            }
            if (response.Action.Action.Kind is not ("confirm_selection" or "select_player")
                || (completed.RestOptionId == "smith"
                    && (completed.SelectionKind != "card"
                        || response.Action.Action.Kind != "confirm_selection"))
                || (completed.RestOptionId == "mend"
                    && (completed.SelectionKind != "player"
                        || response.Action.Action.Kind is not ("confirm_selection" or "select_player")))
                || completed.SelectedChoiceIds.Count != completed.RequiredCount
                || completed.SelectedChoiceIds.Count != completed.SelectedChoiceIds
                    .Distinct(StringComparer.Ordinal).Count())
            {
                error = "selection completion fields are invalid";
                return false;
            }
            if (!ValidateWitness(response.EffectWitness, response.Generation,
                    response.Operation, completed.RestOptionId, response.StateId,
                    out error)) return false;
            return ValidateTypedCompletedWitnessBinding(response.EffectWitness, completed,
                response.Action, out error);
        }
        if (response.Transition is RuntimeV4ExpertRestCompletedTransition completedOption)
        {
            if (response.Action.Action.Kind != "rest_option"
                || RuntimeV4ExpertRestActionContract.SelectorOptionKinds.Contains(
                    completedOption.RestOptionId)
                || response.EffectWitness is null
                || completedOption.EffectWitness != response.EffectWitness)
            {
                error = "rest completion witness is missing or duplicated incorrectly";
                return false;
            }
            return ValidateWitness(response.EffectWitness, response.Generation,
                response.Operation, completedOption.RestOptionId, response.StateId,
                out error);
        }
        error = "rest transition type is unsupported";
        return false;
    }

    private static bool ValidateTypedCompletedWitnessBinding(
        RuntimeV4ExpertRestEffectWitness witness,
        RuntimeV4ExpertRestSelectionCompletedTransition transition,
        RuntimeV4ExpertRestActionReference action,
        out string error)
    {
        error = string.Empty;
        if (transition.RestOptionId == "smith"
            && (witness.Evidence is not RuntimeV4ExpertRestCardEvidence cards
                || !transition.SelectedChoiceIds.SequenceEqual(
                    cards.UpgradedCardIds, StringComparer.Ordinal)))
            return Fail(out error, "Smith completion witness does not bind selected cards");
        if (transition.RestOptionId == "mend"
            && (witness.TargetPlayerId is null
                || transition.SelectedChoiceIds.Count != 1
                || transition.SelectedChoiceIds[0] != witness.TargetPlayerId
                || action.Action.Kind == "select_player"
                    && action.Action.PlayerId != witness.TargetPlayerId))
            return Fail(out error, "Mend completion witness does not bind selected player");
        return true;
    }
}
