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

        if (TryGetDuplicate(operationId, fingerprint, out CoopOperationReceipt duplicate))
        {
            return duplicate;
        }

        CoopHostObservation before = Observe();
        if (!CanDispatchRejoin(before, before.HostGeneration, actorPeerId, out string error))
        {
            return Rejected(operationId, before.HostGeneration, error);
        }
        var accepted = new CoopOperationReceipt(operationId, fingerprint, CoopOutcome.Accepted,
            before.HostGeneration, null, null, before, null)
        {
            AdmissionAuthorityId = before.AuthorityId,
            AdmissionAuthorityEpoch = before.AuthorityEpoch
        };
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
        try
        {
            // JoinFlow acceptance is asynchronous. A converged observation from the same
            // frame may describe the pre-rejoin state, so require the native adapter's own
            // recovery witness before allowing this initial dispatch to settle.
            if (!_port.IsRejoinSettled())
            {
                return StoreRejoin(current, CoopOutcome.Accepted, after,
                    "rejoin_recovery_pending");
            }
        }
        catch
        {
            return StoreRejoin(current, CoopOutcome.Accepted, after,
                "rejoin_settlement_unknown");
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

        try
        {
            if (!_port.IsRejoinSettled())
                return true;
        }
        catch
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
        string admissionAuthorityId = receipt.AdmissionAuthorityId
            ?? receipt.Observation.AuthorityId;
        string admissionAuthorityEpoch = receipt.AdmissionAuthorityEpoch
            ?? receipt.Observation.AuthorityEpoch;
        if (after.RecoveryRequired || !after.AllConnectedPeersConverged())
        {
            return true;
        }
        if (!string.Equals(after.AuthorityId, admissionAuthorityId,
                StringComparison.Ordinal)
            || !string.Equals(after.AuthorityEpoch, admissionAuthorityEpoch,
                StringComparison.Ordinal))
        {
            // The reconnect may have crossed into a new native authority while the admitted
            // receipt was still pending. It is unsafe to call that recovery successful, but
            // retaining it as Accepted would block every later mutation forever. Retire the
            // receipt as a fenced rejection so the caller can observe the authority transition
            // and submit a fresh operation under the new lineage.
            updated = StoreRejoin(receipt, CoopOutcome.Rejected, after,
                "native_authority_changed");
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
