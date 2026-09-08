// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Closed JSON projection and strict request/response checks for the inactive rest profile.
/// The projection is deliberately separate from the potion codec so a future adoption cannot
/// silently widen the existing potion wire contract.
/// </summary>
internal static class RuntimeV4ExpertRestActionCodec
{
    internal static bool TrySerializeRequest(
        RuntimeV4ExpertRestRequest request,
        out string json,
        out string error)
    {
        json = string.Empty;
        if (!ValidateRequest(request, out error)) return false;
        json = JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV4ExpertRestActionContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV4ExpertRestActionContract.SchemaDigest,
            ["provenance"] = Provenance(),
            ["profile"] = RuntimeV4ExpertRestActionContract.Profile,
            ["correlation_id"] = request.Context.CorrelationId,
            ["instance_id"] = request.Context.InstanceId,
            ["session_id"] = request.Context.SessionId,
            ["lease_id"] = request.Context.LeaseId,
            ["lease_epoch"] = request.Context.LeaseEpoch,
            ["generation"] = request.Generation,
            ["state_id"] = request.StateId,
            ["operation_id"] = request.Operation.OperationId,
            ["kind"] = "action_request",
            ["action"] = ActionReference(request.Action),
            ["status"] = null,
            ["observation"] = null,
            ["transition"] = null,
            ["effect_witness"] = null,
            ["error_code"] = null
        });
        return true;
    }

    internal static bool TrySerializeResponse(
        RuntimeV4ExpertRestResponse response,
        out string json,
        out string error)
    {
        json = string.Empty;
        if (!ValidateResponse(response, out error)) return false;

        JsonElement? observation = null;
        if (response.Observation is not null)
        {
            if (!RuntimeV4ExpertGameplayCodec.TrySerialize(
                    response.Observation, out string observationJson, out string observationError))
            {
                error = observationError.Length == 0
                    ? "rest response observation is not serializable"
                    : observationError;
                return false;
            }
            using JsonDocument document = JsonDocument.Parse(observationJson);
            observation = document.RootElement.Clone();
        }

        RuntimeV4ExpertRestEffectWitness? witness = response.EffectWitness;
        var root = new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV4ExpertRestActionContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV4ExpertRestActionContract.SchemaDigest,
            ["provenance"] = Provenance(),
            ["profile"] = RuntimeV4ExpertRestActionContract.Profile,
            ["correlation_id"] = response.Context.CorrelationId,
            ["instance_id"] = response.Context.InstanceId,
            ["session_id"] = response.Context.SessionId,
            ["lease_id"] = response.Context.LeaseId,
            ["lease_epoch"] = response.Context.LeaseEpoch,
            ["generation"] = response.Generation,
            ["state_id"] = response.StateId,
            ["operation_id"] = response.Operation.OperationId,
            ["kind"] = "action_response",
            ["action"] = response.Action is null ? null : ActionReference(response.Action),
            ["status"] = response.Status,
            ["observation"] = observation,
            ["transition"] = response.Transition is null ? null : Transition(response.Transition),
            ["effect_witness"] = witness is null ? null : EffectWitness(witness),
            ["error_code"] = response.ErrorCode
        };
        json = JsonSerializer.Serialize(root);
        return true;
    }

    /// <summary>Strictly checks a producer response as a downstream consumer would.</summary>
    internal static bool TryValidateResponse(
        string json,
        RuntimeV4ExpertRestContext context,
        out string error)
    {
        error = string.Empty;
        try
        {
            using JsonDocument document = JsonDocument.Parse(json,
                new JsonDocumentOptions { MaxDepth = 24 });
            JsonElement root = document.RootElement;
            if (!HasExactFields(root,
                    "protocol_version", "schema_digest", "provenance", "profile",
                    "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch",
                    "generation", "state_id", "operation_id", "kind", "action", "status",
                    "observation", "transition", "effect_witness", "error_code"))
            {
                return Fail(out error, "rest response envelope fields are not exact");
            }
            if (StringField(root, "protocol_version")
                != RuntimeV4ExpertRestActionContract.ProtocolVersion
                || StringField(root, "schema_digest")
                != RuntimeV4ExpertRestActionContract.SchemaDigest
                || StringField(root, "profile") != RuntimeV4ExpertRestActionContract.Profile
                || StringField(root, "kind") != "action_response"
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || !UInt64Field(root, "lease_epoch", context.LeaseEpoch)
                || !UInt64Field(root, "generation", out ulong generation)
                || generation > RuntimeV3GameplayContract.MaxGeneration
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "state_id"))
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "operation_id"))
                || !Provenance(root))
            {
                return Fail(out error, "rest response envelope identity is invalid");
            }

            if (!root.TryGetProperty("status", out JsonElement statusValue)
                || statusValue.ValueKind != JsonValueKind.String)
            {
                return Fail(out error, "rest response status is missing");
            }
            string? status = statusValue.GetString();
            if (status is not ("accepted" or "settled" or "rejected" or "unknown" or "cancelled"))
            {
                return Fail(out error, "rest response status is outside the closed set");
            }
            bool actionNull = root.GetProperty("action").ValueKind == JsonValueKind.Null;
            bool observationNull = root.GetProperty("observation").ValueKind == JsonValueKind.Null;
            bool transitionNull = root.GetProperty("transition").ValueKind == JsonValueKind.Null;
            bool witnessNull = root.GetProperty("effect_witness").ValueKind == JsonValueKind.Null;
            bool errorNull = root.GetProperty("error_code").ValueKind == JsonValueKind.Null;

            if (status == "accepted")
            {
                if (actionNull || !observationNull || !transitionNull || !witnessNull || !errorNull)
                    return Fail(out error, "accepted rest response has invalid settlement fields");
                return true;
            }
            if (status is "rejected" or "unknown" or "cancelled")
            {
                if (actionNull || !observationNull || !transitionNull || !witnessNull
                    || errorNull || !RuntimeV4ExpertRestActionContract.IsIdentity(
                        StringField(root, "error_code")))
                    return Fail(out error, "non-settled rest response has invalid fields");
                return ValidateActionReference(root.GetProperty("action"), out error);
            }

            if (actionNull || observationNull || transitionNull || !errorNull)
                return Fail(out error, "settled rest response has invalid fields");
            if (!ValidateActionReference(root.GetProperty("action"), out error)) return false;
            if (!ValidateObservation(root.GetProperty("observation"), generation, out error)) return false;
            if (!ValidateTransition(root.GetProperty("transition"), generation,
                    root.GetProperty("operation_id").GetString()!, out string? transitionOption,
                    out bool transitionWitness, out error)) return false;
            if (transitionWitness != !witnessNull)
                return Fail(out error, "root and transition witness presence differs");
            if (!witnessNull && !ValidateWitness(root.GetProperty("effect_witness"), generation,
                    root.GetProperty("operation_id").GetString()!, transitionOption!, out error))
                return false;
            if (!witnessNull && !JsonElementDeepEquals(
                    root.GetProperty("effect_witness"),
                    root.GetProperty("transition").GetProperty("effect_witness")))
                return Fail(out error, "root and transition witnesses differ");
            return true;
        }
        catch (JsonException)
        {
            return Fail(out error, "rest response JSON is invalid");
        }
        catch (InvalidOperationException)
        {
            return Fail(out error, "rest response shape is invalid");
        }
    }

    internal static bool TryParseRequest(
        string json,
        RuntimeV4ExpertRestContext context,
        out RuntimeV4ExpertRestRequest? request,
        out string error)
    {
        request = null;
        error = string.Empty;
        try
        {
            using JsonDocument document = JsonDocument.Parse(json,
                new JsonDocumentOptions { MaxDepth = 24 });
            JsonElement root = document.RootElement;
            if (!HasExactFields(root,
                    "protocol_version", "schema_digest", "provenance", "profile",
                    "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch",
                    "generation", "state_id", "operation_id", "kind", "action", "status",
                    "observation", "transition", "effect_witness", "error_code")
                || StringField(root, "protocol_version")
                    != RuntimeV4ExpertRestActionContract.ProtocolVersion
                || StringField(root, "schema_digest")
                    != RuntimeV4ExpertRestActionContract.SchemaDigest
                || StringField(root, "profile") != RuntimeV4ExpertRestActionContract.Profile
                || StringField(root, "kind") != "action_request"
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || !UInt64Field(root, "lease_epoch", context.LeaseEpoch)
                || !UInt64Field(root, "generation", out ulong generation)
                || generation > RuntimeV3GameplayContract.MaxGeneration
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "state_id"))
                || !RuntimeV4ExpertRestActionContract.IsIdentity(StringField(root, "operation_id"))
                || !Provenance(root)
                || !NullField(root, "status") || !NullField(root, "observation")
                || !NullField(root, "transition") || !NullField(root, "effect_witness")
                || !NullField(root, "error_code"))
            {
                error = "rest action request envelope is invalid";
                return false;
            }
            if (!TryParseAction(root.GetProperty("action"), out RuntimeV4ExpertRestActionReference? action,
                    out error)) return false;
            string stateId = StringField(root, "state_id")!;
            string operationId = StringField(root, "operation_id")!;
            request = new RuntimeV4ExpertRestRequest(
                context,
                new RuntimeV4ExpertRestOperation(context.InstanceId, context.SessionId,
                    context.LeaseId, context.LeaseEpoch, operationId),
                action!, stateId, generation);
            return true;
        }
        catch (JsonException)
        {
            error = "rest action request JSON is invalid";
            return false;
        }
        catch (InvalidOperationException)
        {
            error = "rest action request shape is invalid";
            return false;
        }
    }

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
        if (response.Transition.AfterGeneration != response.Generation
            || response.Transition.AfterGeneration <= response.Transition.BeforeGeneration)
        {
            error = "rest transition generation is not fenced";
            return false;
        }
        if (response.Transition is RuntimeV4ExpertRestSelectionRequestedTransition requested)
        {
            if (response.EffectWitness is not null || requested.Selector is null)
            {
                error = "selection request cannot carry an effect witness";
                return false;
            }
            return ValidateSelector(requested.Selector, requested.RestOptionId, out error);
        }
        if (response.Transition is RuntimeV4ExpertRestSelectionProgressedTransition progressed)
        {
            if (response.EffectWitness is not null || progressed.Selector is null)
            {
                error = "selection progress cannot carry an effect witness";
                return false;
            }
            return ValidateSelector(progressed.Selector, progressed.RestOptionId, out error);
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
            return ValidateWitness(response.EffectWitness, response.Generation,
                response.Operation.OperationId, completed.RestOptionId, out error)
                && completed.SelectedChoiceIds.Count == completed.RequiredCount;
        }
        if (response.Transition is RuntimeV4ExpertRestCompletedTransition completedOption)
        {
            if (response.EffectWitness is null
                || completedOption.EffectWitness != response.EffectWitness)
            {
                error = "rest completion witness is missing or duplicated incorrectly";
                return false;
            }
            return ValidateWitness(response.EffectWitness, response.Generation,
                response.Operation.OperationId, completedOption.RestOptionId, out error);
        }
        error = "rest transition type is unsupported";
        return false;
    }

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

    private static bool ValidateWitness(
        RuntimeV4ExpertRestEffectWitness witness,
        ulong generation,
        string operationId,
        string optionId,
        out string error)
    {
        error = string.Empty;
        if (witness is null
            || witness.Operation is null
            || witness.Operation.OperationId != operationId
            || witness.RestOptionId != optionId
            || witness.Generation != generation
            || (optionId == "mend") != (witness.TargetPlayerId is not null)
            || witness.TargetPlayerId is not null
                && !RuntimeV4ExpertRestActionContract.IsIdentity(witness.TargetPlayerId))
        {
            error = "rest effect witness identity is invalid";
            return false;
        }
        using JsonDocument document = JsonDocument.Parse(JsonSerializer.Serialize(
            EffectWitness(witness)));
        return ValidateWitness(document.RootElement, generation, operationId, optionId, out error);
    }

    private static bool ValidateWitness(
        JsonElement witness,
        ulong generation,
        string operationId,
        string optionId,
        out string error)
    {
        error = string.Empty;
        if (!HasExactFieldsOrOptional(witness, "target_player_id",
                "version", "kind", "operation_id", "rest_option_id", "generation", "evidence")
            || StringField(witness, "version")
                != RuntimeV4ExpertRestActionContract.EffectWitnessVersion
            || StringField(witness, "operation_id") != operationId
            || StringField(witness, "rest_option_id") != optionId
            || !UInt64Field(witness, "generation", generation)
            || !witness.TryGetProperty("evidence", out JsonElement evidence)
            || evidence.ValueKind != JsonValueKind.Object)
        {
            error = "rest effect witness identity is invalid";
            return false;
        }
        string? kind = StringField(witness, "kind");
        if (kind != optionId + "_applied")
        {
            error = "rest effect witness kind does not match its option";
            return false;
        }
        bool targetRequired = optionId == "mend";
        string? target = null;
        if (witness.TryGetProperty("target_player_id", out JsonElement targetValue))
        {
            target = targetValue.ValueKind == JsonValueKind.Null
                ? null : StringField(witness, "target_player_id");
            if (targetValue.ValueKind is not (JsonValueKind.Null or JsonValueKind.String))
            {
                error = "rest effect witness target field has an invalid type";
                return false;
            }
        }
        if (targetRequired != (target is not null)
            || target is not null && !RuntimeV4ExpertRestActionContract.IsIdentity(target))
        {
            error = "rest effect witness target identity is invalid";
            return false;
        }
        return ValidateEvidence(evidence, kind!, out error);
    }

    private static bool ValidateEvidence(JsonElement evidence, string witnessKind, out string error)
    {
        error = string.Empty;
        string? kind = StringField(evidence, "kind");
        if (kind is null)
        {
            error = "rest effect evidence kind is missing";
            return false;
        }
        if (witnessKind is "heal_applied" or "mend_applied")
            return kind is "hp_change" or "native_completion"
                ? true : Fail(out error, "HP or native evidence is required");
        if (witnessKind is "clone_applied" or "cook_applied" or "smith_applied")
            return kind == "card_change"
                ? HasExactFields(evidence, "kind", "added_card_ids", "removed_card_ids", "upgraded_card_ids")
                    && BoundedIds(evidence, "added_card_ids") && BoundedIds(evidence, "removed_card_ids")
                    && BoundedIds(evidence, "upgraded_card_ids")
                    && (witnessKind != "clone_applied"
                        || evidence.GetProperty("added_card_ids").GetArrayLength() > 0)
                    && (witnessKind != "cook_applied"
                        || evidence.GetProperty("removed_card_ids").GetArrayLength() > 0)
                    && (witnessKind != "smith_applied"
                        || evidence.GetProperty("upgraded_card_ids").GetArrayLength() > 0)
                : Fail(out error, "card evidence is required");
        if (witnessKind is "dig_applied" or "hatch_applied")
            return kind == "relic_change"
                ? HasExactFields(evidence, "kind", "added_relic_ids", "removed_relic_ids")
                    && BoundedIds(evidence, "added_relic_ids") && BoundedIds(evidence, "removed_relic_ids")
                    && evidence.GetProperty("added_relic_ids").GetArrayLength() > 0
                : Fail(out error, "relic evidence is required");
        if (witnessKind == "lift_applied")
            return kind is "stat_change" or "native_completion"
                ? true : Fail(out error, "stat or native evidence is required");
        if (witnessKind == "kindle_applied")
            return kind == "native_completion"
                ? HasExactFields(evidence, "kind", "completion_id", "native_state_id")
                    && RuntimeV4ExpertRestActionContract.IsIdentity(StringField(evidence, "completion_id"))
                    && RuntimeV4ExpertRestActionContract.IsIdentity(StringField(evidence, "native_state_id"))
                : Fail(out error, "native completion evidence is required");
        return Fail(out error, "rest effect witness kind is unsupported");
    }

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
                value.GetProperty("selection_id").GetString());
            return RuntimeV4ExpertRestActionContract.TryValidateAction(
                new RuntimeV4ExpertRestActionReference("action", action), out error);
        }
        return Fail(out error, "rest action payload is not a typed closed arm");
    }

    private static Dictionary<string, object?> Provenance() => new()
    {
        ["artifact"] = RuntimeV4ExpertRestActionContract.Artifact,
        ["source"] = RuntimeV4ExpertRestActionContract.SchemaSource,
        ["generator"] = RuntimeV4ExpertRestActionContract.Generator
    };

    private static Dictionary<string, object?> ActionReference(
        RuntimeV4ExpertRestActionReference reference) => new()
    {
        ["action_id"] = reference.ActionId,
        ["action"] = ActionPayload(reference.Action)
    };

    private static Dictionary<string, object?> ActionPayload(RuntimeV4ExpertRestAction action)
    {
        var result = new Dictionary<string, object?> { ["kind"] = action.Kind };
        switch (action.Kind)
        {
            case "rest_option":
                result["rest_option_id"] = action.RestOptionId;
                break;
            case "select_card":
                result["selection_id"] = action.SelectionId;
                result["rest_option_id"] = action.RestOptionId;
                result["card_id"] = action.CardId;
                break;
            case "select_player":
                result["selection_id"] = action.SelectionId;
                result["rest_option_id"] = action.RestOptionId;
                result["player_id"] = action.PlayerId;
                break;
            case "confirm_selection":
            case "cancel_selection":
                result["selection_id"] = action.SelectionId;
                result["rest_option_id"] = action.RestOptionId;
                break;
            default:
                throw new InvalidOperationException("unsupported rest action arm");
        }
        return result;
    }

    private static Dictionary<string, object?> Selector(RuntimeV4ExpertRestSelector selector) => new()
    {
        ["selection_id"] = selector.SelectionId,
        ["selection_kind"] = selector.SelectionKind,
        ["required_count"] = selector.RequiredCount,
        ["selected_choice_ids"] = selector.SelectedChoiceIds,
        ["remaining_count"] = selector.RemainingCount,
        ["legal_actions"] = selector.LegalActions.Select(ActionReference).ToArray()
    };

    private static Dictionary<string, object?> Transition(RuntimeV4ExpertRestTransition transition)
    {
        var result = new Dictionary<string, object?>
        {
            ["before_generation"] = transition.BeforeGeneration,
            ["after_generation"] = transition.AfterGeneration,
            ["rest_option_id"] = transition.RestOptionId
        };
        switch (transition)
        {
            case RuntimeV4ExpertRestCompletedTransition completed:
                result["kind"] = "rest_option_completed";
                result["completed"] = true;
                result["effect_witness"] = EffectWitness(completed.EffectWitness);
                break;
            case RuntimeV4ExpertRestSelectionRequestedTransition requested:
                result["kind"] = "rest_option_selection_requested";
                result["selector"] = Selector(requested.Selector);
                result["effect_witness"] = null;
                break;
            case RuntimeV4ExpertRestSelectionProgressedTransition progressed:
                result["kind"] = "rest_option_selection_progressed";
                result["selection_id"] = progressed.Selector.SelectionId;
                result["selection_kind"] = progressed.Selector.SelectionKind;
                result["required_count"] = progressed.Selector.RequiredCount;
                result["selected_choice_ids"] = progressed.Selector.SelectedChoiceIds;
                result["remaining_count"] = progressed.Selector.RemainingCount;
                result["selector"] = Selector(progressed.Selector);
                result["effect_witness"] = null;
                break;
            case RuntimeV4ExpertRestSelectionCompletedTransition selected:
                result["kind"] = "rest_option_selection_completed";
                result["selection_id"] = selected.SelectionId;
                result["selection_kind"] = selected.SelectionKind;
                result["required_count"] = selected.RequiredCount;
                result["selected_choice_ids"] = selected.SelectedChoiceIds;
                result["remaining_count"] = 0;
                result["completed"] = true;
                result["effect_witness"] = EffectWitness(selected.EffectWitness);
                break;
            default:
                throw new InvalidOperationException("unsupported rest transition arm");
        }
        return result;
    }

    private static Dictionary<string, object?> EffectWitness(
        RuntimeV4ExpertRestEffectWitness witness)
    {
        var result = new Dictionary<string, object?>
        {
            ["version"] = RuntimeV4ExpertRestActionContract.EffectWitnessVersion,
            ["kind"] = witness.Kind,
            ["operation_id"] = witness.Operation.OperationId,
            ["rest_option_id"] = witness.RestOptionId,
            ["generation"] = witness.Generation,
            ["evidence"] = Evidence(witness.Evidence)
        };
        if (witness.TargetPlayerId is not null)
            result["target_player_id"] = witness.TargetPlayerId;
        return result;
    }

    private static Dictionary<string, object?> Evidence(RuntimeV4ExpertRestEvidence evidence) =>
        evidence switch
        {
            RuntimeV4ExpertRestHpEvidence hp => new()
            {
                ["kind"] = hp.Kind, ["hp_before"] = hp.HpBefore, ["hp_after"] = hp.HpAfter,
                ["max_hp_before"] = hp.MaxHpBefore, ["max_hp_after"] = hp.MaxHpAfter
            },
            RuntimeV4ExpertRestCardEvidence cards => new()
            {
                ["kind"] = cards.Kind, ["added_card_ids"] = cards.AddedCardIds,
                ["removed_card_ids"] = cards.RemovedCardIds,
                ["upgraded_card_ids"] = cards.UpgradedCardIds
            },
            RuntimeV4ExpertRestRelicEvidence relics => new()
            {
                ["kind"] = relics.Kind, ["added_relic_ids"] = relics.AddedRelicIds,
                ["removed_relic_ids"] = relics.RemovedRelicIds
            },
            RuntimeV4ExpertRestStatEvidence stat => new()
            {
                ["kind"] = stat.Kind, ["stat_id"] = stat.StatId,
                ["before"] = stat.Before, ["after"] = stat.After
            },
            RuntimeV4ExpertRestNativeEvidence native => new()
            {
                ["kind"] = native.Kind, ["completion_id"] = native.CompletionId,
                ["native_state_id"] = native.NativeStateId
            },
            _ => throw new InvalidOperationException("unsupported rest evidence arm")
        };

    private static bool BoundedIds(JsonElement value, string field) =>
        value.TryGetProperty(field, out JsonElement ids)
        && ids.ValueKind == JsonValueKind.Array
        && ids.GetArrayLength() <= RuntimeV4ExpertRestActionContract.MaxChoices
        && ids.EnumerateArray().All(item => item.ValueKind == JsonValueKind.String
            && RuntimeV4ExpertRestActionContract.IsIdentity(item.GetString()));

    private static bool Provenance(JsonElement root) =>
        root.TryGetProperty("provenance", out JsonElement value)
        && HasExactFields(value, "artifact", "source", "generator")
        && StringField(value, "artifact") == RuntimeV4ExpertRestActionContract.Artifact
        && StringField(value, "source") == RuntimeV4ExpertRestActionContract.SchemaSource
        && StringField(value, "generator") == RuntimeV4ExpertRestActionContract.Generator;

    private static bool NullField(JsonElement root, string field) =>
        root.TryGetProperty(field, out JsonElement value) && value.ValueKind == JsonValueKind.Null;

    private static bool UInt64Field(JsonElement root, string field, ulong expected) =>
        UInt64Field(root, field, out ulong value) && value == expected;

    private static bool UInt64Field(JsonElement root, string field, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(field, out JsonElement element)
            && element.ValueKind == JsonValueKind.Number
            && element.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration;
    }

    private static string? StringField(JsonElement root, string field) =>
        root.TryGetProperty(field, out JsonElement value)
            && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    private static bool HasExactFields(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!names.Add(property.Name)) return false;
        return names.Count == fields.Length && fields.All(names.Contains);
    }

    private static bool HasExactFieldsOrOptional(
        JsonElement value, string optional, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!names.Add(property.Name)) return false;
        return (names.Count == fields.Length || names.Count == fields.Length + 1)
            && fields.All(names.Contains)
            && (!names.Contains(optional) || names.Count == fields.Length + 1);
    }

    private static bool JsonElementDeepEquals(JsonElement left, JsonElement right) =>
        JsonSerializer.Serialize(left) == JsonSerializer.Serialize(right);

    private static bool Fail(out string error, string value)
    {
        error = value;
        return false;
    }
}
