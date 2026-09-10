// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class CoopNativeRuntime
{
    private static (int Status, string Response) ReceiptResponse(
        string instanceId, string sessionId, string leaseId, string correlationId,
        ulong leaseEpoch, CoopOperationReceipt receipt)
    {
        if (!receipt.Observation.Validate(out _)
            || receipt.Observation.Peers.Count < 2)
        {
            return Error(Rejected, receipt.ErrorCode ?? "coop_native_operation_rejected");
        }
        string status = receipt.Outcome switch
        {
            CoopOutcome.Accepted => "accepted",
            CoopOutcome.Settled => "settled",
            CoopOutcome.Rejected => "rejected",
            CoopOutcome.Unknown => "unknown",
            _ => "recovery_required"
        };
        Dictionary<string, object?> value = Envelope(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            "effect_response", receipt.OperationId, null, null, null, null, status,
            receipt.Observation, receipt.Outcome == CoopOutcome.Settled ? receipt.Effect : null, null,
            null, receipt);
        return (receipt.Outcome == CoopOutcome.Rejected ? Rejected : Accepted, Serialize(value));
    }

    private static (int Status, string Response) RecoveryResponse(
        string instanceId, string sessionId, string leaseId, string correlationId,
        ulong leaseEpoch, CoopOperationReceipt receipt, string recoveryKind)
    {
        if (!receipt.Observation.Validate(out _)
            || receipt.Observation.Peers.Count < 2)
        {
            return Error(Rejected, receipt.ErrorCode ?? "coop_native_recovery_rejected");
        }
        string status = receipt.Outcome switch
        {
            CoopOutcome.Recovered or CoopOutcome.Settled => "settled",
            CoopOutcome.Rejected => "rejected",
            _ => "unknown"
        };
        Dictionary<string, object?> value = Envelope(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            "recovery_response", receipt.OperationId, null, null, null, null, status,
            receipt.Observation, null, new { kind = recoveryKind, rejoin_epoch = receipt.AfterHostGeneration ?? receipt.BeforeHostGeneration },
            null, receipt);
        return (receipt.Outcome == CoopOutcome.Rejected ? Rejected : Accepted, Serialize(value));
    }

    private static (int Status, string Response) ObservationResponse(
        string instanceId, string sessionId, string leaseId, string correlationId,
        ulong leaseEpoch, CoopHostObservation observation)
    {
        Dictionary<string, object?> value = Envelope(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            "observation", null, null, null, null, null, null, observation, null, null, null, null);
        return (Accepted, Serialize(value));
    }

    private static (int Status, string Response) CatalogResponse(
        string instanceId, string sessionId, string leaseId, string correlationId,
        ulong leaseEpoch, CoopNativeLegalCatalog catalog, CoopHostObservation observation)
    {
        Dictionary<string, object?> value = Envelope(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            "legal_catalog_response", null, catalog.ActorPeerId, catalog.HostGeneration,
            null, null, null, observation, null, null, catalog, null);
        return (Accepted, Serialize(value));
    }

    private static Dictionary<string, object?> Envelope(
        string instanceId, string sessionId, string leaseId, string correlationId, ulong leaseEpoch,
        string kind, string? operationId, string? actorPeer, ulong? expectedGeneration,
        object? action, object? vote, string? status, CoopHostObservation observation,
        CoopEffectWitness? effect, object? recovery, CoopNativeLegalCatalog? catalog,
        CoopOperationReceipt? receipt)
    {
        if (!observation.Validate(out string error))
        {
            throw new InvalidOperationException(error);
        }
        if (observation.Peers.Count < 2)
        {
            throw new InvalidOperationException("native co-op profile requires a multiplayer roster");
        }
        return new Dictionary<string, object?>
        {
            ["protocol_version"] = ProtocolVersion,
            ["schema_digest"] = SchemaDigest,
            ["provenance"] = new { artifact = Artifact, source = SchemaSource, generator = "hand-authored" },
            ["correlation_id"] = correlationId,
            ["instance_id"] = instanceId,
            ["session_id"] = sessionId,
            ["lease_id"] = leaseId,
            ["lease_epoch"] = leaseEpoch,
            ["kind"] = kind,
            ["operation_id"] = operationId,
            ["actor_peer"] = actorPeer,
            ["expected_host_generation"] = expectedGeneration,
            ["action"] = action,
            ["vote"] = vote,
            ["status"] = status,
            ["observation"] = ObservationValue(observation),
            ["effect"] = effect is null ? null : new
            {
                effect_id = effect.EffectId,
                operation_id = effect.OperationId,
                kind = effect.EffectKind,
                from_generation = effect.FromHostGeneration,
                to_generation = effect.ToHostGeneration,
                state_digest = effect.StateDigest,
                authority_epoch = effect.AuthorityEpoch,
                checkpoint_id = effect.CheckpointId,
                authority_id = effect.AuthorityId,
                native_checksum = effect.NativeChecksum
            },
            ["recovery"] = recovery,
            ["catalog"] = catalog is null ? null : CatalogValue(catalog),
            ["receipt"] = receipt is null ? null : ReceiptValue(receipt)
        };
    }

    private static object CatalogValue(CoopNativeLegalCatalog catalog) => new
    {
        host_generation = catalog.HostGeneration,
        actor_peer = catalog.ActorPeerId,
        actions = catalog.Actions.Select(action => new
        {
            action_id = action.ActionId,
            kind = action.Kind,
            target_peer = action.TargetPeer
        }).ToArray(),
        votes = catalog.Votes.Select(vote => new
        {
            proposal_id = vote.ProposalId,
            voter_peer = vote.VoterPeer,
            choice = vote.Choice
        }).ToArray()
    };

    private static object ReceiptValue(CoopOperationReceipt receipt) => new
    {
        operation_id = receipt.OperationId,
        status = receipt.Outcome switch
        {
            CoopOutcome.Accepted => "accepted",
            CoopOutcome.Settled or CoopOutcome.Recovered => "settled",
            CoopOutcome.Rejected => "rejected",
            CoopOutcome.Unknown => "unknown",
            _ => "unknown"
        },
        before_host_generation = receipt.BeforeHostGeneration,
        after_host_generation = receipt.AfterHostGeneration,
        authority_id = receipt.Observation.AuthorityId,
        authority_epoch = receipt.Observation.AuthorityEpoch,
        checkpoint_id = receipt.Effect?.CheckpointId ?? receipt.Observation.CheckpointId,
        state_digest = receipt.Observation.HostStateDigest,
        native_checksum = receipt.Observation.NativeChecksum,
        error_code = receipt.ErrorCode
    };

    private static object ObservationValue(CoopHostObservation observation) => new
    {
        host_authority_epoch = observation.AuthorityEpoch,
        authority_id = observation.AuthorityId,
        run_id = observation.RunId,
        host_sequence_kind = observation.HostSequenceKind,
        host_generation = observation.HostGeneration,
        state_digest = observation.HostStateDigest,
        checkpoint_id = observation.CheckpointId,
        checksum_algorithm = observation.ChecksumAlgorithm,
        checksum_status = observation.ChecksumStatus,
        native_checksum = observation.NativeChecksum,
        host_digest_known = observation.HostDigestKnown,
        host_loading = observation.HostLoading,
        host_divergent = observation.HostDivergent,
        peers = observation.Peers.Select(peer => new
            {
                peer_token = peer.PeerId,
                authority_id = peer.AuthorityId,
            role = peer.IsLocal ? "local" : "ally",
            connected = peer.Connected,
            peer_generation = peer.Generation,
            state_digest = peer.StateDigest,
            rejoin_epoch = peer.RejoinEpoch,
            authority_epoch = peer.AuthorityEpoch,
            checkpoint_id = peer.CheckpointId,
            digest_known = peer.DigestKnown,
            is_loading = peer.IsLoading,
            is_divergent = peer.IsDivergent,
            checksum_status = peer.ChecksumStatus
        }).ToArray()
    };
}
