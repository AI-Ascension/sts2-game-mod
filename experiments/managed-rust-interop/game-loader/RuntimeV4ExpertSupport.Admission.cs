// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV4ExpertSupport
{
    internal bool HasPendingMutation
    {
        get
        {
            foreach (RuntimeV4ExpertReceipt receipt in _receipts.Values)
            {
                if (receipt.Status is "accepted" or "unknown") return true;
            }
            return false;
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
        if (HasPendingMutation || ModEntry.HasPendingNonExpertMutation())
        {
            RuntimeV4ExpertReceipt rejected = new(operation, request.Action,
                request.StateId, request.Generation, "rejected", null,
                "sts2.runtime/operation_in_progress");
            _receipts.Add(operation, rejected);
            return RenderReceipt(context, rejected, out status);
        }

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
}
