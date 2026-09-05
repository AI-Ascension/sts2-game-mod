// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplaySupport
{
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
        TryString(root, "operation_id", out string? operationId);
        if (!TryString(root, "state_id", out string? stateId)
            || !TryAction(root, requestGeneration, out LegalActionReference? requestedAction)
            || _host is null)
        {
            return UnknownEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, "host_not_configured_or_invalid_action",
                "recovery_required", out status, operationId);
        }
        if (operationId is null || stateId is null || requestedAction is null)
        {
            return UnknownEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, "invalid_action", "recovery_required", out status, operationId);
        }
        RuntimeV3OperationKey operation = new(instanceId, sessionId, leaseId, leaseEpoch, operationId);
        if (_host.TryReplay(operation, stateId, requestedAction, out RuntimeV3DispatchReceipt? replay)
            && replay is not null)
        {
            return RenderDispatchReceipt(
                correlationId, instanceId, sessionId, leaseId, leaseEpoch, requestGeneration, replay, out status);
        }
        return DispatchCurrent(operation, stateId, requestedAction, correlationId, requestGeneration, out status);
    }

    private string DispatchCurrent(RuntimeV3OperationKey operation, string stateId,
        LegalActionReference requestedAction, string correlationId, ulong requestGeneration, out int status)
    {
        string instanceId = operation.InstanceId;
        string sessionId = operation.SessionId;
        string leaseId = operation.LeaseId;
        ulong leaseEpoch = operation.LeaseEpoch;
        string operationId = operation.OperationId;
        if (!TryCurrent(null, out RuntimeV3GameplayObservation? observation, out IReadOnlyList<LegalActionReference>? actions, out string error))
        {
            return UnknownEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, error, "recovery_required", out status, operationId);
        }
        if (observation is null || actions is null || requestedAction is null || operationId is null)
        {
            return UnknownEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, "host_observation_unavailable", "recovery_required", out status, operationId);
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
            receipt = _host!.Dispatch(operation, observation, requestedAction);
        }
        catch (Exception)
        {
            return UnknownEnvelope(
                "dispatch_action_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, "dispatch_outcome_unknown", "recovery_required",
                out status, operationId);
        }
        return RenderDispatchReceipt(
            correlationId, instanceId, sessionId, leaseId, leaseEpoch, requestGeneration, receipt, out status);
    }

    private static string RenderDispatchReceipt(
        string correlationId, string instanceId, string sessionId, string leaseId,
        ulong leaseEpoch, ulong requestGeneration, RuntimeV3DispatchReceipt receipt, out int status)
    {
        status = receipt.Status switch
        {
            RuntimeV3DispatchStatus.Rejected => Rejected,
            RuntimeV3DispatchStatus.Unknown => Unavailable,
            _ => Accepted
        };
        return ReceiptEnvelope(
            "dispatch_action_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            requestGeneration, receipt, receipt.Observation, receipt.LegalActions);
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
        if (!TryString(root, "operation_id", out string? operationId) || operationId is null || _host is null)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, "operation_not_found", "timeout", out status, operationId);
        }
        if (!_host.TryGetReceipt(new RuntimeV3OperationKey(instanceId, sessionId, leaseId, leaseEpoch, operationId), out RuntimeV3DispatchReceipt? receipt) || receipt is null)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, "operation_not_found", "recovery_required", out status, operationId);
        }
        if (receipt.Status != RuntimeV3DispatchStatus.Settled)
        {
            return UnknownEnvelope(
                "wait_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, receipt.ErrorCode ?? "operation_in_progress",
                receipt.Status == RuntimeV3DispatchStatus.Rejected ? "recovery_required" : "timeout",
                out status, operationId);
        }
        status = Accepted;
        string outcome = receipt.Observation?.StateId == receipt.Before.StateId
            ? "same_state_mutation" : "successor";
        IReadOnlyList<LegalActionReference> receiptActions = receipt.LegalActions;
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
            return RecoverObservation(instanceId, sessionId, leaseId, correlationId,
                leaseEpoch, requestGeneration, out status);
        }
        if (recoveryKind == "reconcile"
            && TryString(recovery, "operation_id", out string? operationId) && operationId is not null)
        {
            RuntimeV3OperationKey operation = new(instanceId, sessionId, leaseId, leaseEpoch, operationId);
            if (_host is null || !_host.TryGetReceipt(operation, out RuntimeV3DispatchReceipt? receipt)
                || receipt is null)
            {
                return UnknownEnvelope(
                    "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                    requestGeneration, "operation_not_found", "recovery_required", out status, operationId);
            }
            status = receipt.Status switch
            {
                RuntimeV3DispatchStatus.Rejected => Rejected,
                RuntimeV3DispatchStatus.Unknown => Unavailable,
                _ => Accepted
            };
            return ReceiptEnvelope(
                "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
                requestGeneration, receipt, receipt.Observation, receipt.LegalActions);
        }
        return UnknownEnvelope(
            "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            requestGeneration, "recovery_requires_external_owner", "recovery_required", out status, null);
    }

    private string RecoverObservation(string instanceId, string sessionId, string leaseId,
        string correlationId, ulong leaseEpoch, ulong requestGeneration, out int status)
    {
        if (!TryCurrent(null, out RuntimeV3GameplayObservation? observation,
                out IReadOnlyList<LegalActionReference>? actions, out string error))
        {
            return UnknownEnvelope(
                "recover_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, error, "recovery_required", out status, null);
        }
        if (observation is null || actions is null)
        {
            return UnknownEnvelope(
                "recover_response", correlationId, instanceId, sessionId, leaseId,
                leaseEpoch, requestGeneration, "host_observation_unavailable", "recovery_required", out status, null);
        }
        status = Accepted;
        return ObservationEnvelope(
            "recover_response", correlationId, instanceId, sessionId, leaseId, leaseEpoch,
            observation, actions, "recovery-operation", "accepted", null, null);
    }

}
