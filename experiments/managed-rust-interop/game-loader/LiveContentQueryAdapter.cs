// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string QueryBindingKey(JsonElement query) => WithoutCursor(query).GetRawText();

    private static JsonElement WithoutCursor(JsonElement query)
    {
        var value = new Dictionary<string, JsonElement>();
        foreach (JsonProperty property in query.EnumerateObject())
            if (property.Name != "cursor")
                value[property.Name] = property.Value.Clone();
        return JsonSerializer.SerializeToElement(value);
    }

    private static bool TryCaptureStaticIndexWithSource(
        RuntimeContext context, out NativeContentIndexCapture capture) =>
        NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndexWithSource(
            context.CorrelationId, context.Locale, out capture);

    private static bool ValidIdentity(JsonElement value) =>
        value.ValueKind == JsonValueKind.String && ValidIdentity(value.GetString()!);

    private static bool ValidIdentity(string value) =>
        !string.IsNullOrEmpty(value) && value.Length <= 128
        && value.All(character => char.IsLetterOrDigit(character)
            || character is '.' or ':' or '/' or '_' or '-');

    private static bool ValidText(JsonElement value) =>
        value.ValueKind == JsonValueKind.String && !string.IsNullOrEmpty(value.GetString())
        && value.GetString()!.Length <= 1024
        && value.GetString()!.All(character => !char.IsControl(character));

    private static bool ValidCursor(string value) =>
        Encoding.UTF8.GetByteCount(value) <= MaxCursorBytes
        && value.Length > 0
        && value.All(character => char.IsLetterOrDigit(character)
            || character is '.' or '_' or '~' or ':' or '/' or '+' or '=' or '-');

    private static bool StringIdentity(JsonElement value, string name) =>
        value.TryGetProperty(name, out JsonElement field) && ValidIdentity(field);

    private static bool StringEquals(JsonElement value, string name, string expected) =>
        value.TryGetProperty(name, out JsonElement field)
        && field.ValueKind == JsonValueKind.String
        && field.GetString() == expected;

    private static bool LimitsAllow(
        JsonElement limits, int itemBytes, int payloadBytes, int pageBytes, int textBytes) =>
        limits.GetProperty("item_bytes").GetInt32() >= itemBytes
        && limits.GetProperty("page_bytes").GetInt32() >= pageBytes
        && limits.GetProperty("page_items").GetInt32() >= 1
        && limits.GetProperty("text_bytes").GetInt32() >= textBytes
        && payloadBytes <= limits.GetProperty("page_bytes").GetInt32();

    private static (int Status, string Response) ProcessLiveDetail(
        RuntimeContext context, JsonElement query, JsonElement binding)
    {
        if (!TryAuthorizeRuntimeV2Context(context, out string authorizationError))
            return Error(context.CorrelationId, query, "denied_scope", authorizationError, RuntimeRejected);
        if (!binding.TryGetProperty("content_manifest_id", out JsonElement manifest)
            || !binding.TryGetProperty("snapshot_ref", out JsonElement snapshotRef)
            || snapshotRef.ValueKind != JsonValueKind.Object
            || !snapshotRef.TryGetProperty("instance_ref", out JsonElement snapshotInstance)
            || !snapshotInstance.TryGetProperty("epoch", out JsonElement epoch)
            || !epoch.TryGetUInt64(out ulong snapshotEpoch))
            return Error(context.CorrelationId, query, "malformed", "invalid_snapshot_ref", 400);
        LiveCardCapturedSnapshot snapshot = ReadRetainedLiveCardSnapshot(context.InstanceId,
            manifest.GetString() ?? string.Empty, snapshotEpoch);
        if (!snapshot.Available)
            return Error(context.CorrelationId, query, "stale_snapshot",
                "retained_snapshot_unavailable", 409);
        if (!SnapshotMatches(snapshotRef, snapshot) || !InstanceMatches(snapshotInstance, snapshot)
            || !query.TryGetProperty("parent_observation", out JsonElement parent)
            || !parent.TryGetProperty("snapshot_ref", out JsonElement parentSnapshot)
            || !SnapshotMatches(parentSnapshot, snapshot)
            || !parent.TryGetProperty("instance_ref", out JsonElement parentInstance)
            || !InstanceMatches(parentInstance, snapshot)
            || !parent.TryGetProperty("state_generation", out JsonElement parentGeneration)
            || !parentGeneration.TryGetUInt64(out ulong parentState)
            || parentState != snapshot.StateGeneration)
            return Error(context.CorrelationId, query, "stale_snapshot", "snapshot_fence_mismatch", 409);
        if (!query.TryGetProperty("target", out JsonElement target)
            || !target.TryGetProperty("instance_ref", out JsonElement targetInstance)
            || !targetInstance.TryGetProperty("entity_id", out JsonElement entityId))
            return Error(context.CorrelationId, query, "malformed", "missing_live_target", 400);
        LiveCardCapturedCard? card = snapshot.Cards.FirstOrDefault(
            candidate => candidate.InstanceId == entityId.GetString());
        if (card is null)
            return Error(context.CorrelationId, query, "unknown_id", "live_card_not_found", 404);
        if (!target.TryGetProperty("definition_ref", out JsonElement definitionRef)
            || !definitionRef.TryGetProperty("content_manifest_id", out JsonElement definitionManifest)
            || definitionManifest.GetString() != snapshot.ContentManifest
            || !definitionRef.TryGetProperty("entity_kind", out JsonElement definitionKind)
            || definitionKind.GetString() != "card")
            return Error(context.CorrelationId, query, "malformed", "definition_fence_mismatch", 400);
        if (!TryReadCurrentLiveCardBinding(context, snapshot.RunId, snapshot.ContentManifest, snapshot))
            return Error(context.CorrelationId, query, "stale_snapshot", "lookup_binding_mismatch", 409);

        var fields = new List<Dictionary<string, object?>>();
        foreach (JsonElement name in query.GetProperty("fields").EnumerateArray())
            fields.Add(LiveField(name.GetString()!, card, context.InstanceId));
        var item = new Dictionary<string, object?>
        {
            ["definition_ref"] = definitionRef.Clone(),
            ["instance_ref"] = targetInstance.Clone(),
            ["fields"] = fields
        };
        return BuildLiveResponse(context, query, item, snapshot);
    }

    private static (int Status, string Response) BuildLiveResponse(
        RuntimeContext context, JsonElement query, Dictionary<string, object?> item,
        LiveCardCapturedSnapshot snapshot)
    {
        var items = new[] { item };
        JsonElement limits = query.GetProperty("limits");
        int itemBytes = Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(item));
        int payloadBytes = Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(items));
        int textBytes = LiveTextBytes((IEnumerable<Dictionary<string, object?>>)item["fields"]!);
        var page = new Dictionary<string, object?>
        {
            ["items"] = items, ["next_cursor"] = null, ["final_page"] = true,
            ["total_count"] = 1, ["total_count_known"] = true, ["coverage"] = "complete",
            ["cursor_binding"] = null, ["limits"] = limits.Clone(),
            ["ordering"] = new Dictionary<string, object?>
            {
                ["algorithm"] = "identity_bytes", ["deterministic"] = true,
                ["direction"] = "ascending", ["key"] = "instance_ref"
            }
        };
        int pageBytes = Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(page));
        if (!LimitsAllow(limits, itemBytes, payloadBytes, pageBytes, textBytes))
            return Error(context.CorrelationId, query, "result_limit_exceeded",
                "requested_page_limit_exceeded", 413);
        page["accounting"] = new Dictionary<string, int>
        {
            ["item_count"] = 1, ["item_bytes"] = itemBytes,
            ["payload_bytes"] = payloadBytes, ["page_bytes"] = pageBytes, ["text_bytes"] = textBytes
        };
        return (200, JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = context.CorrelationId,
            ["kind"] = "query_response", ["query"] = query.Clone(), ["capabilities"] = null,
            ["error"] = null, ["result"] = new Dictionary<string, object?>
            {
                ["page"] = page, ["read_only"] = true,
                ["result_generation"] = snapshot.StateGeneration,
                ["parent_observation"] = query.GetProperty("parent_observation").Clone()
            }
        }));
    }

    private static int LiveTextBytes(IEnumerable<Dictionary<string, object?>> fields)
    {
        int total = 0;
        foreach (Dictionary<string, object?> field in fields)
            if (field["value"] is string text)
                total = checked(total + Encoding.UTF8.GetByteCount(text));
        return total;
    }

    private static Dictionary<string, object?> LiveField(
        string name, LiveCardCapturedCard card, string instanceId)
    {
        object? value = null;
        string kind = "text";
        string? unit = null;
        string availability = "not_observable";
        string? reason = "field_not_observed";
        if (name == "display_name" && card.Title.Status == LiveCardFieldStatus.Available)
            { value = card.Title.Value; availability = "available"; reason = null; }
        else if (name == "cost" && card.ResolvedCost.Status == LiveCardFieldStatus.Available)
            { value = card.ResolvedCost.Value; kind = "integer"; unit = "count"; availability = "available"; reason = null; }
        else if (name == "owner" && card.OwnerId.Status == LiveCardFieldStatus.Available)
            { value = card.OwnerId.Value; availability = "available"; reason = null; }
        return new Dictionary<string, object?>
        {
            ["name"] = name, ["kind"] = kind, ["value"] = value, ["unit"] = unit,
            ["availability"] = availability, ["reason"] = reason,
            ["source"] = new Dictionary<string, string> { ["kind"] = "game_mod", ["ref"] = instanceId }
        };
    }

    private static bool InstanceMatches(JsonElement value, LiveCardCapturedSnapshot snapshot) =>
        value.ValueKind == JsonValueKind.Object
        && StringEquals(value, "instance_id", snapshot.InstanceId)
        && StringEquals(value, "run_id", snapshot.RunId)
        && value.TryGetProperty("epoch", out JsonElement epoch)
        && epoch.TryGetUInt64(out ulong valueEpoch) && valueEpoch == snapshot.Epoch
        && StringEquals(value, "entity_kind", "card");

    private static bool SnapshotMatches(JsonElement value, LiveCardCapturedSnapshot snapshot) =>
        value.ValueKind == JsonValueKind.Object
        && StringEquals(value, "snapshot_id", snapshot.SnapshotId)
        && value.TryGetProperty("state_generation", out JsonElement generation)
        && generation.TryGetUInt64(out ulong valueGeneration)
        && valueGeneration == snapshot.StateGeneration
        && value.TryGetProperty("instance_ref", out JsonElement instance)
        && InstanceMatches(instance, snapshot);

    private static bool TryReadCurrentLiveCardBinding(
        RuntimeContext context, string runId, string contentManifestId,
        LiveCardCapturedSnapshot snapshot)
    {
        LiveCardBindingAssociation? binding = _liveCardBinding;
        return binding is not null
            && binding.InstanceId == context.InstanceId && binding.CallerId == context.CallerId
            && binding.SessionId == context.SessionId && binding.LeaseId == context.LeaseId
            && binding.LeaseEpoch == ParseEpoch(context.LeaseEpoch)
            && binding.RunId == runId && binding.ContentManifestId == contentManifestId
            && binding.NativeRunId == snapshot.RunId
            && binding.SourceIncarnation == snapshot.SourceIncarnation
            && binding.SourceEpoch <= snapshot.Epoch
            && binding.SourceGeneration <= snapshot.StateGeneration;
    }

    private static bool ValidGameInformationContext(RuntimeContext context) =>
        ContentManifestWireContract.ValidIdentity(context.InstanceId)
        && ContentManifestWireContract.ValidIdentity(context.CorrelationId)
        && ContentManifestWireContract.ValidLocale(context.Locale);

}
