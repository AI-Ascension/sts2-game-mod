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
            receipt.Observation, receipt.Outcome == CoopOutcome.Settled ? receipt.Effect : null, null);
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
            receipt.Observation, null, new { kind = recoveryKind, rejoin_epoch = receipt.AfterHostGeneration ?? receipt.BeforeHostGeneration });
        return (receipt.Outcome == CoopOutcome.Rejected ? Rejected : Accepted, Serialize(value));
    }

    private static (int Status, string Response) ObservationResponse(
        string instanceId, string sessionId, string leaseId, string correlationId,
        ulong leaseEpoch, CoopHostObservation observation)
    {
        Dictionary<string, object?> value = Envelope(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            "observation", null, null, null, null, null, null, observation, null, null);
        return (Accepted, Serialize(value));
    }

    private static Dictionary<string, object?> Envelope(
        string instanceId, string sessionId, string leaseId, string correlationId, ulong leaseEpoch,
        string kind, string? operationId, string? actorPeer, ulong? expectedGeneration,
        object? action, object? vote, string? status, CoopHostObservation observation,
        CoopEffectWitness? effect, object? recovery)
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
                native_checksum = (string?)null
            },
            ["recovery"] = recovery
        };
    }

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

    private static bool HasClosedEnvelope(JsonElement root)
    {
        if (root.ValueKind != JsonValueKind.Object) return false;
        string[] fields = { "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch", "kind", "operation_id", "actor_peer", "expected_host_generation", "action", "vote", "status", "observation", "effect", "recovery" };
        foreach (string field in fields)
        {
            if (!root.TryGetProperty(field, out _)) return false;
        }
        return true;
    }

    private static bool TryRequestFields(JsonElement root, out string operationId, out string actorPeer, out ulong expectedGeneration)
    {
        operationId = NullableStringField(root, "operation_id") ?? string.Empty;
        actorPeer = NullableStringField(root, "actor_peer") ?? string.Empty;
        expectedGeneration = 0;
        return RuntimeV3GameplayContract.IsIdentity(operationId)
            && CoopPeerIdentity.IsOpaque(actorPeer)
            && TryNumber(root, "expected_host_generation", out expectedGeneration);
    }

    private static bool TryAction(JsonElement action, out string kind, out string actionId, out string? targetPeer)
    {
        kind = StringField(action, "kind") ?? string.Empty;
        actionId = StringField(action, "action_id") ?? string.Empty;
        targetPeer = NullableStringField(action, "target_peer");
        return RuntimeV3GameplayContract.IsIdentity(actionId)
            && kind is "play_card" or "end_turn" or "select_card" or "choose_reward" or "confirm_selection"
            && (targetPeer is null || CoopPeerIdentity.IsOpaque(targetPeer));
    }

    private static bool TryVote(JsonElement vote, string actorPeer, out string proposalId,
        out string choice, out string voterPeer, out CoopVoteDomain domain)
    {
        proposalId = StringField(vote, "proposal_id") ?? string.Empty;
        choice = StringField(vote, "choice") ?? string.Empty;
        voterPeer = StringField(vote, "voter_peer") ?? string.Empty;
        domain = VoteDomain(proposalId);
        return RuntimeV3GameplayContract.IsIdentity(proposalId)
            && CoopPeerIdentity.IsOpaque(voterPeer)
            && voterPeer == actorPeer
            && RuntimeV3GameplayContract.IsIdentity(choice);
    }

    private static CoopVoteDomain VoteDomain(string proposalId) =>
        proposalId is "event" || proposalId.StartsWith("event:", StringComparison.Ordinal)
            ? CoopVoteDomain.SharedEvent
            : proposalId is "relic" || proposalId.StartsWith("relic:", StringComparison.Ordinal)
                ? CoopVoteDomain.TreasureRelic
                : proposalId is "map" || proposalId.StartsWith("map:", StringComparison.Ordinal)
                    ? CoopVoteDomain.Map
                    : CoopVoteDomain.PlayerChoice;

    private static string? NullableStringField(JsonElement root, string name) =>
        root.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() : root.TryGetProperty(name, out value) && value.ValueKind == JsonValueKind.Null ? null : null;

    private static string? StringField(JsonElement root, string name) =>
        root.TryGetProperty(name, out JsonElement value) && value.ValueKind == JsonValueKind.String
            ? value.GetString() : null;

    private static bool NumberField(JsonElement root, string name, ulong expected) =>
        TryNumber(root, name, out ulong value) && value == expected;

    private static bool TryNumber(JsonElement root, string name, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(name, out JsonElement number)
            && number.ValueKind == JsonValueKind.Number
            && number.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration;
    }

    private static string Serialize(Dictionary<string, object?> value)
    {
        string output = JsonSerializer.Serialize(value, JsonOptions);
        if (output.Length > MaxResponseBytes) throw new InvalidOperationException("co-op response is oversized");
        return output;
    }

    private static (int Status, string Response) Error(int status, string code) =>
        (status, JsonSerializer.Serialize(new Dictionary<string, object?> { ["error_code"] = code }, JsonOptions));
}
