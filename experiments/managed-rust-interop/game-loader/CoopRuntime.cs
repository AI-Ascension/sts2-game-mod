// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Serialization;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Managed composition seam for the host-backed coop-native-v1 wire. The listener and native
/// callback route remain outside this class; callers invoke <see cref="Handle"/> only from the
/// existing game-thread dispatch queue. This class never accepts a caller-supplied effect.
/// </summary>
internal sealed partial class CoopNativeRuntime
{
    private const int Accepted = 200;
    private const int Rejected = 409;
    private const int Unavailable = 503;
    private const int MaxRequestBytes = 64 * 1024;
    private const int MaxResponseBytes = 128 * 1024;
    private const string ProtocolVersion = "coop-native-v1";
    private const string Artifact = "sts2-protocol/coop-native-v1";
    private const string SchemaSource = "schemas/coop-native-v1.schema.json";
    private const string SchemaDigest = "9c24c6d0dbcc3b52c2c504b2a60c9faf9d02f902c00cf16712a2a810c2358391";
    private static readonly JsonSerializerOptions JsonOptions = new()
    {
        UnmappedMemberHandling = JsonUnmappedMemberHandling.Disallow,
        WriteIndented = false
    };

    private readonly CoopHostRuntime _host;

    internal CoopNativeRuntime(ICoopNativeHostPort port, Func<bool>? canDispatch = null)
    {
        _host = new CoopHostRuntime(port, canDispatch);
    }

    internal bool HasPendingMutation => _host.HasPendingMutation;

    internal (int Status, string Response) Handle(
        string instanceId,
        string sessionId,
        string leaseId,
        string correlationId,
        string leaseEpochText,
        string expectedKind,
        string body)
    {
        if (body.Length > MaxRequestBytes
            || !ulong.TryParse(leaseEpochText, out ulong leaseEpoch)
            || leaseEpoch > RuntimeV3GameplayContract.MaxGeneration)
        {
            return Error(400, "invalid_coop_native_context");
        }

        // Observation is the one read route whose gateway request has no JSON body. Its
        // identity and lease fields arrive in the authenticated loopback headers; the producer
        // supplies the closed observation envelope below after taking a fresh game-thread
        // snapshot. Mutations still require the caller-supplied closed envelope.
        if (expectedKind == "observation" && body.Length == 0)
        {
            return Observe(instanceId, sessionId, leaseId, correlationId, leaseEpoch);
        }

        JsonDocument document;
        try
        {
            document = JsonDocument.Parse(body, new JsonDocumentOptions { MaxDepth = 16 });
        }
        catch (JsonException)
        {
            return Error(400, "invalid_coop_native_json");
        }

        using (document)
        {
            JsonElement root = document.RootElement;
            if (!HasClosedEnvelope(root)
                || StringField(root, "protocol_version") != ProtocolVersion
                || StringField(root, "schema_digest") != SchemaDigest
                || StringField(root, "correlation_id") != correlationId
                || StringField(root, "instance_id") != instanceId
                || StringField(root, "session_id") != sessionId
                || StringField(root, "lease_id") != leaseId
                || !NumberField(root, "lease_epoch", leaseEpoch)
                || StringField(root, "kind") != expectedKind)
            {
                return Error(400, "invalid_coop_native_envelope");
            }

            return expectedKind switch
            {
                "observation" => Observe(instanceId, sessionId, leaseId, correlationId, leaseEpoch),
                "local_action_request" => LocalAction(root, instanceId, sessionId, leaseId, correlationId, leaseEpoch),
                "shared_vote_request" => SharedVote(root, instanceId, sessionId, leaseId, correlationId, leaseEpoch),
                "legal_catalog_request" => LegalCatalog(root, instanceId, sessionId, leaseId, correlationId, leaseEpoch),
                "rejoin_request" => Rejoin(root, instanceId, sessionId, leaseId, correlationId, leaseEpoch),
                "recovery_response" => Reconcile(root, instanceId, sessionId, leaseId, correlationId, leaseEpoch),
                _ => Error(400, "unsupported_coop_native_kind")
            };
        }
    }

    private (int Status, string Response) Observe(
        string instanceId, string sessionId, string leaseId, string correlationId, ulong leaseEpoch)
    {
        try
        {
            return ObservationResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch, _host.Observe());
        }
        catch (Exception)
        {
            return Error(Unavailable, "coop_native_observation_unavailable");
        }
    }

    private (int Status, string Response) LocalAction(
        JsonElement root, string instanceId, string sessionId, string leaseId,
        string correlationId, ulong leaseEpoch)
    {
        if (!TryRequestFields(root, out string operationId, out string actorPeer, out ulong expectedGeneration)
            || !root.TryGetProperty("action", out JsonElement action)
            || action.ValueKind != JsonValueKind.Object
            || !TryAction(action, out string actionKind, out string actionId, out string? targetPeer))
        {
            return Error(400, "invalid_coop_native_action");
        }
        CoopOperationReceipt receipt = _host.DispatchLocalAction(new(
            operationId, expectedGeneration, actorPeer, actionKind, actionId, targetPeer));
        return ReceiptResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch, receipt);
    }

    private (int Status, string Response) SharedVote(
        JsonElement root, string instanceId, string sessionId, string leaseId,
        string correlationId, ulong leaseEpoch)
    {
        if (!TryRequestFields(root, out string operationId, out string actorPeer, out ulong expectedGeneration)
            || !root.TryGetProperty("vote", out JsonElement vote)
            || vote.ValueKind != JsonValueKind.Object
            || !TryVote(vote, actorPeer, out string proposalId, out string choice,
                out string voterPeer, out CoopVoteDomain domain))
        {
            return Error(400, "invalid_coop_native_vote");
        }
        CoopOperationReceipt receipt = _host.SubmitSharedVote(new(
            operationId, proposalId, expectedGeneration, voterPeer, domain, choice));
        return ReceiptResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch, receipt);
    }

    private (int Status, string Response) LegalCatalog(
        JsonElement root, string instanceId, string sessionId, string leaseId,
        string correlationId, ulong leaseEpoch)
    {
        if (!TryCatalogFields(root, out string actorPeer, out ulong expectedGeneration))
        {
            return Error(400, "invalid_coop_native_legal_catalog");
        }

        if (!_host.TryLegalCatalog(expectedGeneration, actorPeer,
                out CoopNativeLegalCatalog? catalog,
                out CoopHostObservation? observation,
                out string errorCode)
            || catalog is null
            || observation is null)
        {
            return Error(Rejected, errorCode);
        }

        return CatalogResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            catalog!, observation!);
    }

    private (int Status, string Response) Rejoin(
        JsonElement root, string instanceId, string sessionId, string leaseId,
        string correlationId, ulong leaseEpoch)
    {
        if (!TryRequestFields(root, out string operationId, out string actorPeer, out _)
            || !root.TryGetProperty("recovery", out JsonElement recovery)
            || recovery.ValueKind != JsonValueKind.Object
            || StringField(recovery, "kind") != "rejoin"
            || !TryNumber(recovery, "rejoin_epoch", out ulong rejoinEpoch))
        {
            return Error(400, "invalid_coop_native_rejoin");
        }
        CoopOperationReceipt receipt = _host.Rejoin(operationId, actorPeer, rejoinEpoch);
        return RecoveryResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch, receipt, "rejoin");
    }

    private (int Status, string Response) Reconcile(
        JsonElement root, string instanceId, string sessionId, string leaseId,
        string correlationId, ulong leaseEpoch)
    {
        string? operationId = NullableStringField(root, "operation_id");
        JsonElement recovery = root.TryGetProperty("recovery", out JsonElement value) ? value : default;
        if (operationId is null || recovery.ValueKind != JsonValueKind.Object
            || StringField(recovery, "kind") != "reconcile"
            || !TryNumber(recovery, "rejoin_epoch", out _)
            || !_host.Reconcile(operationId, out CoopOperationReceipt? receipt)
            || receipt is null)
        {
            return Error(409, "coop_native_reconcile_unknown_operation");
        }
        return RecoveryResponse(instanceId, sessionId, leaseId, correlationId, leaseEpoch,
            receipt, "reconcile");
    }

}
