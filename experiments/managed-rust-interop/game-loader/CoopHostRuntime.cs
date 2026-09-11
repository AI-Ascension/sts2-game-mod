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
    string? ErrorCode)
{
    // The authority and epoch captured when a rejoin mutation was admitted are immutable
    // settlement fences. Observation is allowed to move forward while a receipt is pending, so
    // reconciliation must never infer the admission lineage from that mutable snapshot.
    internal string? AdmissionAuthorityId { get; init; }
    internal string? AdmissionAuthorityEpoch { get; init; }
}

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
        PruneReceiptsForAuthority(observation);
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
        ConfirmNativeSettlement(operationId);
        receipt = settled;
        return true;
    }

    private void ConfirmNativeSettlement(string operationId)
    {
        try
        {
            _port.ConfirmSettlement(operationId);
        }
        catch
        {
            // Receipt settlement is already fenced by the host observation. A cleanup marker
            // in the adapter must not turn an otherwise settled receipt into an exception.
        }
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
