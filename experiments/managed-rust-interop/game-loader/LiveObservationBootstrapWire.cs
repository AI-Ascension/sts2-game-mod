// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static Dictionary<string, object?> BootstrapResponse(
        RuntimeContext context,
        BootstrapRequest request,
        LiveCardCapturedSnapshot snapshot,
        IReadOnlyList<LiveCardCapturedCard> cards)
    {
        var visible = new List<Dictionary<string, object?>>(cards.Count);
        string? parentEntityId = null;
        foreach (LiveCardCapturedCard card in cards)
        {
            Dictionary<string, object?> entity = VisibleEntity(
                context.InstanceId,
                request.RunId,
                snapshot.Epoch,
                snapshot.StateGeneration,
                snapshot.SnapshotId,
                card);
            if (Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(entity))
                > request.MaxItemBytes)
                continue;
            parentEntityId ??= card.InstanceId;
            visible.Add(entity);
        }
        if (visible.Count == 0)
            throw new InvalidOperationException("all visible entities exceed item bound");
        Dictionary<string, object?> parent = ParentObservation(
            context.InstanceId, request.RunId, snapshot.Epoch, snapshot.StateGeneration,
            snapshot.SnapshotId,
            parentEntityId!);

        return Envelope(
            "bootstrap_response",
            request.Scope,
            request.Selector,
            request.MaxVisibleEntities,
            request.MaxItemBytes,
            request.MaxMessageBytes,
            parent,
            visible,
            null,
            ownerProvenance: true,
            context.CorrelationId);
    }

    private static string LiveBootstrapError(
        RuntimeContext context,
        JsonElement? scope,
        JsonElement? selector,
        string code,
        string field,
        string reason,
        bool retryable)
    {
        var error = new Dictionary<string, object?>
        {
            ["code"] = code,
            ["field"] = field,
            ["reason"] = reason,
            ["retryable"] = retryable
        };
        return JsonSerializer.Serialize(Envelope(
            "error_response",
            scope,
            selector,
            null,
            null,
            null,
            null,
            null,
            error,
            ownerProvenance: false,
            context.CorrelationId));
    }

    private static Dictionary<string, object?> Envelope(
        string kind,
        object? scope,
        object? selector,
        int? maxVisible,
        int? maxItem,
        int? maxMessage,
        object? parent,
        object? visible,
        object? error,
        bool ownerProvenance,
        string correlationId) =>
        new()
        {
            ["protocol_version"] = LiveBootstrapProtocol,
            ["schema_digest"] = LiveBootstrapDigest,
            ["provenance"] = new Dictionary<string, object?>
            {
                ["artifact"] = LiveBootstrapArtifact,
                ["source"] = LiveBootstrapSchemaSource,
                ["generator"] = LiveBootstrapGenerator
            },
            ["correlation_id"] = correlationId,
            ["kind"] = kind,
            ["selector"] = selector,
            ["scope"] = scope,
            ["limits"] = maxVisible is null
                ? null
                : new Dictionary<string, object?>
                {
                    ["max_visible_entities"] = maxVisible.Value,
                    ["max_item_bytes"] = maxItem!.Value,
                    ["max_message_bytes"] = maxMessage!.Value
                },
            ["parent_observation"] = parent,
            ["visible_entities"] = visible,
            ["owner_provenance"] = ownerProvenance
                ? new Dictionary<string, string>
                {
                    ["native_snapshot_owner"] = LiveBootstrapOwnerGameMod,
                    ["content_manifest_owner"] = LiveBootstrapOwnerGameMod,
                    ["instance_fence_owner"] = LiveBootstrapOwnerGateway,
                    ["authority_epoch_owner"] = LiveBootstrapOwnerHarness,
                    ["instance_ref_epoch_owner"] = LiveBootstrapOwnerGameMod,
                    ["transport_lease_epoch_role"] = LiveBootstrapLeaseRole
                }
                : null,
            ["error"] = error
        };

    private static Dictionary<string, object?> ParentObservation(
        string instanceId,
        string runId,
        ulong instanceEpoch,
        ulong stateGeneration,
        string snapshotId,
        string entityId)
    {
        Dictionary<string, object?> instance = InstanceReference(
            instanceId, runId, instanceEpoch, entityId);
        Dictionary<string, object?> snapshot = new()
        {
            ["snapshot_id"] = snapshotId,
            ["instance_ref"] = instance,
            ["state_generation"] = stateGeneration
        };
        return new Dictionary<string, object?>
        {
            ["instance_ref"] = instance,
            ["snapshot_ref"] = snapshot,
            ["state_generation"] = stateGeneration
        };
    }

    private static Dictionary<string, object?> VisibleEntity(
        string instanceId,
        string runId,
        ulong instanceEpoch,
        ulong stateGeneration,
        string snapshotId,
        LiveCardCapturedCard card)
    {
        Dictionary<string, object?> instance = InstanceReference(
            instanceId, runId, instanceEpoch, card.InstanceId);
        Dictionary<string, object?> snapshot = new()
        {
            ["snapshot_id"] = snapshotId,
            ["instance_ref"] = instance,
            ["state_generation"] = stateGeneration
        };
        return new Dictionary<string, object?>
        {
            ["instance_ref"] = instance,
            ["snapshot_ref"] = snapshot,
            ["definition_ref"] = new Dictionary<string, object?>
            {
                ["content_manifest_id"] = card.DefinitionManifest.Value,
                ["entity_kind"] = "card",
                ["namespaced_id"] = card.DefinitionId.Value,
                ["variant"] = card.UpgradeVariant.Status == LiveCardFieldStatus.Available
                    ? card.UpgradeVariant.Value
                    : null
            }
        };
    }

    private static Dictionary<string, object?> InstanceReference(
        string instanceId,
        string runId,
        ulong epoch,
        string entityId) =>
        new()
        {
            ["instance_id"] = instanceId,
            ["run_id"] = runId,
            ["epoch"] = epoch,
            ["entity_kind"] = "card",
            ["entity_id"] = entityId
        };
}
