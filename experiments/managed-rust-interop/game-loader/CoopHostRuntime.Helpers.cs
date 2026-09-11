// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopHostRuntime
{
    private CoopOperationReceipt FinalizeDispatch(
        string operationId,
        CoopHostObservation before,
        CoopNativeDispatchResult result)
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
                ErrorCode = result.ErrorCode ?? "native_rejected"
            };
            _receipts[operationId] = rejected;
            return rejected;
        }

        if (result.Effect is not null && result.Effect.Validate(out _))
        {
            CoopHostObservation after = Observe();
            if (IsSettled(current, after, result.Effect))
            {
                CoopOperationReceipt settled = current with
                {
                    Outcome = CoopOutcome.Settled,
                    AfterHostGeneration = after.HostGeneration,
                    Effect = result.Effect,
                    Observation = after,
                    ErrorCode = null
                };
                _receipts[operationId] = settled;
                ConfirmNativeSettlement(operationId);
                return settled;
            }
        }

        CoopOperationReceipt pending = current with
        {
            // An effect witness that cannot pass the fresh all-peer postcondition is uncertain:
            // the caller must reconcile the same operation after reconnecting, never retry it.
            Outcome = result.Outcome == CoopOutcome.Unknown || result.Effect is not null
                ? CoopOutcome.Unknown
                : CoopOutcome.Accepted,
            ErrorCode = result.ErrorCode ?? "settlement_unproven"
        };
        _receipts[operationId] = pending;
        return pending;
    }

    private bool CanDispatch(CoopHostObservation observation,
        ulong expectedHostGeneration, string actorPeerId, out string error)
    {
        if (HasPendingMutation)
        {
            error = "operation_in_progress";
            return false;
        }
        if (observation.RecoveryRequired)
        {
            error = "recovery_required";
            return false;
        }
        if (observation.Role == CoopHostRole.Singleplayer)
        {
            error = "multiplayer_not_connected";
            return false;
        }
        if (observation.HostGeneration != expectedHostGeneration)
        {
            error = "stale_host_generation";
            return false;
        }
        if ((!observation.HostDigestKnown
                && !_port.MayDispatchWithUnknownDigest(observation))
            || observation.HostLoading || observation.HostDivergent)
        {
            error = observation.HostDivergent
                ? "native_state_divergent"
                : "native_state_not_settled";
            return false;
        }
        bool actorKnown = false;
        foreach (CoopPeerSnapshot peer in observation.Peers)
        {
            if (!peer.Connected)
            {
                error = "peer_not_connected";
                return false;
            }
            if (peer.IsLoading)
            {
                error = "peer_loading";
                return false;
            }
            if (peer.IsDivergent)
            {
                error = "peer_state_divergent";
                return false;
            }
            if (string.Equals(peer.PeerId, actorPeerId, StringComparison.Ordinal))
            {
                actorKnown = true;
            }
        }
        if (!actorKnown)
        {
            error = "unknown_or_stale_peer_identity";
            return false;
        }
        if (!string.Equals(actorPeerId, observation.LocalPeerId, StringComparison.Ordinal))
        {
            error = "actor_is_not_local_peer";
            return false;
        }

        CoopPeerSnapshot? local = null;
        foreach (CoopPeerSnapshot peer in observation.Peers)
        {
            if (string.Equals(peer.PeerId, actorPeerId, StringComparison.Ordinal))
            {
                local = peer;
                break;
            }
        }
        if (local is null || !local.Connected)
        {
            error = "local_peer_not_connected";
            return false;
        }
        CoopNativePeerBinding binding;
        try
        {
            if (!_port.TryResolvePeer(actorPeerId, out binding))
            {
                error = "unknown_or_stale_peer_identity";
                return false;
            }
        }
        catch
        {
            error = "peer_identity_lookup_failed";
            return false;
        }
        if (!binding.Validate(out _)
            || !string.Equals(binding.OpaquePeerId, actorPeerId, StringComparison.Ordinal))
        {
            error = "unknown_or_stale_peer_identity";
            return false;
        }
        error = string.Empty;
        return true;
    }

    private bool CanDispatchRejoin(CoopHostObservation observation,
        ulong expectedHostGeneration, string actorPeerId, out string error)
    {
        // Rejoin is the recovery operation for a disconnected local client. It deliberately
        // shares the generation and identity fences with ordinary mutations while allowing the
        // local peer's Connected bit and the observation's RecoveryRequired bit to be false.
        if (observation.Role == CoopHostRole.Singleplayer)
        {
            error = "multiplayer_not_connected";
            return false;
        }
        if (observation.HostGeneration != expectedHostGeneration)
        {
            error = "stale_host_generation";
            return false;
        }
        if (!string.Equals(actorPeerId, observation.LocalPeerId, StringComparison.Ordinal))
        {
            error = "actor_is_not_local_peer";
            return false;
        }

        CoopPeerSnapshot? local = null;
        foreach (CoopPeerSnapshot peer in observation.Peers)
        {
            if (string.Equals(peer.PeerId, actorPeerId, StringComparison.Ordinal))
            {
                local = peer;
                break;
            }
        }
        if (local is null)
        {
            error = "unknown_or_stale_peer_identity";
            return false;
        }
        CoopNativePeerBinding binding;
        try
        {
            if (!_port.TryResolvePeer(actorPeerId, out binding))
            {
                error = "unknown_or_stale_peer_identity";
                return false;
            }
        }
        catch
        {
            error = "peer_identity_lookup_failed";
            return false;
        }
        if (!binding.Validate(out _)
            || !string.Equals(binding.OpaquePeerId, actorPeerId, StringComparison.Ordinal))
        {
            error = "unknown_or_stale_peer_identity";
            return false;
        }
        error = string.Empty;
        return true;
    }

    private static bool IsSettled(CoopOperationReceipt receipt,
        CoopHostObservation after, CoopEffectWitness effect) =>
        string.Equals(effect.OperationId, receipt.OperationId, StringComparison.Ordinal)
        && string.Equals(effect.AuthorityId, receipt.Observation.AuthorityId, StringComparison.Ordinal)
        && string.Equals(effect.AuthorityId, after.AuthorityId, StringComparison.Ordinal)
        && effect.FromHostGeneration == receipt.BeforeHostGeneration
        && effect.ToHostGeneration == after.HostGeneration
        && after.HostGeneration > receipt.BeforeHostGeneration
        && string.Equals(effect.StateDigest, after.HostStateDigest, StringComparison.Ordinal)
        && after.AllConnectedPeersConverged();

    private static CoopOperationReceipt ValidateDuplicate(CoopOperationReceipt prior,
        string requestFingerprint) => prior.RequestFingerprint == requestFingerprint
        ? prior
        : prior with { Outcome = CoopOutcome.Rejected, ErrorCode = "idempotency_conflict" };

    private static string Fingerprint(CoopLocalActionRequest request) =>
        $"local|{request.ExpectedHostGeneration}|{request.ActorPeerId}|{request.ActionKind}|{request.Value}|{request.TargetPeerId}";

    private static string Fingerprint(CoopSharedVoteRequest request) =>
        $"vote|{request.ProposalId}|{request.ExpectedHostGeneration}|{request.VoterPeerId}|{request.Domain}|{request.Choice}";

    private static CoopOperationReceipt Rejected(string operationId,
        ulong generation, string errorCode)
    {
        // Rejected requests have no native snapshot. The caller must observe before retrying;
        // this placeholder is never exported as a successful host observation.
        var empty = new CoopHostObservation("unavailable", "peer:local", CoopHostRole.Unknown,
            "unknown", null, generation, "digest:unknown",
            new[] { new CoopPeerSnapshot("peer:local", true, false, generation, "digest:unknown") },
            true, null);
        return new CoopOperationReceipt(operationId, string.Empty, CoopOutcome.Rejected, generation,
            null, null, empty, errorCode);
    }
}
