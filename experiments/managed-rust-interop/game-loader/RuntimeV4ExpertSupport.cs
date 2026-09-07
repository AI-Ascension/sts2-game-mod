// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record RuntimeV4ExpertContext(
    string InstanceId,
    string SessionId,
    string LeaseId,
    ulong LeaseEpoch,
    string CorrelationId);

/// <summary>Host-thread owner for the additive expert potion action transport.</summary>
internal sealed partial class RuntimeV4ExpertSupport
{
    private const int Accepted = 200;
    private const int Rejected = 409;
    private const int Unavailable = 503;
    private const int MaxRequestBytes = 128 * 1024;
    private const int MaxReceipts = 4096;
    private const string ProtocolVersion = "runtime-v4-expert-action";
    private const string SchemaDigest =
        "393318bda8c3522c0ecbacc78b95471a9f4dc3f825169d2048f4c74a7b7f2929";
    private const string Artifact = "sts2-protocol/runtime-v4-expert-action";
    private const string SchemaSource = "schemas/runtime-v4-expert-action.schema.json";
    private const string Generator = "hand-authored";
    private const string Profile = "expert-action";
    private readonly LiveCombatSource? _source;
    private readonly IRuntimeV3HostThread? _thread;
    private readonly Dictionary<RuntimeV3OperationKey, RuntimeV4ExpertReceipt> _receipts = new();

    private RuntimeV4ExpertSupport(LiveCombatSource? source, IRuntimeV3HostThread? thread)
    {
        _source = source;
        _thread = thread;
    }

    internal static RuntimeV4ExpertSupport Unconfigured() => new(null, null);

    internal static RuntimeV4ExpertSupport WithHost(
        LiveCombatSource source, IRuntimeV3HostThread thread) => new(source, thread);

    internal (int Status, string Response) HandleState(
        RuntimeV4ExpertContext context, out int status)
    {
        status = Unavailable;
        if (_source is null || _thread is null)
            return (status, Error("runtime_v4_expert_host_unavailable"));
        try
        {
            RuntimeV4ExpertGameplayObservation? observation = null;
            bool ran = InvokeHost(() => observation = _source.ObserveExpert());
            if (!ran || observation is null)
            {
                return (status, Error("runtime_v4_expert_observation_unavailable"));
            }
            if (!RuntimeV4ExpertGameplayCodec.TrySerialize(observation, out string json,
                    out string error))
            {
                return (status, Error(error is { Length: > 0 }
                    ? error : "runtime_v4_expert_observation_unavailable"));
            }
            status = Accepted;
            return (status, json);
        }
        catch (Exception)
        {
            return (status, Error("runtime_v4_expert_observation_unavailable"));
        }
    }

    internal (int Status, string Response) Handle(
        RuntimeV4ExpertContext context, string body, out int status)
    {
        status = Unavailable;
        if (body.Length > MaxRequestBytes)
            return (400, Error("runtime_v4_expert_action_oversized"));
        // The native route supplies the JSON request for POST and an opaque operation suffix
        // for reconciliation GET. The gateway has already admitted the corresponding method.
        return body.TrimStart().StartsWith('{')
            ? HandleRequest(context, body, out status)
            : HandleReconcile(context, body, out status);
    }

    private (int Status, string Response) HandleRequest(
        RuntimeV4ExpertContext context, string body, out int status)
    {
        status = Unavailable;
        if (!TryParseRequest(context, body, out ParsedExpertRequest? request))
            return (400, Error("runtime_v4_expert_action_invalid"));
        RuntimeV3OperationKey operation = request!.Operation;
        if (_receipts.TryGetValue(operation, out RuntimeV4ExpertReceipt? existing))
        {
            if (existing.Action != request.Action || existing.StateId != request.StateId
                || existing.Generation != request.Generation)
            {
                status = Rejected;
                return (status, Response(context, request.StateId, request.Generation,
                    operation.OperationId, request.Action, "rejected", null, false,
                    "sts2.runtime/idempotency_conflict", null));
            }
            return RenderReceipt(context, existing, out status);
        }
        if (_source is null || _thread is null)
            return (status, Response(context, request.StateId, request.Generation,
                operation.OperationId, request.Action, "unknown", null, false,
                "sts2.game-mod/host_unavailable", null));
        if (_receipts.Count >= MaxReceipts)
            return (Rejected, Response(context, request.StateId, request.Generation,
                operation.OperationId, request.Action, "rejected", null, false,
                "sts2.runtime/operation_capacity", null));

        RuntimeV4ExpertGameplayObservation? before = null;
        bool dispatched = false;
        try
        {
            bool ran = InvokeHost(() =>
            {
                before = _source.ObserveExpert();
                dispatched = before.StateId == request.StateId
                    && before.Generation == request.Generation
                    && ContainsExact(before.LegalActions, request.Action)
                    && _source.DispatchExpert(operation, request.Action);
            });
            if (!ran)
                return RetainUnknown(context, request, "sts2.game-mod/dispatch_queue_unavailable",
                    out status);
        }
        catch (Exception)
        {
            return RetainUnknown(context, request, "sts2.game-mod/dispatch_outcome_unknown",
                out status);
        }
        if (!dispatched)
        {
            status = Rejected;
            if (_receipts.Count < MaxReceipts)
                _receipts.Add(operation, new RuntimeV4ExpertReceipt(
                    operation, request.Action, request.StateId, request.Generation,
                    "rejected", null, before is null
                        ? "sts2.game-mod/observation_unavailable"
                        : "sts2.game-mod/action_not_current"));
            return (status, Response(context, request.StateId, request.Generation,
                operation.OperationId, request.Action, "rejected", null, false,
                before is null ? "sts2.game-mod/observation_unavailable"
                    : "sts2.game-mod/action_not_current", null));
        }
        var receipt = new RuntimeV4ExpertReceipt(operation, request.Action,
            request.StateId, request.Generation, "accepted", null, null);
        _receipts.Add(operation, receipt);
        return RenderReceipt(context, receipt, out status);
    }

    private (int Status, string Response) HandleReconcile(
        RuntimeV4ExpertContext context, string operationId, out int status)
    {
        status = Unavailable;
        if (!RuntimeV3GameplayContract.IsIdentity(operationId))
            return (400, Error("runtime_v4_expert_operation_invalid"));
        RuntimeV3OperationKey operation = new(context.InstanceId, context.SessionId,
            context.LeaseId, context.LeaseEpoch, operationId);
        if (!_receipts.TryGetValue(operation, out RuntimeV4ExpertReceipt? receipt))
        {
            status = Unavailable;
            return (status, Response(context, "unknown", 0, operationId, null,
                "unknown", null, false, "sts2.game-mod/operation_not_found", null));
        }
        if (receipt.Status is not "accepted" and not "unknown")
            return RenderReceipt(context, receipt, out status);
        RuntimeV4ExpertGameplayObservation? after = null;
        try
        {
            bool ran = _source is not null && _thread is not null && InvokeHost(() =>
                after = _source.CompleteExpert(operation, receipt.Action));
            if (ran && after is not null)
            {
                RuntimeV4ExpertReceipt settled = receipt with
                {
                    Status = "settled", Observation = after
                };
                _receipts[operation] = settled;
                return RenderReceipt(context, settled, out status);
            }
        }
        catch (Exception)
        {
            // An unknown result is retried through the same operation identity; no second action
            // is admitted and the pending receipt remains available for later reconciliation.
        }
        status = Unavailable;
        return (status, Response(context, receipt.StateId, receipt.Generation, operationId,
            receipt.Action, "unknown", null, false, "sts2.game-mod/outcome_unknown", null));
    }

    private (int Status, string Response) RetainUnknown(
        RuntimeV4ExpertContext context, ParsedExpertRequest request, string error,
        out int status)
    {
        RuntimeV3OperationKey operation = request.Operation;
        if (!_receipts.ContainsKey(operation) && _receipts.Count < MaxReceipts)
            _receipts.Add(operation, new RuntimeV4ExpertReceipt(operation, request.Action,
                request.StateId, request.Generation, "unknown", null, error));
        status = Unavailable;
        return (status, Response(context, request.StateId, request.Generation,
            operation.OperationId, request.Action, "unknown", null, false, error, null));
    }

    private static (int Status, string Response) RenderReceipt(
        RuntimeV4ExpertContext context, RuntimeV4ExpertReceipt receipt, out int status)
    {
        if (receipt.Status == "settled" && receipt.Observation is null)
        {
            status = Unavailable;
            return (Unavailable, Error("runtime_v4_expert_observation_unavailable"));
        }
        status = receipt.Status switch
        {
            "rejected" => Rejected,
            "settled" => Accepted,
            _ => Unavailable
        };
        ulong generation = receipt.Observation?.Generation ?? receipt.Generation;
        string stateId = receipt.Observation?.StateId ?? receipt.StateId;
        string? error = receipt.Status == "accepted" ? null : receipt.ErrorCode;
        return (status, Response(context, stateId, generation, receipt.Operation.OperationId,
            receipt.Action, receipt.Status, receipt.Observation, receipt.Status == "settled",
            error, receipt.Status == "settled" ? receipt.Generation : null));
    }

    private bool InvokeHost(Action work)
    {
        if (_thread is null) return false;
        bool ran = false;
        _thread.Enqueue(() => { work(); ran = true; });
        return ran;
    }

    private static bool ContainsExact(
        IReadOnlyList<RuntimeV4ExpertGameplayAction> actions,
        RuntimeV4ExpertGameplayAction requested)
    {
        foreach (RuntimeV4ExpertGameplayAction action in actions)
            if (action == requested) return true;
        return false;
    }

    private sealed record ParsedExpertRequest(
        RuntimeV3OperationKey Operation,
        RuntimeV4ExpertGameplayAction Action,
        string StateId,
        ulong Generation);

    private sealed record RuntimeV4ExpertReceipt(
        RuntimeV3OperationKey Operation,
        RuntimeV4ExpertGameplayAction Action,
        string StateId,
        ulong Generation,
        string Status,
        RuntimeV4ExpertGameplayObservation? Observation,
        string? ErrorCode);

    private static string Error(string code) => JsonSerializer.Serialize(new { error_code = code });
}
