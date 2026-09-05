// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplaySupport
{
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

}
