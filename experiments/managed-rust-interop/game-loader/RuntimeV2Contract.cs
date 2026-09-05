// SPDX-License-Identifier: MIT

using System;
using Godot;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static (int Status, string Response) ProcessRuntimeV2Work(RuntimeWork work)
    {
        if (!TryAuthorizeRuntimeV2Context(work.Context, out string authorizationError))
        {
            return (RuntimeRejected, RuntimeV2PlainError(authorizationError));
        }

        return work.Kind switch
        {
            RuntimeRequestKindRuntimeV2State => ProcessRuntimeV2State(work.Context, work.Body),
            RuntimeRequestKindRuntimeV2Action => ProcessRuntimeV2Action(work.Context, work.Body),
            RuntimeRequestKindRuntimeV2Operation => ProcessRuntimeV2Operation(work.Context, work.Body),
            _ => (400, RuntimeV2PlainError("unknown_request_kind"))
        };
    }

    private static (int Status, string Response) ProcessRuntimeV2State(
        RuntimeContext context,
        string body)
    {
        if (body.Length != 0)
        {
            return (400, RuntimeV2PlainError("state_body_not_allowed"));
        }

        RuntimeV2HostObservation observation = ReadRuntimeV2Observation();
        return (RuntimeAccepted, RuntimeV2StateResponse(context, observation));
    }

    private static (int Status, string Response) ProcessRuntimeV2Action(
        RuntimeContext context,
        string body)
    {
        if (!TryParseRuntimeV2ActionRequest(context, body, out RuntimeV2Request? request, out string error))
        {
            return (400, RuntimeV2PlainError(error));
        }
        RuntimeV2Request parsedRequest = request!;

        RuntimeV2HostObservation observation = ReadRuntimeV2Observation();
        if (RuntimeV2Operations.TryGetValue(parsedRequest.OperationId, out RuntimeV2Operation? existing))
        {
            TryFinalizePendingRuntimeV2();
            if (String.Equals(existing.CanonicalBody, parsedRequest.CanonicalBody, StringComparison.Ordinal))
            {
                return (RuntimeStatusFor(existing.Status), RuntimeV2OperationResponse(context, existing));
            }

            return (
                RuntimeRejected,
                RuntimeV2ResultResponse(
                    context,
                    parsedRequest.OperationId,
                    RuntimeV2GenerationFor(observation),
                    "rejected",
                    observation,
                    "idempotency_conflict",
                    false));
        }

        if (parsedRequest.Generation != _runtimeGeneration)
        {
            RuntimeV2Operation rejected = RetainRuntimeV2Operation(
                parsedRequest,
                "rejected",
                observation,
                "sts2.game-core/stale_generation");
            return (RuntimeRejected, RuntimeV2OperationResponse(context, rejected));
        }

        if (_runtimeV2Pending != null)
        {
            RuntimeV2Operation rejected = RetainRuntimeV2Operation(
                parsedRequest,
                "rejected",
                observation,
                "sts2.runtime/operation_in_progress");
            return (RuntimeRejected, RuntimeV2OperationResponse(context, rejected));
        }

        string? preconditionError = RuntimeV2ActionPreconditionError(observation);
        if (preconditionError != null)
        {
            RuntimeV2Operation rejected = RetainRuntimeV2Operation(
                parsedRequest,
                "rejected",
                observation,
                preconditionError);
            return (RuntimeRejected, RuntimeV2OperationResponse(context, rejected));
        }

        if (!TryGetRuntimeV2Player(out Player? player))
        {
            RuntimeV2Operation rejected = RetainRuntimeV2Operation(
                parsedRequest,
                "rejected",
                observation,
                "sts2.game-mod/local_player_unavailable");
            return (RuntimeRejected, RuntimeV2OperationResponse(context, rejected));
        }

        if (RuntimeV2Operations.Count >= RuntimeV2OperationCapacity)
        {
            RuntimeV2Operation rejected = new(
                parsedRequest.OperationId,
                parsedRequest.CanonicalBody,
                parsedRequest.Generation,
                observation.TurnIndex,
                "rejected",
                observation,
                "sts2.runtime/operation_capacity");
            return (RuntimeTooManyRequests, RuntimeV2OperationResponse(context, rejected));
        }

        RuntimeV2Operation operation = new(
            parsedRequest.OperationId,
            parsedRequest.CanonicalBody,
            parsedRequest.Generation,
            observation.TurnIndex,
            "dispatching",
            observation,
            null);
        RuntimeV2Operations.Add(operation.OperationId, operation);
        _runtimeV2Pending = operation;

        try
        {
            if (RunManager.Instance?.ActionQueueSynchronizer == null || player?.PlayerCombatState == null)
            {
                operation.Status = "rejected";
                operation.Observation = observation;
                operation.ErrorCode = "sts2.runtime/action_queue_unavailable";
                _runtimeV2Pending = null;
                return (RuntimeRejected, RuntimeV2OperationResponse(context, operation));
            }

            RunManager.Instance.ActionQueueSynchronizer.RequestEnqueue(
                new EndPlayerTurnAction(player, player.PlayerCombatState.TurnNumber));
            GD.Print(
                $"{LogPrefix} Runtime-v2 end_turn queued: player_turn={player.PlayerCombatState.TurnNumber} "
                + $"generation={operation.RequestGeneration}");
        }
        catch (Exception exception)
        {
            operation.Status = "unknown";
            operation.Observation = null;
            operation.ErrorCode = "sts2.runtime/host_action_exception";
            GD.PrintErr($"{LogPrefix} Runtime-v2 end_turn dispatch failed: {exception.GetType().Name}");
            return (RuntimeUnavailable, RuntimeV2OperationResponse(context, operation));
        }

        TryFinalizePendingRuntimeV2();
        return (RuntimeUnavailable, RuntimeV2OperationResponse(context, operation));
    }

    private static (int Status, string Response) ProcessRuntimeV2Operation(
        RuntimeContext context,
        string operationId)
    {
        if (!IsRuntimeV2Identity(operationId))
        {
            return (400, RuntimeV2PlainError("invalid_operation_id"));
        }

        TryFinalizePendingRuntimeV2();
        if (!RuntimeV2Operations.TryGetValue(operationId, out RuntimeV2Operation? operation))
        {
            return (404, RuntimeV2PlainError("operation_not_found"));
        }

        return (RuntimeStatusFor(operation.Status), RuntimeV2OperationResponse(context, operation));
    }

    private static RuntimeV2Operation RetainRuntimeV2Operation(
        RuntimeV2Request request,
        string status,
        RuntimeV2HostObservation observation,
        string errorCode)
    {
        RuntimeV2Operation operation = new(
            request.OperationId,
            request.CanonicalBody,
            request.Generation,
            observation.TurnIndex,
            status,
            observation,
            errorCode);
        if (RuntimeV2Operations.Count < RuntimeV2OperationCapacity)
        {
            RuntimeV2Operations.Add(operation.OperationId, operation);
        }
        return operation;
    }

    private static bool TryAuthorizeRuntimeV2Context(RuntimeContext context, out string error)
    {
        error = string.Empty;
        if (!IsRuntimeV2Identity(context.InstanceId)
            || !IsRuntimeV2Identity(context.CallerId)
            || !IsRuntimeV2Identity(context.SessionId)
            || !IsRuntimeV2Identity(context.LeaseId)
            || !IsRuntimeV2Identity(context.CorrelationId)
            || !UInt64FromHeader(context.LeaseEpoch, out ulong leaseEpoch)
            || leaseEpoch > RuntimeV2MaxSafeInteger)
        {
            error = "invalid_runtime_identity";
            return false;
        }

        RuntimeV2Binding? binding = _runtimeV2Binding;
        if (binding == null)
        {
            _runtimeV2Binding = new RuntimeV2Binding(
                context.InstanceId,
                context.CallerId,
                context.SessionId,
                context.LeaseId,
                leaseEpoch);
            return true;
        }

        if (binding.InstanceId != context.InstanceId
            || binding.CallerId != context.CallerId
            || binding.SessionId != context.SessionId
            || binding.LeaseId != context.LeaseId
            || binding.LeaseEpoch != leaseEpoch)
        {
            error = "runtime_v2_identity_fence";
            return false;
        }

        return true;
    }

}
