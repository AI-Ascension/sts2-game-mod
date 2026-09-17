// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string LiveBootstrapProtocol =
        "game-information-live-observation-bootstrap-v1";
    private const string LiveBootstrapSchemaSource =
        "schemas/game-information-live-observation-bootstrap-v1.schema.json";
    private const string LiveBootstrapArtifact =
        "sts2-protocol/game-information-live-observation-bootstrap-v1";
    private const string LiveBootstrapDigest =
        "6041a282ffda8757af4e3eb6ab551e082f136fe53138ab8ac17db9fab52765c2";
    private const string LiveBootstrapGenerator = "hand-authored";
    private const string LiveBootstrapOwnerGameMod = "sts2-game-mod";
    private const string LiveBootstrapOwnerGateway = "sts2-gateway";
    private const string LiveBootstrapOwnerHarness = "sts2-harness";
    private const string LiveBootstrapLeaseRole = "fence_only";
    private const int LiveBootstrapMaxVisible = 64;
    private const int LiveBootstrapMaxItemBytes = 65_536;
    private const int LiveBootstrapMaxMessageBytes = 262_144;

    private static (int Status, string Response) ProcessLiveObservationBootstrapWork(
        RuntimeContext context,
        string body)
    {
        BootstrapRequest? parsedRequest = null;
        try
        {
            using JsonDocument document = JsonDocument.Parse(body, new JsonDocumentOptions
            {
                MaxDepth = 16
            });
            JsonElement root = document.RootElement;
            if (!TryReadBootstrapRequest(root, context, out BootstrapRequest request))
                return (400, LiveBootstrapError(context, null, null, "invalid_request",
                    "request", "bootstrap request is malformed", false));
            parsedRequest = request;

            if (!TryAuthorizeRuntimeV2Context(context, out _))
                return (409, LiveBootstrapError(context, request.Scope, request.Selector,
                    "invalid_binding", "scope", "transport owner is not current", false));

            LiveCardCapturedSnapshot snapshot = request.RequestedInstance is null
                ? ReadLiveCardSnapshot(context.InstanceId, request.ContentManifestId)
                : ReadRetainedLiveCardSnapshot(
                    context.InstanceId, request.ContentManifestId, request.RequestedEpoch);
            if (!snapshot.Available)
                return (503, LiveBootstrapError(context, request.Scope, request.Selector,
                    "not_observable", "parent_observation",
                    snapshot.UnavailableReason ?? "native snapshot identity is unavailable", true));
            if (!TryReadLiveCardBinding(
                    context,
                    request.RunId,
                    request.ContentManifestId,
                    request.AuthorityEpoch,
                    snapshot))
                return (409, LiveBootstrapError(context, request.Scope, request.Selector,
                    "invalid_binding", "scope",
                    "lookup binding is absent or stale for the native snapshot", false));
            if (snapshot.Epoch > RuntimeMaximumInteger)
                return (503, LiveBootstrapError(context, request.Scope, request.Selector,
                    "stale_snapshot", "parent_observation", "snapshot epoch is out of wire bounds",
                    true));
            if (snapshot.StateGeneration > RuntimeMaximumInteger)
                return (503, LiveBootstrapError(context, request.Scope, request.Selector,
                    "stale_snapshot", "parent_observation",
                    "gameplay generation is out of wire bounds", true));

            List<LiveCardCapturedCard> matches = MatchingCards(snapshot, request);
            if (matches.Count == 0)
                return (404, LiveBootstrapError(context, request.Scope, request.Selector,
                    "not_observable", "selector", "selected card is not visible", false));
            if (matches.Count > request.MaxVisibleEntities)
                matches = matches.GetRange(0, request.MaxVisibleEntities);

            Dictionary<string, object?> response = BootstrapResponse(
                context, request, snapshot, matches);
            string serialized = JsonSerializer.Serialize(response);
            if (System.Text.Encoding.UTF8.GetByteCount(serialized) > request.MaxMessageBytes)
                return (413, LiveBootstrapError(context, request.Scope, request.Selector,
                    "invalid_bounds", "limits", "serialized bootstrap exceeds max_message_bytes",
                    false));
            return (200, serialized);
        }
        catch (JsonException)
        {
            return (400, LiveBootstrapError(context, null, null, "invalid_request",
                "request", "bootstrap request is malformed", false));
        }
        catch (Exception)
        {
            return (503, LiveBootstrapError(context, parsedRequest?.Scope, parsedRequest?.Selector,
                "unavailable",
                "parent_observation", "native snapshot source unavailable", true));
        }
    }

    private static bool TryReadBootstrapRequest(
        JsonElement root,
        RuntimeContext context,
        out BootstrapRequest request)
    {
        request = default;
        if (!ExactFields(root, "correlation_id", "error", "kind", "limits",
                "owner_provenance", "parent_observation", "protocol_version", "provenance",
                "schema_digest", "scope", "selector", "visible_entities")
            || StringField(root, "protocol_version") != LiveBootstrapProtocol
            || StringField(root, "schema_digest") != LiveBootstrapDigest
            || StringField(root, "kind") != "bootstrap_request"
            || StringField(root, "correlation_id") != context.CorrelationId
            || root.GetProperty("error").ValueKind != JsonValueKind.Null
            || root.GetProperty("owner_provenance").ValueKind != JsonValueKind.Null
            || root.GetProperty("parent_observation").ValueKind != JsonValueKind.Null
            || root.GetProperty("visible_entities").ValueKind != JsonValueKind.Null)
            return false;

        JsonElement provenance = root.GetProperty("provenance");
        if (!ExactFields(provenance, "artifact", "source", "generator")
            || StringField(provenance, "artifact") != LiveBootstrapArtifact
            || StringField(provenance, "source") != LiveBootstrapSchemaSource
            || StringField(provenance, "generator") != LiveBootstrapGenerator)
            return false;

        JsonElement scope = root.GetProperty("scope");
        if (!ExactFields(scope, "authority_epoch", "content_manifest_id", "instance_id",
                "locale", "run_id"))
            return false;
        string contentManifestId = StringField(scope, "content_manifest_id")!;
        string instanceId = StringField(scope, "instance_id")!;
        string locale = StringField(scope, "locale")!;
        string runId = StringField(scope, "run_id")!;
        if (!RuntimeIdentity(contentManifestId) || instanceId != context.InstanceId
            || !RuntimeIdentity(runId) || locale != context.Locale
            || !BoundedInteger(scope.GetProperty("authority_epoch"), out ulong authorityEpoch)
            || authorityEpoch == 0)
            return false;

        JsonElement limits = root.GetProperty("limits");
        if (!ExactFields(limits, "max_item_bytes", "max_message_bytes", "max_visible_entities")
            || !BoundedInteger(limits.GetProperty("max_item_bytes"), out ulong maxItemBytes)
            || !BoundedInteger(limits.GetProperty("max_message_bytes"), out ulong maxMessageBytes)
            || !BoundedInteger(limits.GetProperty("max_visible_entities"),
                out ulong maxVisibleEntities)
            || maxItemBytes == 0 || maxItemBytes > LiveBootstrapMaxItemBytes
            || maxMessageBytes == 0 || maxMessageBytes > LiveBootstrapMaxMessageBytes
            || maxVisibleEntities == 0 || maxVisibleEntities > LiveBootstrapMaxVisible)
            return false;

        JsonElement selector = root.GetProperty("selector");
        if (!ExactFields(selector, "definition_ref", "instance_ref")
            || selector.GetProperty("definition_ref").ValueKind != JsonValueKind.Object)
            return false;
        JsonElement definition = selector.GetProperty("definition_ref");
        if (!ExactFields(definition, "content_manifest_id", "entity_kind", "namespaced_id",
                "variant")
            || StringField(definition, "content_manifest_id") != contentManifestId
            || StringField(definition, "entity_kind") != "card"
            || !RuntimeIdentity(StringField(definition, "namespaced_id")!)
            || (definition.GetProperty("variant").ValueKind != JsonValueKind.Null
                && !RuntimeIdentity(StringField(definition, "variant")!)))
            return false;
        string? definitionVariant = definition.GetProperty("variant").ValueKind == JsonValueKind.Null
            ? null
            : StringField(definition, "variant");

        JsonElement instance = selector.GetProperty("instance_ref");
        string? requestedInstance = null;
        ulong requestedEpoch = 0;
        if (instance.ValueKind != JsonValueKind.Null)
        {
            if (!ExactFields(instance, "entity_id", "entity_kind", "epoch", "instance_id",
                    "run_id")
                || StringField(instance, "instance_id") != context.InstanceId
                || StringField(instance, "run_id") != runId
                || StringField(instance, "entity_kind") != "card"
                || !RuntimeIdentity(StringField(instance, "entity_id")!)
                || !BoundedInteger(instance.GetProperty("epoch"), out requestedEpoch))
                return false;
            requestedInstance = StringField(instance, "entity_id")!;
        }

        request = new BootstrapRequest(
            contentManifestId,
            runId,
            authorityEpoch,
            StringField(definition, "namespaced_id")!,
            definitionVariant,
            requestedInstance,
            requestedEpoch,
            (int)maxVisibleEntities,
            (int)maxItemBytes,
            (int)maxMessageBytes,
            scope,
            selector);
        return true;
    }

    private static List<LiveCardCapturedCard> MatchingCards(
        LiveCardCapturedSnapshot snapshot,
        BootstrapRequest request)
    {
        var matches = new List<LiveCardCapturedCard>();
        foreach (LiveCardCapturedCard card in snapshot.Cards)
        {
            if (card.DefinitionId.Status != LiveCardFieldStatus.Available
                || card.DefinitionManifest.Status != LiveCardFieldStatus.Available
                || card.DefinitionId.Value != request.DefinitionId
                || card.DefinitionManifest.Value != request.ContentManifestId)
                continue;
            if (request.DefinitionVariant is null)
            {
                if (card.UpgradeVariant.Status is not LiveCardFieldStatus.NotObserved
                    and not LiveCardFieldStatus.Available)
                    continue;
            }
            else if (card.UpgradeVariant.Status != LiveCardFieldStatus.Available
                || card.UpgradeVariant.Value != request.DefinitionVariant)
                continue;
            if (request.RequestedInstance is not null
                && (card.InstanceId != request.RequestedInstance
                    || request.RequestedEpoch != snapshot.Epoch))
                continue;
            matches.Add(card);
        }
        return matches;
    }

    private readonly record struct BootstrapRequest(
        string ContentManifestId,
        string RunId,
        ulong AuthorityEpoch,
        string DefinitionId,
        string? DefinitionVariant,
        string? RequestedInstance,
        ulong RequestedEpoch,
        int MaxVisibleEntities,
        int MaxItemBytes,
        int MaxMessageBytes,
        JsonElement Scope,
        JsonElement Selector);
}
