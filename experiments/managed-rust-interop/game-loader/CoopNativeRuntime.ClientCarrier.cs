// SPDX-License-Identifier: MIT

using System;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Validates the local half of an opaque first-party carrier.  The carrier retains the frozen
/// v1 request bytes; it does not manufacture a peer identity from a route credential or a
/// client claim.  A terminal response is owned by the authenticated host.
/// </summary>
internal sealed partial class CoopNativeRuntime
{
    internal CoopOperationReceipt DispatchAuthenticatedOpaqueCarrier(ulong nativePeerId,
        string expectedOperationId, byte[] payload)
    {
        if (payload is null || payload.Length == 0
            || payload.Length > OpaqueOperationNativeMessage.MaximumPayloadBytes
            || !_host.TryResolveNativePeer(nativePeerId, out CoopNativePeerBinding binding)
            || !binding.Validate(out _))
        {
            return CarrierRejected("invalid_or_stale_native_peer");
        }

        try
        {
            using JsonDocument document = JsonDocument.Parse(payload,
                new JsonDocumentOptions { MaxDepth = 16, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (!HasClosedEnvelope(root)
                || !TryRequestFields(root, out string operationId, out string actorPeer,
                    out ulong expectedGeneration)
                || !string.Equals(operationId, expectedOperationId, StringComparison.Ordinal)
                || !string.Equals(actorPeer, binding.OpaquePeerId, StringComparison.Ordinal))
            {
                return CarrierRejected("carrier_actor_not_authenticated");
            }

            if (StringField(root, "kind") == "local_action_request"
                && root.TryGetProperty("action", out JsonElement action)
                && action.ValueKind == JsonValueKind.Object
                && TryAction(action, out string actionKind, out string actionId, out string? targetPeer))
            {
                return DispatchAuthenticatedLocalAction(new(operationId, expectedGeneration,
                    binding.OpaquePeerId, actionKind, actionId, targetPeer), binding);
            }

            if (StringField(root, "kind") == "shared_vote_request"
                && root.TryGetProperty("vote", out JsonElement vote)
                && vote.ValueKind == JsonValueKind.Object
                && TryVote(vote, binding.OpaquePeerId, out string proposalId, out string choice,
                    out _, out _)
                && (proposalId == "map" || proposalId.StartsWith("map:", StringComparison.Ordinal)))
            {
                return DispatchAuthenticatedSharedVote(new(operationId, proposalId,
                    expectedGeneration, binding.OpaquePeerId, CoopVoteDomain.Map, choice), binding);
            }
            return CarrierRejected("unsupported_authenticated_carrier_request");
        }
        catch (JsonException)
        {
            return CarrierRejected("invalid_authenticated_carrier_json");
        }
    }

    internal bool TryCreateClientCarrierRequest(string expectedKind, string instanceId,
        string sessionId, string leaseId, string correlationId, string leaseEpochText, string body,
        out string operationId, out byte[] payload, out string errorCode)
    {
        operationId = string.Empty;
        payload = Array.Empty<byte>();
        errorCode = "invalid_coop_native_client_request";
        if (expectedKind is not ("local_action_request" or "shared_vote_request")
            || body.Length == 0 || body.Length > OpaqueOperationNativeMessage.MaximumPayloadBytes)
        {
            return false;
        }

        try
        {
            using JsonDocument document = JsonDocument.Parse(body,
                new JsonDocumentOptions { MaxDepth = 16, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (!ulong.TryParse(leaseEpochText, out ulong leaseEpoch)
                || !HasClosedEnvelope(root)
                || StringField(root, "kind") != expectedKind
                || StringField(root, "instance_id") != instanceId
                || StringField(root, "session_id") != sessionId
                || StringField(root, "lease_id") != leaseId
                || StringField(root, "correlation_id") != correlationId
                || !NumberField(root, "lease_epoch", leaseEpoch)
                || !TryRequestFields(root, out operationId, out string actorPeer,
                    out ulong expectedGeneration))
            {
                return false;
            }

            CoopHostObservation observation = _host.Observe();
            if (observation.Role != CoopHostRole.Client
                || !string.Equals(observation.LocalPeerId, actorPeer, StringComparison.Ordinal)
                || observation.HostGeneration != expectedGeneration
                || !observation.AllConnectedPeersConverged()
                || !LocalPeerIsConnectedAndConverged(observation, actorPeer))
            {
                errorCode = "coop_native_client_not_admitted";
                return false;
            }

            if (expectedKind == "local_action_request")
            {
                if (!root.TryGetProperty("action", out JsonElement action)
                    || action.ValueKind != JsonValueKind.Object
                    || !TryAction(action, out _, out _, out string? targetPeer)
                    || targetPeer is not null && !string.Equals(targetPeer, actorPeer,
                        StringComparison.Ordinal))
                {
                    return false;
                }
            }
            else if (!root.TryGetProperty("vote", out JsonElement vote)
                || vote.ValueKind != JsonValueKind.Object
                || !TryVote(vote, actorPeer, out string proposalId, out _, out _, out _)
                || !(proposalId == "map" || proposalId.StartsWith("map:", StringComparison.Ordinal)))
            {
                return false;
            }

            payload = Encoding.UTF8.GetBytes(body);
            if (payload.Length == 0 || payload.Length > OpaqueOperationNativeMessage.MaximumPayloadBytes)
            {
                payload = Array.Empty<byte>();
                return false;
            }
            errorCode = string.Empty;
            return true;
        }
        catch (JsonException)
        {
            errorCode = "invalid_coop_native_client_json";
            return false;
        }
        catch
        {
            errorCode = "coop_native_client_observation_unavailable";
            return false;
        }
    }

    private static bool LocalPeerIsConnectedAndConverged(CoopHostObservation observation,
        string peerId)
    {
        foreach (CoopPeerSnapshot peer in observation.Peers)
        {
            if (!peer.IsLocal || !string.Equals(peer.PeerId, peerId, StringComparison.Ordinal))
                continue;
            return peer.Connected && !peer.IsLoading && !peer.IsDivergent && peer.DigestKnown
                && string.Equals(peer.AuthorityId, observation.AuthorityId, StringComparison.Ordinal)
                && string.Equals(peer.StateDigest, observation.HostStateDigest, StringComparison.Ordinal);
        }
        return false;
    }

    internal (int Status, string Response) ClientCarrierReceipt(string instanceId, string sessionId,
        string leaseId, string correlationId, string leaseEpochText, string originalBody,
        OpaqueOperationNativeMessage? hostReply)
    {
        try
        {
            using JsonDocument document = JsonDocument.Parse(originalBody,
                new JsonDocumentOptions { MaxDepth = 16, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (!HasClosedEnvelope(root)
                || !TryRequestFields(root, out string operationId, out _, out _))
            {
                return Error(400, "invalid_coop_native_client_request");
            }
            if (!ulong.TryParse(leaseEpochText, out ulong leaseEpoch)
                || StringField(root, "instance_id") != instanceId
                || StringField(root, "session_id") != sessionId
                || StringField(root, "lease_id") != leaseId
                || StringField(root, "correlation_id") != correlationId
                || !NumberField(root, "lease_epoch", leaseEpoch))
            {
                return Error(400, "invalid_coop_native_client_context");
            }
            CoopHostObservation observation = _host.Observe();
            CoopOutcome outcome = hostReply?.Kind switch
            {
                OpaqueOperationNativeMessageKind.SettledReply => CoopOutcome.Settled,
                OpaqueOperationNativeMessageKind.RejectedReply => CoopOutcome.Rejected,
                _ => CoopOutcome.Unknown
            };
            string? error = hostReply?.Kind switch
            {
                OpaqueOperationNativeMessageKind.RejectedReply => hostReply.RejectionCode,
                OpaqueOperationNativeMessageKind.PendingReply => hostReply.RejectionCode,
                _ => outcome == CoopOutcome.Unknown ? "awaiting_authenticated_host_receipt" : null
            };
            var receipt = new CoopOperationReceipt(operationId, "authenticated_native_carrier",
                outcome, observation.HostGeneration, null, null, observation, error);
            return ReceiptResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch, receipt);
        }
        catch
        {
            return Error(Unavailable, "coop_native_client_observation_unavailable");
        }
    }

    private static CoopOperationReceipt CarrierRejected(string code) => new("carrier_rejected",
        "authenticated_native_carrier", CoopOutcome.Rejected, 0, null, null,
        new CoopHostObservation("unavailable", "peer:local", CoopHostRole.Unknown, "unknown",
            null, 0, "digest:unknown", new[] { new CoopPeerSnapshot("peer:local", true,
                false, 0, "digest:unknown") }, true, null), code);
}
