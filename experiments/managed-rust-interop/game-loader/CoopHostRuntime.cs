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
    private readonly ConcurrentDictionary<string, CoopOperationReceipt> _receipts = new(
        StringComparer.Ordinal);

    internal CoopHostRuntime(ICoopNativeHostPort port)
    {
        _port = port;
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

    internal CoopOperationReceipt Rejoin(string operationId, string actorPeerId, ulong rejoinEpoch)
    {
        if (!RuntimeV3GameplayContract.IsIdentity(operationId)
            || !CoopPeerIdentity.IsOpaque(actorPeerId)
            || rejoinEpoch > RuntimeV3GameplayContract.MaxGeneration)
        {
            return Rejected(operationId, 0, "invalid_rejoin");
        }

        CoopHostObservation before = Observe();
        if (!CanDispatch(before, before.HostGeneration, actorPeerId, out string error))
        {
            return Rejected(operationId, before.HostGeneration, error);
        }
        CoopNativeDispatchResult result;
        try
        {
            result = _port.Rejoin(actorPeerId, rejoinEpoch);
        }
        catch
        {
            result = CoopNativeDispatchResult.Unknown("native_rejoin_outcome_unknown");
        }
        if (result.Outcome == CoopOutcome.Rejected)
        {
            return Rejected(operationId, before.HostGeneration,
                result.ErrorCode ?? "native_rejoin_rejected");
        }

        CoopHostObservation after;
        try
        {
            after = Observe();
        }
        catch
        {
            return Rejected(operationId, before.HostGeneration, "rejoin_observation_unknown");
        }
        CoopOutcome outcome = after.RecoveryRequired || !after.AllConnectedPeersConverged()
            ? CoopOutcome.Unknown
            : CoopOutcome.Recovered;
        return new CoopOperationReceipt(operationId, $"rejoin|{actorPeerId}|{rejoinEpoch}", outcome,
            before.HostGeneration, after.HostGeneration, null, after,
            outcome == CoopOutcome.Recovered ? null : result.ErrorCode ?? "rejoin_recovery_required");
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
}
