// SPDX-License-Identifier: MIT

using System;
using Godot;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static (int Status, string Response) ProcessRuntimeV3GameplayWork(RuntimeWork work)
    {
        if (!TryAuthorizeRuntimeV3GameplayContext(work.Context, out string authorizationError))
        {
            return (RuntimeRejected, RuntimeV3GameplayPlainError(authorizationError));
        }

        return work.Kind switch
        {
            RuntimeRequestKindRuntimeV3State => ProcessRuntimeV3GameplayState(work.Context, work.Body),
            RuntimeRequestKindRuntimeV3Action => ProcessRuntimeV3GameplayAction(work.Context, work.Body),
            RuntimeRequestKindRuntimeV3Operation => ProcessRuntimeV3GameplayOperation(work.Context, work.Body),
            _ => (400, RuntimeV3GameplayPlainError("unknown_request_kind"))
        };
    }

    private static (int Status, string Response) ProcessRuntimeV3GameplayState(
        RuntimeContext context,
        string body)
    {
        if (body.Length != 0)
        {
            return (400, RuntimeV3GameplayPlainError("state_body_not_allowed"));
        }

        return (RuntimeAccepted, RuntimeV3GameplayStateResponse(context, ReadRuntimeV3GameplayObservation()));
    }

    private static (int Status, string Response) ProcessRuntimeV3GameplayAction(
        RuntimeContext context,
        string body)
    {
        if (!TryParseRuntimeV3GameplayActionRequest(context, body, out RuntimeV3GameplayRequest? request, out string error))
        {
            return (400, RuntimeV3GameplayPlainError(error));
        }
        RuntimeV3GameplayRequest parsedRequest = request!;
        RuntimeV3GameplayHostObservation observation = ReadRuntimeV3GameplayObservation();

        if (RuntimeV3GameplayOperations.TryGetValue(parsedRequest.OperationId, out RuntimeV3GameplayOperation? existing))
        {
            TryFinalizePendingRuntimeV3Gameplay();
            if (String.Equals(existing.CanonicalBody, parsedRequest.CanonicalBody, StringComparison.Ordinal))
            {
                return (RuntimeV3GameplayStatusFor(existing.Status), RuntimeV3GameplayOperationResponse(context, existing));
            }

            return (RuntimeRejected, RuntimeV3GameplayResultResponse(
                context,
                parsedRequest.OperationId,
                parsedRequest.CardIndex,
                parsedRequest.TargetId,
                observation.Generation,
                "rejected",
                observation,
                "idempotency_conflict",
                false));
        }

        if (parsedRequest.Generation != _runtimeV3GameplayGeneration)
        {
            RuntimeV3GameplayOperation rejected = RetainRuntimeV3GameplayOperation(
                parsedRequest,
                observation,
                "rejected",
                "sts2.game-core/stale_generation");
            return (RuntimeRejected, RuntimeV3GameplayOperationResponse(context, rejected));
        }

        if (_runtimeV3GameplayPending != null)
        {
            RuntimeV3GameplayOperation rejected = RetainRuntimeV3GameplayOperation(
                parsedRequest,
                observation,
                "rejected",
                "sts2.runtime/operation_in_progress");
            return (RuntimeRejected, RuntimeV3GameplayOperationResponse(context, rejected));
        }

        string? preconditionError = RuntimeV3GameplayPreconditionError(observation);
        if (preconditionError != null)
        {
            RuntimeV3GameplayOperation rejected = RetainRuntimeV3GameplayOperation(
                parsedRequest,
                observation,
                "rejected",
                preconditionError);
            return (RuntimeRejected, RuntimeV3GameplayOperationResponse(context, rejected));
        }

        RuntimeV3GameplayOperation operation = new(
            parsedRequest,
            observation,
            "dispatching",
            observation,
            null);
        if (RuntimeV3GameplayOperations.Count >= 64)
        {
            operation.Status = "rejected";
            operation.ErrorCode = "sts2.runtime/operation_capacity";
            return (RuntimeTooManyRequests, RuntimeV3GameplayOperationResponse(context, operation));
        }

        RuntimeV3GameplayOperations.Add(operation.OperationId, operation);
        _runtimeV3GameplayPending = operation;
        if (!TryQueueRuntimeV3GameplayAction(parsedRequest, out string dispatchError))
        {
            operation.Status = "rejected";
            operation.Observation = observation;
            operation.ErrorCode = dispatchError;
            _runtimeV3GameplayPending = null;
            return (RuntimeRejected, RuntimeV3GameplayOperationResponse(context, operation));
        }

        TryFinalizePendingRuntimeV3Gameplay();
        if (operation.Status == "settled")
        {
            return (RuntimeAccepted, RuntimeV3GameplayOperationResponse(context, operation));
        }
        if (_runtimeV3GameplayGeneration != parsedRequest.Generation)
        {
            operation.Status = "unknown";
            operation.Observation = null;
            operation.ErrorCode = "sts2.runtime/host_transition_uncertain";
            _runtimeV3GameplayPending = null;
            return (RuntimeUnavailable, RuntimeV3GameplayOperationResponse(context, operation));
        }

        operation.Status = "accepted";
        operation.Observation = observation;
        return (RuntimeAccepted, RuntimeV3GameplayOperationResponse(context, operation));
    }

    private static (int Status, string Response) ProcessRuntimeV3GameplayOperation(
        RuntimeContext context,
        string operationId)
    {
        if (!IsRuntimeV2Identity(operationId))
        {
            return (400, RuntimeV3GameplayPlainError("invalid_operation_id"));
        }
        TryFinalizePendingRuntimeV3Gameplay();
        if (!RuntimeV3GameplayOperations.TryGetValue(operationId, out RuntimeV3GameplayOperation? operation))
        {
            return (404, RuntimeV3GameplayPlainError("operation_not_found"));
        }
        return (RuntimeV3GameplayStatusFor(operation.Status), RuntimeV3GameplayOperationResponse(context, operation));
    }

    private static RuntimeV3GameplayOperation RetainRuntimeV3GameplayOperation(
        RuntimeV3GameplayRequest request,
        RuntimeV3GameplayHostObservation observation,
        string status,
        string errorCode)
    {
        RuntimeV3GameplayOperation operation = new(request, observation, status, observation, errorCode);
        if (RuntimeV3GameplayOperations.Count < 64)
        {
            RuntimeV3GameplayOperations.Add(operation.OperationId, operation);
        }
        return operation;
    }

    private static bool TryAuthorizeRuntimeV3GameplayContext(RuntimeContext context, out string error)
    {
        error = String.Empty;
        if (!IsRuntimeV2Identity(context.InstanceId)
            || !IsRuntimeV2Identity(context.CallerId)
            || !IsRuntimeV2Identity(context.SessionId)
            || !IsRuntimeV2Identity(context.LeaseId)
            || !IsRuntimeV2Identity(context.CorrelationId)
            || !UInt64FromHeader(context.LeaseEpoch, out ulong leaseEpoch)
            || leaseEpoch > RuntimeV3GameplayMaxSafeInteger)
        {
            error = "invalid_runtime_identity";
            return false;
        }

        RuntimeV3GameplayBinding? binding = _runtimeV3GameplayBinding;
        if (binding == null)
        {
            _runtimeV3GameplayBinding = new RuntimeV3GameplayBinding(
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
            error = "runtime_v3_gameplay_identity_fence";
            return false;
        }
        return true;
    }
}
