// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV4ExpertRestActionSupport
{
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
            if (ran && completion is { Status: "settled" or "cancelled" }
                && (receipt.AdmissionSelector is null
                    || CompletionMatchesAdmission(receipt, completion)))
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
        out int status,
        RuntimeV4ExpertRestSelector? admissionSelector = null)
    {
        RuntimeV4ExpertRestReceipt receipt = new(
            request.Operation, request.Action, request.StateId, request.Generation,
            "unknown", null, null, null, errorCode, SnapshotSelector(admissionSelector));
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
        string? ErrorCode,
        RuntimeV4ExpertRestSelector? AdmissionSelector);

    private static string Error(string code) => JsonSerializer.Serialize(new { error_code = code });
}
