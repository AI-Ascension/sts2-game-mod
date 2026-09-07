// SPDX-License-Identifier: MIT

using System;
using System.Collections.Concurrent;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed record CoopOperationReceipt(
    string OperationId,
    string RequestFingerprint,
    CoopOutcome Outcome,
    ulong BeforeHostGeneration,
    ulong? AfterHostGeneration,
    CoopEffectWitness? Effect,
    CoopHostObservation Observation,
    string? ErrorCode);

/// <summary>
/// Fenced host-side owner for native co-op operations. The native port is the only component
/// allowed to call host synchronizers; this owner supplies idempotency, caller identity checks,
/// host-generation fencing, and postcondition settlement.
/// </summary>
internal sealed partial class CoopHostRuntime
{
    private const int MaxReceipts = 4096;
    private readonly ICoopNativeHostPort _port;
    // The caller supplies the cross-profile admission predicate. It is evaluated on the
    // serialized game thread immediately before each native mutation so a stale read cannot
    // open a second mutation while another runtime profile is still settling.
    private readonly Func<bool> _canDispatch;
    private readonly ConcurrentDictionary<string, CoopOperationReceipt> _receipts = new(
        StringComparer.Ordinal);

    internal CoopHostRuntime(ICoopNativeHostPort port, Func<bool>? canDispatch = null)
    {
        _port = port;
        _canDispatch = canDispatch ?? (() => true);
    }

    internal bool HasPendingMutation
    {
        get
        {
            foreach (CoopOperationReceipt receipt in _receipts.Values)
            {
                if (receipt.Outcome is CoopOutcome.Accepted or CoopOutcome.Unknown)
                {
                    return true;
                }
            }
            return false;
        }
    }

    internal CoopHostObservation Observe()
    {
        CoopHostObservation observation = _port.Observe();
        if (!observation.Validate(out string error))
        {
            throw new InvalidOperationException($"invalid native co-op host observation: {error}");
        }
        return observation;
    }

    internal bool TryGetReceipt(string operationId, out CoopOperationReceipt? receipt)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(operationId))
        {
            receipt = null;
            return false;
        }

        if (!_receipts.TryGetValue(operationId, out receipt))
        {
            return false;
        }

        if (receipt.Outcome == CoopOutcome.Unknown)
        {
            Reconcile(operationId, out receipt);
        }
        return true;
    }

    internal CoopOperationReceipt DispatchLocalAction(CoopLocalActionRequest request)
    {
        if (!request.Validate(out string error))
        {
            return Rejected(request.OperationId, 0, "invalid_local_action");
        }

        if (_receipts.TryGetValue(request.OperationId, out CoopOperationReceipt? prior))
        {
            return ValidateDuplicate(prior, Fingerprint(request));
        }

        CoopHostObservation before = Observe();
        if (!CanDispatch(before, request.ExpectedHostGeneration, request.ActorPeerId, out error))
        {
            return Rejected(request.OperationId, before.HostGeneration, error);
        }

        var accepted = new CoopOperationReceipt(request.OperationId, Fingerprint(request), CoopOutcome.Accepted,
            before.HostGeneration, null, null, before, null);
        if (_receipts.Count >= MaxReceipts
            || !_receipts.TryAdd(request.OperationId, accepted))
        {
            return Rejected(request.OperationId, before.HostGeneration, "receipt_capacity_exhausted");
        }

        if (!TryPassExternalAdmission(out string admissionError))
        {
            _receipts.TryRemove(request.OperationId, out _);
            return Rejected(request.OperationId, before.HostGeneration, admissionError);
        }

        CoopNativeDispatchResult result;
        try
        {
            result = _port.DispatchLocalAction(request);
        }
        catch
        {
            result = CoopNativeDispatchResult.Unknown("native_dispatch_outcome_unknown");
        }
        return FinalizeDispatch(request.OperationId, before, result);
    }

    internal CoopOperationReceipt SubmitSharedVote(CoopSharedVoteRequest request)
    {
        if (!request.Validate(out string error))
        {
            return Rejected(request.OperationId, 0, "invalid_shared_vote");
        }

        if (_receipts.TryGetValue(request.OperationId, out CoopOperationReceipt? prior))
        {
            return ValidateDuplicate(prior, Fingerprint(request));
        }

        CoopHostObservation before = Observe();
        if (!CanDispatch(before, request.ExpectedHostGeneration, request.VoterPeerId, out error))
        {
            return Rejected(request.OperationId, before.HostGeneration, error);
        }

        var accepted = new CoopOperationReceipt(request.OperationId, Fingerprint(request), CoopOutcome.Accepted,
            before.HostGeneration, null, null, before, null);
        if (_receipts.Count >= MaxReceipts
            || !_receipts.TryAdd(request.OperationId, accepted))
        {
            return Rejected(request.OperationId, before.HostGeneration, "receipt_capacity_exhausted");
        }

        if (!TryPassExternalAdmission(out string admissionError))
        {
            _receipts.TryRemove(request.OperationId, out _);
            return Rejected(request.OperationId, before.HostGeneration, admissionError);
        }

        CoopNativeDispatchResult result;
        try
        {
            result = _port.SubmitSharedVote(request);
        }
        catch
        {
            result = CoopNativeDispatchResult.Unknown("native_vote_outcome_unknown");
        }
        return FinalizeDispatch(request.OperationId, before, result);
    }

    internal bool Reconcile(string operationId, out CoopOperationReceipt? receipt)
    {
        if (!_receipts.TryGetValue(operationId, out receipt))
        {
            return false;
        }

        if (receipt.Outcome is CoopOutcome.Settled or CoopOutcome.Rejected)
        {
            return true;
        }

        if (IsRejoinReceipt(receipt))
        {
            return ReconcileRejoin(receipt, out receipt);
        }

        CoopEffectWitness? effect;
        try
        {
            effect = _port.Reconcile(operationId);
        }
        catch
        {
            effect = null;
        }

        if (effect is null || !effect.Validate(out _)
            || !string.Equals(effect.OperationId, operationId, StringComparison.Ordinal))
        {
            return true;
        }

        CoopHostObservation after;
        try
        {
            after = Observe();
        }
        catch
        {
            return true;
        }

        if (!IsSettled(receipt, after, effect))
        {
            return true;
        }

        CoopOperationReceipt settled = receipt with
        {
            Outcome = CoopOutcome.Settled,
            AfterHostGeneration = after.HostGeneration,
            Effect = effect,
            Observation = after,
            ErrorCode = null
        };
        _receipts[operationId] = settled;
        receipt = settled;
        return true;
    }

    private bool TryPassExternalAdmission(out string error)
    {
        try
        {
            if (_canDispatch())
            {
                error = string.Empty;
                return true;
            }
            error = "operation_in_progress";
            return false;
        }
        catch
        {
            // A failed cross-profile probe cannot establish that the native mutation is safe.
            error = "operation_admission_unavailable";
            return false;
        }
    }
}
