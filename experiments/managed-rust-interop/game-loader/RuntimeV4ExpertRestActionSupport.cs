// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Host-owned projection of the rest surface. The projection contains only owned transport
/// values; native Godot objects remain inside the LiveCombatSource implementation.
/// </summary>
internal sealed record RuntimeV4ExpertRestHostProjection(
    RuntimeV4ExpertGameplayObservation Observation,
    IReadOnlyList<RuntimeV4ExpertRestActionReference> LegalActions,
    RuntimeV4ExpertRestSelector? Selector = null);

/// <summary>Typed completion returned after the source has evidence for one operation.</summary>
internal sealed record RuntimeV4ExpertRestHostCompletion(
    string Status,
    RuntimeV4ExpertGameplayObservation? Observation,
    RuntimeV4ExpertRestTransition? Transition,
    RuntimeV4ExpertRestEffectWitness? EffectWitness,
    string? ErrorCode);

/// <summary>
/// The narrow host boundary for the additive rest profile. Implementations must perform every
/// native read and mutation on the host thread and retain the exact native operation identity.
/// </summary>
internal interface IRuntimeV4ExpertRestHostSource
{
    RuntimeV4ExpertRestHostProjection ObserveRest();

    bool DispatchRest(
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action,
        RuntimeV4ExpertRestHostProjection current);

    RuntimeV4ExpertRestHostCompletion? CompleteRest(
        RuntimeV4ExpertRestOperation operation,
        RuntimeV4ExpertRestActionReference action);
}

/// <summary>
/// Authenticated host-thread owner for the additive v4 rest action transport.
/// </summary>
internal sealed partial class RuntimeV4ExpertRestActionSupport
{
    private const int Accepted = 200;
    private const int Rejected = 409;
    private const int Unavailable = 503;

    private readonly IRuntimeV4ExpertRestHostSource? _source;
    private readonly Action<Action>? _enqueue;
    private readonly Func<bool> _canDispatch;
    private readonly Dictionary<RuntimeV4ExpertRestOperation, RuntimeV4ExpertRestReceipt> _receipts = new();

    private RuntimeV4ExpertRestActionSupport(
        IRuntimeV4ExpertRestHostSource? source,
        Action<Action>? enqueue,
        Func<bool>? canDispatch)
    {
        _source = source;
        _enqueue = enqueue;
        _canDispatch = canDispatch ?? (() => true);
    }

    internal static RuntimeV4ExpertRestActionSupport Unconfigured() =>
        new(null, null, null);

    // The callback overload keeps the serialized producer/consumer test independent of the
    // proprietary host assembly while exercising the same support implementation and source
    // interface. Production composition passes the existing host-thread Enqueue method group.
    internal static RuntimeV4ExpertRestActionSupport WithHost(
        IRuntimeV4ExpertRestHostSource source,
        Action<Action> enqueue,
        Func<bool>? canDispatch = null) =>
        new(source, enqueue, canDispatch);

    internal bool HasPendingMutation
    {
        get
        {
            foreach (RuntimeV4ExpertRestReceipt receipt in _receipts.Values)
            {
                if (receipt.Status is "accepted" or "unknown") return true;
            }
            return false;
        }
    }

    internal (int Status, string Response) Handle(
        RuntimeV4ExpertRestContext context,
        string body,
        out int status)
    {
        status = Unavailable;
        if (body.Length > RuntimeV4ExpertRestActionContract.MaxRequestBytes)
        {
            status = 400;
            return (400, Error("runtime_v4_expert_rest_action_oversized"));
        }

        string trimmed = body.TrimStart();
        return trimmed.StartsWith('{')
            ? HandleRequest(context, body, out status)
            : HandleReconcile(context, body, out status);
    }

    private (int Status, string Response) HandleRequest(
        RuntimeV4ExpertRestContext context,
        string body,
        out int status)
    {
        status = Unavailable;
        if (!RuntimeV4ExpertRestActionCodec.TryParseRequest(body, context,
                out RuntimeV4ExpertRestRequest? request, out string error))
        {
            status = 400;
            return (400, Error(error.Length == 0
                ? "runtime_v4_expert_rest_action_invalid" : error));
        }

        RuntimeV4ExpertRestRequest parsed = request!;
        RuntimeV4ExpertRestOperation operation = parsed.Operation;
        if (_receipts.TryGetValue(operation, out RuntimeV4ExpertRestReceipt? existing))
        {
            if (existing.Action != parsed.Action || existing.StateId != parsed.StateId
                || existing.Generation != parsed.Generation)
            {
                status = Rejected;
                return Render(context, new RuntimeV4ExpertRestReceipt(
                    operation, parsed.Action, parsed.StateId, parsed.Generation,
                    "rejected", null, null, null, "sts2.runtime/idempotency_conflict"),
                    out status);
            }
            return Render(context, existing, out status);
        }

        if (_source is null || _enqueue is null)
            return RetainUnknown(context, parsed, "sts2.game-mod/host_unavailable", out status);
        if (_receipts.Count >= RuntimeV4ExpertRestActionContract.MaxReceipts)
        {
            status = Rejected;
            return (Rejected, Error("sts2.runtime/operation_capacity"));
        }
        if (HasPendingMutation || !_canDispatch())
        {
            RuntimeV4ExpertRestReceipt rejected = new(
                operation, parsed.Action, parsed.StateId, parsed.Generation,
                "rejected", null, null, null, "sts2.runtime/operation_in_progress");
            _receipts.Add(operation, rejected);
            return Render(context, rejected, out status);
        }

        RuntimeV4ExpertRestHostProjection? current = null;
        bool dispatched = false;
        try
        {
            bool ran = InvokeHost(() =>
            {
                current = _source.ObserveRest();
                dispatched = current.Observation.StateId == parsed.StateId
                    && current.Observation.Generation == parsed.Generation
                    && ContainsExact(current.LegalActions, parsed.Action)
                    && _source.DispatchRest(operation, parsed.Action, current);
            });
            if (!ran)
                return RetainUnknown(context, parsed, "sts2.game-mod/dispatch_queue_unavailable",
                    out status);
        }
        catch (Exception)
        {
            return RetainUnknown(context, parsed, "sts2.game-mod/dispatch_outcome_unknown",
                out status);
        }

        if (!dispatched)
        {
            RuntimeV4ExpertRestReceipt rejected = new(
                operation, parsed.Action, parsed.StateId, parsed.Generation,
                "rejected", null, null, null,
                current is null
                    ? "sts2.game-mod/observation_unavailable"
                    : "sts2.game-mod/rest_option_not_legal");
            if (_receipts.Count < RuntimeV4ExpertRestActionContract.MaxReceipts)
                _receipts.Add(operation, rejected);
            return Render(context, rejected, out status);
        }

        RuntimeV4ExpertRestReceipt accepted = new(
            operation, parsed.Action, parsed.StateId, parsed.Generation,
            "accepted", null, null, null, null);
        _receipts.Add(operation, accepted);
        return Render(context, accepted, out status);
    }

    private (int Status, string Response) HandleReconcile(
        RuntimeV4ExpertRestContext context,
        string operationId,
        out int status)
    {
        status = Unavailable;
        if (!RuntimeV4ExpertRestActionContract.IsIdentity(operationId))
        {
            status = 400;
            return (400, Error("runtime_v4_expert_rest_operation_invalid"));
        }

        RuntimeV4ExpertRestOperation operation = new(
            context.InstanceId, context.SessionId, context.LeaseId,
            context.LeaseEpoch, operationId);
        if (!_receipts.TryGetValue(operation, out RuntimeV4ExpertRestReceipt? receipt))
        {
            status = 404;
            return (404, Error("sts2.game-mod/operation_not_found"));
        }
        if (receipt.Status is not ("accepted" or "unknown"))
            return Render(context, receipt, out status);
        if (_source is null || _enqueue is null)
            return Render(context, receipt with
            {
                Status = "unknown", ErrorCode = "sts2.game-mod/host_unavailable"
            }, out status);

        RuntimeV4ExpertRestHostCompletion? completion = null;
        try
        {
            bool ran = InvokeHost(() =>
                completion = _source.CompleteRest(operation, receipt.Action));
            if (ran && completion is { Status: "settled" or "cancelled" })
            {
                RuntimeV4ExpertRestReceipt completed = receipt with
                {
                    Status = completion.Status,
                    Observation = completion.Observation,
                    Transition = completion.Transition,
                    EffectWitness = completion.EffectWitness,
                    ErrorCode = completion.ErrorCode
                };
                if (completion.Status == "settled"
                    && !RuntimeV4ExpertRestActionCodec.TrySerializeResponse(
                        ToResponse(context, completed), out _, out _))
                {
                    return Render(context, receipt with
                    {
                        Status = "unknown", ErrorCode = "sts2.game-mod/settlement_unproven"
                    }, out status);
                }
                _receipts[operation] = completed;
                return Render(context, completed, out status);
            }
        }
        catch (Exception)
        {
            // An uncertain host call retains the original operation and is reconciled by the
            // same opaque identity. It is never dispatched a second time.
        }

        RuntimeV4ExpertRestReceipt unknown = receipt with
        {
            Status = "unknown", ErrorCode = "sts2.game-mod/outcome_unknown"
        };
        _receipts[operation] = unknown;
        return Render(context, unknown, out status);
    }

    private (int Status, string Response) RetainUnknown(
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestRequest request,
        string errorCode,
        out int status)
    {
        RuntimeV4ExpertRestReceipt receipt = new(
            request.Operation, request.Action, request.StateId, request.Generation,
            "unknown", null, null, null, errorCode);
        if (!_receipts.ContainsKey(request.Operation)
            && _receipts.Count < RuntimeV4ExpertRestActionContract.MaxReceipts)
            _receipts.Add(request.Operation, receipt);
        return Render(context, receipt, out status);
    }

    private static (int Status, string Response) Render(
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestReceipt receipt,
        out int status)
    {
        status = receipt.Status switch
        {
            "accepted" => Accepted,
            "settled" => Accepted,
            "rejected" => Rejected,
            "cancelled" => Accepted,
            _ => Unavailable
        };
        RuntimeV4ExpertRestResponse response = ToResponse(context, receipt);
        if (!RuntimeV4ExpertRestActionCodec.TrySerializeResponse(response,
                out string json, out _))
        {
            status = Unavailable;
            return (Unavailable, Error("sts2.game-mod/rest_response_unavailable"));
        }
        return (status, json);
    }

    private static RuntimeV4ExpertRestResponse ToResponse(
        RuntimeV4ExpertRestContext context,
        RuntimeV4ExpertRestReceipt receipt) => new(
            context,
            receipt.Observation?.StateId ?? receipt.StateId,
            receipt.Observation?.Generation ?? receipt.Generation,
            receipt.Operation,
            receipt.Action,
            receipt.Status,
            receipt.Observation,
            receipt.Transition,
            receipt.EffectWitness,
            receipt.ErrorCode);

    private bool InvokeHost(Action work)
    {
        if (_enqueue is null) return false;
        bool ran = false;
        _enqueue(() => { work(); ran = true; });
        return ran;
    }

    private static bool ContainsExact(
        IReadOnlyList<RuntimeV4ExpertRestActionReference> actions,
        RuntimeV4ExpertRestActionReference requested)
    {
        foreach (RuntimeV4ExpertRestActionReference action in actions)
            if (action == requested) return true;
        return false;
    }

    private sealed record RuntimeV4ExpertRestReceipt(
        RuntimeV4ExpertRestOperation Operation,
        RuntimeV4ExpertRestActionReference Action,
        string StateId,
        ulong Generation,
        string Status,
        RuntimeV4ExpertGameplayObservation? Observation,
        RuntimeV4ExpertRestTransition? Transition,
        RuntimeV4ExpertRestEffectWitness? EffectWitness,
        string? ErrorCode);

    private static string Error(string code) => JsonSerializer.Serialize(new { error_code = code });
}
