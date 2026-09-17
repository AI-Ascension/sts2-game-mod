// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// The native listener's fixed game-information producer.  It deliberately accepts only the
/// pinned envelope and never performs a host read from a selector: live callers must use an
/// already-retained snapshot owned by the observation path.
/// </summary>
public static partial class ModEntry
{
    private const string GameInformationProtocol = "game-information-query-v1";
    private const string GameInformationDigest =
        "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";
    private static readonly string[] QueryKinds = ["detail"];
    private static readonly string[] EntityKinds = ["card"];
    private static readonly string[] Levels = ["summary", "standard", "full"];
    private static readonly string[] Fields = ["amount", "cost", "description", "display_name", "flags", "owner", "position", "rarity", "source_id", "tags"];
    private static readonly string[] InvalidatedBy = ["content_change", "epoch_change", "profile_change", "restore", "restart", "run_change"];

    private static (int Status, string Response) ProcessGameInformationQueryWork(
        RuntimeContext context, string body)
    {
        if (!ValidGameInformationContext(context))
            return (400, "{\"error_code\":\"invalid_game_information_context\"}");
        if (body.Length == 0)
            return (RuntimeAccepted, GameInformationCapabilities(context.CorrelationId));
        if (body.Length > 256 * 1024)
            return (413, GameInformationError(context.CorrelationId, null, "result_limit_exceeded",
                "request_too_large"));
        try
        {
            using JsonDocument document = JsonDocument.Parse(body,
                new JsonDocumentOptions { MaxDepth = 16, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object
                || !root.TryGetProperty("protocol_version", out JsonElement version)
                || version.GetString() != GameInformationProtocol
                || !root.TryGetProperty("schema_digest", out JsonElement digest)
                || digest.GetString() != GameInformationDigest
                || !root.TryGetProperty("kind", out JsonElement kind)
                || kind.GetString() != "query_request"
                || !root.TryGetProperty("correlation_id", out JsonElement correlation)
                || correlation.GetString() != context.CorrelationId
                || !root.TryGetProperty("query", out JsonElement query)
                || query.ValueKind != JsonValueKind.Object)
                return (400, GameInformationError(context.CorrelationId, null, "malformed",
                    "invalid_query_envelope"));

            if (!query.TryGetProperty("query_kind", out JsonElement queryKind)
                || queryKind.ValueKind != JsonValueKind.String
                || queryKind.GetString() is not ("list" or "search" or "get" or "detail" or "availability")
                || !query.TryGetProperty("binding", out JsonElement binding)
                || binding.ValueKind != JsonValueKind.Object
                || !binding.TryGetProperty("locale", out JsonElement locale)
                || locale.GetString() != context.Locale
                || !binding.TryGetProperty("mode", out JsonElement mode))
                return (400, GameInformationError(context.CorrelationId, null, "malformed",
                    "invalid_query_binding"));

            // A retained live read is an authority check, never a fresh observation.  The full
            // projection is intentionally unavailable until the content-index owner exposes its
            // immutable index through this adapter.
            if (mode.GetString() == "live" && queryKind.GetString() == "detail")
                return ProcessLiveDetail(context, query, binding);

            return (503, GameInformationError(context.CorrelationId, query, "missing_capability",
                "content_index_adapter_unavailable"));
        }
        catch (JsonException)
        {
            return (400, GameInformationError(context.CorrelationId, null, "malformed",
                "invalid_json"));
        }
        catch (Exception)
        {
            return (503, GameInformationError(context.CorrelationId, null, "source_unavailable",
                "query_source_unavailable"));
        }
    }

    private static (int Status, string Response) ProcessLiveDetail(
        RuntimeContext context, JsonElement query, JsonElement binding)
    {
        if (!binding.TryGetProperty("content_manifest_id", out JsonElement manifest)
            || !binding.TryGetProperty("snapshot_ref", out JsonElement snapshotRef)
            || snapshotRef.ValueKind != JsonValueKind.Object
            || !snapshotRef.TryGetProperty("instance_ref", out JsonElement snapshotInstance)
            || !snapshotInstance.TryGetProperty("epoch", out JsonElement epoch)
            || !epoch.TryGetUInt64(out ulong snapshotEpoch))
            return (400, GameInformationError(context.CorrelationId, query, "malformed",
                "invalid_snapshot_ref"));
        LiveCardCapturedSnapshot snapshot = ReadRetainedLiveCardSnapshot(context.InstanceId,
            manifest.GetString() ?? string.Empty, snapshotEpoch);
        if (!snapshot.Available)
            return (409, GameInformationError(context.CorrelationId, query, "stale_snapshot",
                "retained_snapshot_unavailable"));
        if (!query.TryGetProperty("target", out JsonElement target)
            || !target.TryGetProperty("instance_ref", out JsonElement targetInstance)
            || !targetInstance.TryGetProperty("entity_id", out JsonElement entityId))
            return (400, GameInformationError(context.CorrelationId, query, "malformed",
                "missing_live_target"));
        LiveCardCapturedCard? card = null;
        foreach (LiveCardCapturedCard candidate in snapshot.Cards)
            if (candidate.InstanceId == entityId.GetString()) { card = candidate; break; }
        if (card is null)
            return (404, GameInformationError(context.CorrelationId, query, "not_found",
                "live_card_not_found"));

        object definitionRef = target.GetProperty("definition_ref").Clone();
        object instanceRef = targetInstance.Clone();
        var fields = new List<Dictionary<string, object?>>();
        foreach (JsonElement name in query.GetProperty("fields").EnumerateArray())
            fields.Add(LiveField(name.GetString() ?? string.Empty, card, context.InstanceId));
        var item = new Dictionary<string, object?>
        {
            ["definition_ref"] = definitionRef, ["instance_ref"] = instanceRef, ["fields"] = fields
        };
        var page = new Dictionary<string, object?>
        {
            ["items"] = new[] { item }, ["next_cursor"] = null, ["final_page"] = true,
            ["total_count"] = 1, ["total_count_known"] = true, ["coverage"] = "complete",
            ["cursor_binding"] = null, ["limits"] = query.GetProperty("limits").Clone(),
            ["ordering"] = new Dictionary<string, object?> { ["algorithm"] = "identity_bytes", ["deterministic"] = true, ["direction"] = "ascending", ["key"] = "instance_ref" },
            ["accounting"] = new Dictionary<string, int> { ["item_count"] = 1, ["item_bytes"] = 4096, ["payload_bytes"] = 4096, ["page_bytes"] = 8192, ["text_bytes"] = 4096 }
        };
        return (200, JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = context.CorrelationId,
            ["kind"] = "query_response", ["query"] = query.Clone(), ["capabilities"] = null, ["error"] = null,
            ["result"] = new Dictionary<string, object?> { ["page"] = page, ["read_only"] = true, ["result_generation"] = snapshot.StateGeneration, ["parent_observation"] = query.GetProperty("parent_observation").Clone() }
        }));
    }

    private static Dictionary<string, object?> LiveField(string name, LiveCardCapturedCard card,
        string instanceId)
    {
        object? value = null; string kind = "text"; string? unit = null; string availability = "not_observable";
        if (name == "display_name" && card.Title.Status == LiveCardFieldStatus.Available)
            { value = card.Title.Value; availability = "available"; }
        else if (name == "cost" && card.ResolvedCost.Status == LiveCardFieldStatus.Available)
            { value = card.ResolvedCost.Value; kind = "integer"; unit = "count"; availability = "available"; }
        else if (name == "owner" && card.OwnerId.Status == LiveCardFieldStatus.Available)
            { value = card.OwnerId.Value; availability = "available"; }
        return new Dictionary<string, object?> { ["name"] = name, ["kind"] = kind, ["value"] = value,
            ["unit"] = unit, ["availability"] = availability, ["reason"] = availability == "available" ? null : "field_not_observed",
            ["source"] = new Dictionary<string, string> { ["kind"] = "game_mod", ["ref"] = instanceId } };
    }

    private static bool ValidGameInformationContext(RuntimeContext context) =>
        ContentManifestWireContract.ValidIdentity(context.InstanceId)
        && ContentManifestWireContract.ValidIdentity(context.CorrelationId)
        && ContentManifestWireContract.ValidLocale(context.Locale);

    private static string GameInformationCapabilities(string correlationId) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol,
            ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(),
            ["correlation_id"] = correlationId,
            ["kind"] = "capabilities_response",
            ["query"] = null,
            ["result"] = null,
            ["error"] = null,
            ["capabilities"] = new Dictionary<string, object?>
            {
                ["profile"] = GameInformationProtocol,
                ["query_kinds"] = QueryKinds,
                ["entity_kinds"] = EntityKinds,
                ["projections"] = Levels,
                ["detail_levels"] = Levels,
                ["fields"] = Fields,
                ["limits"] = new Dictionary<string, int> { ["item_bytes"] = 4096, ["page_bytes"] = 65536, ["page_items"] = 32, ["text_bytes"] = 4096 },
                ["max_cursor_bytes"] = 512,
                ["max_message_bytes"] = 262144,
                ["snapshot_policy"] = new Dictionary<string, object?>
                {
                    ["supports_live"] = true, ["lifetime_generations"] = 128,
                    ["max_retained_snapshots"] = 8, ["expiry_behavior"] = "reject_stale_snapshot",
                    ["invalidated_by"] = InvalidatedBy
                }
            }
        });

    private static string GameInformationError(string correlationId, JsonElement? query,
        string code, string reason) => JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol,
            ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(),
            ["correlation_id"] = correlationId,
            ["kind"] = "error_response",
            ["query"] = query,
            ["result"] = null,
            ["capabilities"] = null,
            ["error"] = new Dictionary<string, string> { ["code"] = code, ["reason"] = reason }
        });

    private static Dictionary<string, string> GameInformationProvenance() => new()
    {
        ["artifact"] = "sts2-protocol/game-information-query-v1",
        ["source"] = "schemas/game-information-query-v1.schema.json",
        ["generator"] = "hand-authored"
    };
}
