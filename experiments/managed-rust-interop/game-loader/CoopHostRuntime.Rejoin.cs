// SPDX-License-Identifier: MIT

using System;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopHostRuntime
{
    private const string RejoinFingerprintPrefix = "rejoin|";

    internal CoopOperationReceipt Rejoin(string operationId, string actorPeerId, ulong rejoinEpoch)
    {
        string fingerprint = RejoinFingerprint(actorPeerId, rejoinEpoch);
        if (!RuntimeV3GameplayContract.IsIdentity(operationId)
            || !CoopPeerIdentity.IsOpaque(actorPeerId)
            || rejoinEpoch > RuntimeV3GameplayContract.MaxGeneration)
        {
            return Rejected(operationId, 0, "invalid_rejoin");
        }

        if (_receipts.TryGetValue(operationId, out CoopOperationReceipt? prior))
        {
            return ValidateDuplicate(prior, fingerprint);
        }

        CoopHostObservation before = Observe();
        if (!CanDispatch(before, before.HostGeneration, actorPeerId, out string error))
        {
            return Rejected(operationId, before.HostGeneration, error);
        }
        var accepted = new CoopOperationReceipt(operationId, fingerprint, CoopOutcome.Accepted,
            before.HostGeneration, null, null, before, null);
        if (_receipts.Count >= MaxReceipts || !_receipts.TryAdd(operationId, accepted))
        {
            return Rejected(operationId, before.HostGeneration, "receipt_capacity_exhausted");
        }
        if (!TryPassExternalAdmission(out error))
        {
            _receipts.TryRemove(operationId, out _);
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
        return FinalizeRejoin(operationId, before, result);
    }

    private CoopOperationReceipt FinalizeRejoin(
        string operationId, CoopHostObservation before, CoopNativeDispatchResult result)
    {
        if (!_receipts.TryGetValue(operationId, out CoopOperationReceipt? current))
        {
            return Rejected(operationId, before.HostGeneration, "receipt_missing");
        }
        if (result.Outcome == CoopOutcome.Rejected)
        {
            CoopOperationReceipt rejected = current with
            {
                Outcome = CoopOutcome.Rejected,
                Observation = before,
                ErrorCode = result.ErrorCode ?? "native_rejoin_rejected"
            };
            _receipts[operationId] = rejected;
            return rejected;
        }
        if (result.Outcome != CoopOutcome.Accepted)
        {
            return StoreRejoin(current, CoopOutcome.Unknown, before,
                result.ErrorCode ?? "native_rejoin_outcome_unknown");
        }

        CoopHostObservation after;
        try
        {
            after = Observe();
        }
        catch
        {
            // Native acceptance is known, but the postcondition is not. Keep the receipt
            // pending so recovery can reobserve without issuing a second rejoin mutation.
            return StoreRejoin(current, CoopOutcome.Accepted, before, "rejoin_observation_unknown");
        }
        if (after.RecoveryRequired || !after.AllConnectedPeersConverged()
            || !string.Equals(after.AuthorityId, before.AuthorityId, StringComparison.Ordinal)
            || !string.Equals(after.AuthorityEpoch, before.AuthorityEpoch, StringComparison.Ordinal))
        {
            return StoreRejoin(current, CoopOutcome.Accepted, after, "rejoin_recovery_required");
        }
        return StoreRejoin(current, CoopOutcome.Recovered, after, null);
    }

    private bool ReconcileRejoin(
        CoopOperationReceipt receipt, out CoopOperationReceipt? updated)
    {
        updated = receipt;
        // An unknown native call has no rejoin effect witness. Convergence alone cannot turn
        // that outcome into recovery, because the call may never have reached the transport.
        if (receipt.Outcome != CoopOutcome.Accepted)
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
        if (after.RecoveryRequired || !after.AllConnectedPeersConverged()
            || !string.Equals(after.AuthorityId, receipt.Observation.AuthorityId,
                StringComparison.Ordinal)
            || !string.Equals(after.AuthorityEpoch, receipt.Observation.AuthorityEpoch,
                StringComparison.Ordinal))
        {
            return true;
        }
        updated = StoreRejoin(receipt, CoopOutcome.Recovered, after, null);
        return true;
    }

    private CoopOperationReceipt StoreRejoin(
        CoopOperationReceipt receipt, CoopOutcome outcome,
        CoopHostObservation observation, string? errorCode)
    {
        CoopOperationReceipt updated = receipt with
        {
            Outcome = outcome,
            AfterHostGeneration = observation.HostGeneration,
            Observation = observation,
            ErrorCode = errorCode
        };
        _receipts[receipt.OperationId] = updated;
        return updated;
    }

    private static bool IsRejoinReceipt(CoopOperationReceipt receipt) =>
        receipt.RequestFingerprint.StartsWith(RejoinFingerprintPrefix, StringComparison.Ordinal);

    private static string RejoinFingerprint(string actorPeerId, ulong rejoinEpoch) =>
        $"{RejoinFingerprintPrefix}{actorPeerId}|{rejoinEpoch}";
}
