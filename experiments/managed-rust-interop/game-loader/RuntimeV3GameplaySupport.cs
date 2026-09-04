// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Managed composition seam for Runtime-v3. It accepts only the versioned semantic envelope and
/// delegates observations, catalog generation, and mutation to the host-thread owner.
/// </summary>
internal sealed class RuntimeV3GameplaySupport
{
    private const int Accepted = 200;
    private const int Rejected = 409;
    private const int Unavailable = 503;
    private const int MaxRequestBytes = 128 * 1024;
    private const int MaxResponseBytes = 128 * 1024;
    private static readonly string[] EnvelopeFields =
    {
        "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id",
        "session_id", "lease_id", "lease_epoch", "generation", "kind", "state_id",
        "operation_id", "observation", "legal_actions", "action", "status", "transition",
        "error_code", "wait_for_millis", "wait_outcome", "recovery"
    };

    private readonly RuntimeV3GameplayHost? _host;

    private RuntimeV3GameplaySupport(RuntimeV3GameplayHost? host)
    {
        _host = host;
    }

    internal static RuntimeV3GameplaySupport Unconfigured() => new(null);

    internal static RuntimeV3GameplaySupport WithHost(
        IRuntimeV3HostSource source,
        IRuntimeV3HostThread thread) =>
        new(new RuntimeV3GameplayHost(source, thread));

    internal string Handle(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        string leaseEpochText,
        string body,
        out int status)
    {
        status = Unavailable;
        if (body.Length > MaxRequestBytes
            || !TryParseRequest(
                instanceId,
                sessionId,
                leaseId,
                correlationId,
                leaseEpochText,
                body,
                out JsonDocument? document,
                out JsonElement root,
                out ulong requestGeneration,
                out string kind,
                out _))
        {
            return ErrorEnvelope(
                correlationId,
                instanceId,
                sessionId,
                leaseId,
                ParseEpoch(leaseEpochText),
                0,
                "state_response",
                "invalid_runtime_v3_envelope",
                out status);
        }

        using (JsonDocument parsedDocument = document!)
        {
            return kind switch
            {
                "state_request" => ReadState(
                    instanceId,
                    sessionId,
                    leaseId,
                    correlationId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    "state_response",
                    out status),
                "reobserve_request" => ReadState(
                    instanceId,
                    sessionId,
                    leaseId,
                    correlationId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    "reobserve_response",
                    out status),
                "legal_actions_request" => ReadLegalActions(
                    root,
                    instanceId,
                    sessionId,
                    leaseId,
                    correlationId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    out status),
                "dispatch_action_request" => Dispatch(
                    root,
                    instanceId,
                    sessionId,
                    leaseId,
                    correlationId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    out status),
                "wait_request" => Wait(
                    root,
                    instanceId,
                    sessionId,
                    leaseId,
                    correlationId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    out status),
                "recover_request" => Recover(
                    root,
                    instanceId,
                    sessionId,
                    leaseId,
                    correlationId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    out status),
                _ => ErrorEnvelope(
                    correlationId,
                    instanceId,
                    sessionId,
                    leaseId,
                    ParseEpoch(leaseEpochText),
                    requestGeneration,
                    "state_response",
                    "unsupported_runtime_v3_operation",
                    out status)
            };
        }
    }

    private string ReadState(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        string responseKind,
        out int status)
    {
        if (!TryCurrent(requestGeneration, out RuntimeV3GameplayObservation? observation, out IReadOnlyList<LegalActionReference>? actions, out string error))
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                responseKind, error, out status);
        }
        status = Accepted;
        if (observation is null || actions is null)
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                responseKind, "host_observation_unavailable", out status);
        }
        return ObservationEnvelope(
            responseKind,
            correlationId,
            instanceId,
            sessionId,
            leaseId,
            leaseEpoch,
            observation,
            actions,
            null,
            null,
            null,
            null);
    }

    private string ReadLegalActions(
        JsonElement root,
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        out int status)
    {
        if (!TryString(root, "state_id", out string? requestedStateId))
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", "invalid_legal_actions_request", out status);
        }
        if (!TryCurrent(
                requestGeneration,
                out RuntimeV3GameplayObservation? observation,
                out IReadOnlyList<LegalActionReference>? actions,
                out string error))
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", error, out status);
        }
        if (observation is null || actions is null)
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", "host_observation_unavailable", out status);
        }
        if (!string.Equals(requestedStateId, observation.StateId, StringComparison.Ordinal)
            || observation.Generation != requestGeneration)
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", "stale_generation", out status);
        }
        status = Accepted;
        return LegalActionsEnvelope(
            "legal_actions_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            observation, actions);
    }

    private string Dispatch(
        JsonElement root,
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        out int status)
    {
        if (!TryString(root, "state_id", out string? stateId)
            || !TryString(root, "operation_id", out string? operationId)
            || !TryAction(root, requestGeneration, out LegalActionReference? requestedAction)
            || _host is null)
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", "host_not_configured_or_invalid_action", out status);
        }
        if (!TryCurrent(requestGeneration, out RuntimeV3GameplayObservation? observation, out IReadOnlyList<LegalActionReference>? actions, out string error))
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", error, out status);
        }
        if (observation is null || actions is null || requestedAction is null || operationId is null)
        {
            return RecoveryState(
                instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                "reobserve_response", "host_observation_unavailable", out status);
        }
        if (!string.Equals(stateId, observation.StateId, StringComparison.Ordinal)
            || requestedAction.Generation != observation.Generation
            || observation.Generation != requestGeneration)
        {
            return RejectedEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                observation, actions, "stale_generation", out status, operationId);
        }
        if (!new LegalActionCatalog(observation.Generation, actions).ContainsExact(requestedAction))
        {
            return RejectedEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                observation, actions, "action_not_current", out status, operationId);
        }
        RuntimeV3DispatchReceipt receipt;
        try
        {
            receipt = _host.Dispatch(operationId, observation, requestedAction);
        }
        catch (Exception)
        {
            return UnknownEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, "dispatch_outcome_unknown", "recovery_required",
                out status, operationId);
        }
        status = receipt.Status switch
        {
            RuntimeV3DispatchStatus.Rejected => Rejected,
            RuntimeV3DispatchStatus.Unknown => Unavailable,
            _ => Accepted
        };
        IReadOnlyList<LegalActionReference> receiptActions = actions;
        if (receipt.Observation is not null)
        {
            try
            {
                receiptActions = _host.LegalActions(receipt.Observation);
            }
            catch (Exception)
            {
                return UnknownEnvelope(
                    "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                    leaseEpoch, requestGeneration, "settlement_unproven", "recovery_required",
                    out status, operationId);
            }
        }
        return ReceiptEnvelope(
            "dispatch_action_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            requestGeneration, receipt, observation, receiptActions);
    }

    private string Wait(
        JsonElement root,
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        out int status)
    {
        if (!TryString(root, "operation_id", out string? operationId) || _host is null)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, "operation_not_found", "timeout", out status, operationId);
        }
        if (!_host.TryGetReceipt(operationId, out RuntimeV3DispatchReceipt? receipt) || receipt is null)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, "operation_not_found", "recovery_required", out status, operationId);
        }
        if (receipt.Status != RuntimeV3DispatchStatus.Settled)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, receipt.ErrorCode ?? "operation_in_progress", "timeout",
                out status, operationId);
        }
        status = Accepted;
        string outcome = "successor";
        IReadOnlyList<LegalActionReference> receiptActions;
        try
        {
            receiptActions = receipt.Observation is null
                ? Array.Empty<LegalActionReference>()
                : _host.LegalActions(receipt.Observation);
        }
        catch (Exception)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, "settlement_unproven", "recovery_required", out status,
                operationId);
        }
        return ReceiptEnvelope(
            "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            requestGeneration, receipt, receipt.Observation, receiptActions, outcome);
    }

    private string Recover(
        JsonElement root,
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong requestGeneration,
        out int status)
    {
        if (!root.TryGetProperty("recovery", out JsonElement recovery)
            || recovery.ValueKind != JsonValueKind.Object
            || !TryString(recovery, "kind", out string? recoveryKind))
        {
            return UnknownEnvelope(
                "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, "invalid_recovery", "recovery_required", out status, null);
        }
        if (recoveryKind == "reobserve")
        {
            if (!TryCurrent(requestGeneration, out RuntimeV3GameplayObservation? observation, out IReadOnlyList<LegalActionReference>? actions, out string error))
            {
                return RecoveryState(
                    instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                    "reobserve_response", error, out status);
            }
            if (observation is null || actions is null)
            {
                return RecoveryState(
                    instanceId, sessionId, leaseId, correlationId, leaseEpoch, requestGeneration,
                    "reobserve_response", "host_observation_unavailable", out status);
            }
            status = Accepted;
            return ObservationEnvelope(
                "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                observation, actions, "recovery-operation", "accepted", null, null);
        }
        return UnknownEnvelope(
            "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            requestGeneration, "recovery_requires_external_owner", "recovery_required", out status, null);
    }

    private bool TryCurrent(
        ulong requestGeneration,
        out RuntimeV3GameplayObservation? observation,
        out IReadOnlyList<LegalActionReference>? actions,
        out string error)
    {
        observation = null;
        actions = Array.Empty<LegalActionReference>();
        error = "host_not_configured";
        if (_host is null)
        {
            return false;
        }
        try
        {
            observation = _host.Observe();
            if (observation.Generation != requestGeneration)
            {
                observation = null;
                actions = Array.Empty<LegalActionReference>();
                error = "stale_generation";
                return false;
            }
            actions = _host.LegalActions(observation);
            if (!observation.Validate(out error))
            {
                observation = null;
                actions = Array.Empty<LegalActionReference>();
                return false;
            }
            return true;
        }
        catch (Exception)
        {
            observation = null;
            actions = Array.Empty<LegalActionReference>();
            error = "host_observation_unavailable";
            return false;
        }
    }

    private static bool TryParseRequest(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        string leaseEpochText,
        string body,
        out JsonDocument? document,
        out JsonElement root,
        out ulong generation,
        out string kind,
        out string? error)
    {
        document = null;
        root = default;
        generation = 0;
        kind = string.Empty;
        error = null;
        try
        {
            document = JsonDocument.Parse(body, new JsonDocumentOptions { MaxDepth = 16 });
        }
        catch (JsonException)
        {
            error = "invalid_json";
            return false;
        }
        root = document!.RootElement;
        if (root.ValueKind != JsonValueKind.Object
            || !HasExactFields(root, EnvelopeFields)
            || !RuntimeV3GameplayContract.IsIdentity(instanceId)
            || !RuntimeV3GameplayContract.IsIdentity(sessionId)
            || !RuntimeV3GameplayContract.IsIdentity(leaseId)
            || !RuntimeV3GameplayContract.IsIdentity(correlationId)
            || !root.TryGetProperty("provenance", out JsonElement provenance)
            || !HasExactFields(provenance, "artifact", "source", "generator")
            || StringField(root, "protocol_version") != RuntimeV3GameplayContract.ProtocolVersion
            || StringField(root, "schema_digest") != RuntimeV3GameplayContract.SchemaDigest
            || StringField(root, "correlation_id") != correlationId
            || StringField(root, "instance_id") != instanceId
            || StringField(root, "session_id") != sessionId
            || StringField(root, "lease_id") != leaseId
            || StringField(root, "provenance.artifact") != RuntimeV3GameplayContract.Artifact
            || StringField(root, "provenance.source") != RuntimeV3GameplayContract.SchemaSource
            || StringField(root, "provenance.generator") != RuntimeV3GameplayContract.Generator
            || !TryEpoch(root, "lease_epoch", leaseEpochText, out _)
            || !root.TryGetProperty("generation", out JsonElement generationElement)
            || !generationElement.TryGetUInt64(out generation)
            || generation > RuntimeV3GameplayContract.MaxGeneration
            || !TryString(root, "kind", out string? parsedKind))
        {
            error = "invalid_runtime_v3_envelope";
            document!.Dispose();
            document = null;
            return false;
        }
        kind = parsedKind!;
        if (!ValidateRequestShape(root, kind, generation))
        {
            error = "invalid_runtime_v3_request_shape";
            document!.Dispose();
            document = null;
            return false;
        }
        return true;
    }

    private static bool ValidateRequestShape(JsonElement root, string kind, ulong generation) =>
        kind switch
        {
            "state_request" or "reobserve_request" => NullFields(
                root, "state_id", "operation_id", "observation", "legal_actions", "action",
                "status", "transition", "error_code", "wait_for_millis", "wait_outcome", "recovery"),
            "legal_actions_request" => TryString(root, "state_id", out _)
                && NullFields(
                    root, "operation_id", "observation", "legal_actions", "action", "status",
                    "transition", "error_code", "wait_for_millis", "wait_outcome", "recovery"),
            "dispatch_action_request" => TryString(root, "state_id", out _)
                && TryString(root, "operation_id", out _)
                && TryAction(root, generation, out _)
                && NullFields(
                    root, "observation", "legal_actions", "status", "transition", "error_code",
                    "wait_for_millis", "wait_outcome", "recovery"),
            "wait_request" => TryString(root, "operation_id", out _)
                && TryWait(root)
                && NullFields(
                    root, "state_id", "observation", "legal_actions", "action", "status",
                    "transition", "error_code", "wait_outcome", "recovery"),
            "recover_request" => NullFields(
                    root, "state_id", "operation_id", "observation", "legal_actions", "action",
                    "status", "transition", "error_code", "wait_for_millis", "wait_outcome")
                && TryRecovery(root),
            _ => false
        };

    private static bool NullFields(JsonElement root, params string[] fields)
    {
        foreach (string field in fields)
        {
            if (!root.TryGetProperty(field, out JsonElement value)
                || value.ValueKind != JsonValueKind.Null)
            {
                return false;
            }
        }
        return true;
    }

    private static bool TryWait(JsonElement root)
    {
        return root.TryGetProperty("wait_for_millis", out JsonElement value)
            && value.TryGetInt32(out int waitForMillis)
            && waitForMillis is >= 1 and <= 120_000;
    }

    private static bool TryRecovery(JsonElement root)
    {
        if (!root.TryGetProperty("recovery", out JsonElement recovery)
            || !HasExactFields(recovery, "kind", "operation_id")
            || !TryString(recovery, "kind", out string? recoveryKind)
            || !MatchesRecoveryKind(recoveryKind)
            || !TryOptionalString(recovery, "operation_id", out string? operationId))
        {
            return false;
        }
        return (recoveryKind == "reconcile") == (operationId is not null);
    }

    private static bool MatchesRecoveryKind(string? kind) =>
        kind is "reobserve" or "reconcile" or "release_lease" or "stop_episode";

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
        string? selectedValue = null;
        string? targetId = null;
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
        if (actionId is null || kind is null)
        {
            return false;
        }
        action = new LegalActionReference(actionId, kind, selectedValue, targetId, generation);
        return action.Validate(out _);
    }

    private static string ObservationEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> actions,
        string? operationId,
        string? status,
        RuntimeV3TransitionWitness? witness,
        string? errorCode)
    {
        if (!FairPlayProjection.TrySerialize(observation, actions, out string projection, out _))
        {
            return UnknownEnvelope(
                kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                observation.Generation, "projection_unavailable", "recovery_required",
                out _, operationId);
        }
        using JsonDocument document = JsonDocument.Parse(projection);
        JsonElement root = document.RootElement;
        var observationObject = new Dictionary<string, object?>
        {
            ["state_id"] = root.GetProperty("state_id").Clone(),
            ["generation"] = root.GetProperty("generation").Clone(),
            ["visible_seed"] = root.GetProperty("visible_seed").Clone(),
            ["player"] = root.GetProperty("player").Clone(),
            ["state"] = root.GetProperty("state").Clone()
        };
        return SerializeEnvelope(
            kind,
            correlationId,
            instanceId,
            sessionId,
            leaseId,
            leaseEpoch,
            observation.Generation,
            observation.StateId,
            operationId,
            observationObject,
            root.GetProperty("legal_actions").Clone(),
            null,
            status,
            witness,
            errorCode,
            null,
            null);
    }

    private static string LegalActionsEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> actions)
    {
        if (!FairPlayProjection.TrySerialize(observation, actions, out string projection, out _))
        {
            return UnknownEnvelope(
                kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                observation.Generation, "projection_unavailable", "recovery_required",
                out _, null);
        }
        using JsonDocument document = JsonDocument.Parse(projection);
        JsonElement root = document.RootElement;
        return SerializeEnvelope(
            kind,
            correlationId,
            instanceId,
            sessionId,
            leaseId,
            leaseEpoch,
            observation.Generation,
            observation.StateId,
            null,
            null,
            root.GetProperty("legal_actions").Clone(),
            null,
            null,
            null,
            null,
            null,
            null);
    }

    private static string ReceiptEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong requestGeneration,
        RuntimeV3DispatchReceipt receipt,
        RuntimeV3GameplayObservation? fallbackObservation,
        IReadOnlyList<LegalActionReference> fallbackActions,
        string? waitOutcome = null)
    {
        if (receipt.Status == RuntimeV3DispatchStatus.Unknown)
        {
            return UnknownEnvelope(
                kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch, requestGeneration,
                receipt.ErrorCode ?? "settlement_unproven",
                waitOutcome ?? "recovery_required", out _, receipt.OperationId);
        }
        RuntimeV3GameplayObservation? observation = receipt.Observation ?? fallbackObservation;
        IReadOnlyList<LegalActionReference> actions = fallbackActions;
        string? status = receipt.Status switch
        {
            RuntimeV3DispatchStatus.Accepted => "accepted",
            RuntimeV3DispatchStatus.Settled => "settled",
            RuntimeV3DispatchStatus.Rejected => "rejected",
            _ => "unknown"
        };
        if (observation is null || !FairPlayProjection.TrySerialize(observation, actions, out string projection, out _))
        {
            return UnknownEnvelope(
                kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch, requestGeneration,
                "settlement_unproven", waitOutcome ?? "recovery_required", out _, receipt.OperationId);
        }
        using JsonDocument document = JsonDocument.Parse(projection);
        JsonElement root = document.RootElement;
        var observationObject = new Dictionary<string, object?>
        {
            ["state_id"] = root.GetProperty("state_id").Clone(),
            ["generation"] = root.GetProperty("generation").Clone(),
            ["visible_seed"] = root.GetProperty("visible_seed").Clone(),
            ["player"] = root.GetProperty("player").Clone(),
            ["state"] = root.GetProperty("state").Clone()
        };
        return SerializeEnvelope(
            kind,
            correlationId,
            instanceId,
            sessionId,
            leaseId,
            leaseEpoch,
            observation.Generation,
            observation.StateId,
            receipt.OperationId,
            observationObject,
            root.GetProperty("legal_actions").Clone(),
            null,
            status,
            receipt.Witness,
            receipt.ErrorCode,
            null,
            waitOutcome);
    }

    private static string RecoveryState(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        ulong leaseEpoch,
        ulong generation,
        string kind,
        string errorCode,
        out int status)
    {
        RuntimeV3GameplayObservation observation = new(
            "recovery-1",
            generation,
            null,
            new RuntimeV3GameplayPlayer(
                0, 0, 0, 0,
                Array.Empty<RuntimeV3GameplayCard>(),
                Array.Empty<RuntimeV3GameplayCard>(),
                Array.Empty<RuntimeV3GameplayCard>(),
                Array.Empty<RuntimeV3GameplayCard>()),
            RuntimeV3GameplayState.Recovery,
            new[] { errorCode },
            Array.Empty<RuntimeV3GameplayEnemy>());
        status = Unavailable;
        return ObservationEnvelope(
            kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch, observation,
            Array.Empty<LegalActionReference>(), null, null, null, null);
    }

    private static string RejectedEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        RuntimeV3GameplayObservation observation,
        IReadOnlyList<LegalActionReference> actions,
        string errorCode,
        out int status,
        string? operationId = null)
    {
        status = Rejected;
        return ObservationEnvelope(
            kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch, observation, actions,
            operationId ?? "rejected-operation", "rejected", null, errorCode);
    }

    private static string UnknownEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong generation,
        string errorCode,
        string waitOutcome,
        out int status,
        string? operationId)
    {
        status = Unavailable;
        if (operationId is null
            && kind is "dispatch_action_response" or "wait_response" or "recover_response")
        {
            operationId = correlationId;
        }
        string? safeWaitOutcome = kind == "wait_response" ? waitOutcome : null;
        return SerializeEnvelope(
            kind, correlationId, instanceId, sessionId, leaseId, leaseEpoch, generation, null,
            operationId, null, null, null, "unknown", null, errorCode, null, safeWaitOutcome);
    }

    private static string ErrorEnvelope(
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong generation,
        string kind,
        string errorCode,
        out int status)
    {
        status = 400;
        return RecoveryState(
            instanceId, sessionId, leaseId, correlationId, leaseEpoch, generation,
            kind, errorCode, out _);
    }

    private static string SerializeEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong generation,
        string? stateId,
        string? operationId,
        object? observation,
        object? legalActions,
        object? action,
        string? status,
        RuntimeV3TransitionWitness? witness,
        string? errorCode,
        int? waitForMillis,
        string? waitOutcome)
    {
        var envelope = new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplayContract.SchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = RuntimeV3GameplayContract.Artifact,
                ["source"] = RuntimeV3GameplayContract.SchemaSource,
                ["generator"] = RuntimeV3GameplayContract.Generator
            },
            ["correlation_id"] = correlationId,
            ["instance_id"] = instanceId,
            ["session_id"] = sessionId,
            ["lease_id"] = leaseId,
            ["lease_epoch"] = leaseEpoch,
            ["generation"] = generation,
            ["kind"] = kind,
            ["state_id"] = stateId,
            ["operation_id"] = operationId,
            ["observation"] = observation,
            ["legal_actions"] = legalActions,
            ["action"] = action,
            ["status"] = status,
            ["transition"] = witness is null
                ? null
                : new Dictionary<string, object?>
                {
                    ["from_generation"] = witness.FromGeneration,
                    ["to_generation"] = witness.ToGeneration,
                    ["state_id"] = witness.StateId,
                    ["effect_kind"] = witness.EffectKind
                },
            ["error_code"] = errorCode,
            ["wait_for_millis"] = waitForMillis,
            ["wait_outcome"] = waitOutcome,
            ["recovery"] = null
        };
        string json = JsonSerializer.Serialize(envelope);
        using JsonDocument document = JsonDocument.Parse(json);
        if (json.Length > MaxResponseBytes
            || !PrivilegedFieldGuard.IsSafeJson(document.RootElement, out _))
        {
            return "{\"error_code\":\"runtime_v3_response_unavailable\"}";
        }
        return json;
    }

    private static bool HasExactFields(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object)
        {
            return false;
        }
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            names.Add(property.Name);
        }
        if (names.Count != fields.Length)
        {
            return false;
        }
        foreach (string field in fields)
        {
            if (!names.Contains(field))
            {
                return false;
            }
        }
        return true;
    }

    private static string? StringField(JsonElement root, string path)
    {
        JsonElement value = root;
        foreach (string segment in path.Split('.'))
        {
            if (value.ValueKind != JsonValueKind.Object || !value.TryGetProperty(segment, out value))
            {
                return null;
            }
        }
        return value.ValueKind == JsonValueKind.String ? value.GetString() : null;
    }

    private static bool TryString(JsonElement root, string property, out string? value)
    {
        value = StringField(root, property);
        return value is not null && RuntimeV3GameplayContract.IsIdentity(value);
    }

    private static bool TryOptionalString(JsonElement root, string property, out string? value)
    {
        value = null;
        if (!root.TryGetProperty(property, out JsonElement element))
        {
            return false;
        }
        if (element.ValueKind == JsonValueKind.Null)
        {
            return true;
        }
        return element.ValueKind == JsonValueKind.String
            && RuntimeV3GameplayContract.IsIdentity(value = element.GetString() ?? string.Empty);
    }

    private static bool TryEpoch(JsonElement root, string property, string expected, out ulong value)
    {
        value = 0;
        return ulong.TryParse(expected, out ulong expectedEpoch)
            && expectedEpoch <= RuntimeV3GameplayContract.MaxGeneration
            && root.TryGetProperty(property, out JsonElement element)
            && element.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration
            && value == expectedEpoch;
    }

    private static ulong ParseEpoch(string value) =>
        ulong.TryParse(value, out ulong epoch)
            && epoch <= RuntimeV3GameplayContract.MaxGeneration
            ? epoch
            : 0;
}

public static partial class ModEntry
{
    private const uint RuntimeRequestKindGameplay = 3;
    private static RuntimeV3GameplaySupport? _runtimeV3Gameplay;

    private static void InitializeRuntimeV3Gameplay()
    {
        _runtimeV3Gameplay = RuntimeV3GameplaySupport.Unconfigured();
    }

    private static (int Status, string Response) ProcessRuntimeV3GameplayWork(
        RuntimeContext context,
        string body)
    {
        RuntimeV3GameplaySupport support = _runtimeV3Gameplay ?? RuntimeV3GameplaySupport.Unconfigured();
        string response = support.Handle(
            context.InstanceId,
            context.SessionId,
            context.LeaseId,
            context.CorrelationId,
            context.LeaseEpoch,
            body,
            out int status);
        return (status, response);
    }
}
