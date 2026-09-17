// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Runtime.InteropServices;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Fixed game-information producer. Static reads consume one canonical content-index capture;
/// live reads consume the already-retained observation owned by the live source.
/// </summary>
public static partial class ModEntry
{
    private const string GameInformationProtocol = "game-information-query-v1";
    private const string GameInformationDigest =
        "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";
    private const int MaxMessageBytes = 262 * 1024;
    private const int MaxCursorBytes = 512;
    private const int MaxPageItems = 128;
    private const int MaxItemBytes = 262144;
    private const int MaxPageBytes = 262144;
    private const int MaxTextBytes = 65536;
    private static readonly string[] QueryKinds = ["availability", "detail", "get", "list", "search"];
    private static readonly string[] EntityKinds = ["card"];
    private static readonly string[] Levels = ["summary", "standard", "full"];
    private static readonly string[] Fields =
        ["cost", "description", "display_name", "owner", "rarity", "source_id", "tags"];
    private static readonly string[] InvalidatedBy =
        ["content_change", "epoch_change", "profile_change", "restore", "restart", "run_change"];
    private static readonly object CursorGate = new();
    private static readonly JsonSerializerOptions NativeJsonOptions =
        new() { PropertyNameCaseInsensitive = true };
    private static readonly Dictionary<string, StaticCursor> StaticCursors = new(StringComparer.Ordinal);
    private static ulong _nextStaticCursor;

    private sealed record StaticCursor(
        string Binding,
        NativeContentIndexSnapshot Snapshot,
        IReadOnlyList<NativeContentIndexDefinition> Entries,
        int Offset);

    [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
    private delegate int NativeContentIndexQuery(
        nint input, nuint inputLength, nint query, nuint queryLength,
        nint output, nuint outputCapacity, out nuint outputLength);

    private sealed record NativeCoreItem(
        string entity_kind,
        string namespaced_id,
        string? display_name,
        string[] aliases,
        string? rendered_description,
        string? character_or_pool,
        string? rarity,
        string unlock_state);

    private sealed record NativeCorePage(
        string manifest_id,
        NativeCoreItem[] items,
        int total_count,
        bool final_page,
        string? next_cursor);

    private static (int Status, string Response) ProcessGameInformationQueryWork(
        RuntimeContext context, string body)
    {
        if (!ValidGameInformationContext(context))
            return (400, "{\"error_code\":\"invalid_game_information_context\"}");
        if (body.Length == 0)
            return (RuntimeAccepted, GameInformationCapabilities(context.CorrelationId));
        if (Encoding.UTF8.GetByteCount(body) > MaxMessageBytes)
            return Error(context.CorrelationId, null, "result_limit_exceeded", "request_too_large", 413);
        try
        {
            if (HasDuplicateJsonKeys(Encoding.UTF8.GetBytes(body)))
                return Error(context.CorrelationId, null, "malformed", "duplicate_key", 400);
            using JsonDocument document = JsonDocument.Parse(
                body, new JsonDocumentOptions { MaxDepth = 24, AllowTrailingCommas = false });
            if (!ValidEnvelope(document.RootElement, context, out JsonElement query, out string envelopeReason))
                return Error(context.CorrelationId, null, "malformed", envelopeReason, 400);
            if (!ValidQuery(query, context, out JsonElement binding, out string queryReason))
                return Error(context.CorrelationId, query,
                    queryReason == "unsupported_field" ? "unsupported_field" : "malformed",
                    queryReason, 400);

            string mode = binding.GetProperty("mode").GetString()!;
            if (mode == "live")
            {
                if (query.GetProperty("query_kind").GetString() != "detail")
                    return Error(context.CorrelationId, query, "missing_capability",
                        "live_query_kind_unavailable", 503);
                return ProcessLiveDetail(context, query, binding);
            }
            return ProcessStatic(context, query, binding);
        }
        catch (JsonException)
        {
            return Error(context.CorrelationId, null, "malformed", "invalid_json", 400);
        }
        catch (Exception)
        {
            return Error(context.CorrelationId, null, "missing_capability",
                "query_source_unavailable", 503);
        }
    }

    private static bool ValidEnvelope(
        JsonElement root,
        RuntimeContext context,
        out JsonElement query,
        out string reason)
    {
        query = default;
        reason = "invalid_query_envelope";
        if (root.ValueKind != JsonValueKind.Object
            || !StringEquals(root, "protocol_version", GameInformationProtocol)
            || !StringEquals(root, "schema_digest", GameInformationDigest)
            || !StringEquals(root, "kind", "query_request")
            || !StringEquals(root, "correlation_id", context.CorrelationId)
            || !root.TryGetProperty("query", out query)
            || query.ValueKind != JsonValueKind.Object
            || !root.TryGetProperty("result", out JsonElement result)
            || result.ValueKind != JsonValueKind.Null
            || !root.TryGetProperty("capabilities", out JsonElement capabilities)
            || capabilities.ValueKind != JsonValueKind.Null
            || !root.TryGetProperty("error", out JsonElement error)
            || error.ValueKind != JsonValueKind.Null)
            return false;
        return true;
    }

    private static bool ValidQuery(
        JsonElement query,
        RuntimeContext context,
        out JsonElement binding,
        out string reason)
    {
        binding = default;
        reason = "invalid_query_binding";
        if (!query.TryGetProperty("query_kind", out JsonElement queryKind)
            || queryKind.ValueKind != JsonValueKind.String
            || queryKind.GetString() is not ("list" or "search" or "get" or "detail" or "availability")
            || !query.TryGetProperty("entity_kind", out JsonElement entityKind)
            || entityKind.ValueKind != JsonValueKind.String
            || entityKind.GetString() != "card"
            || !query.TryGetProperty("target", out JsonElement target)
            || !ValidTarget(target)
            || !query.TryGetProperty("filters", out JsonElement filters)
            || !ValidFilters(filters)
            || !query.TryGetProperty("projection", out JsonElement projection)
            || projection.GetString() is not ("summary" or "standard" or "full")
            || !query.TryGetProperty("detail_level", out JsonElement detailLevel)
            || detailLevel.GetString() is not ("summary" or "standard" or "full")
            || !query.TryGetProperty("fields", out JsonElement fields)
            || !ValidFields(fields, out reason)
            || !query.TryGetProperty("limits", out JsonElement limits)
            || !ValidLimits(limits)
            || !query.TryGetProperty("cursor", out JsonElement cursor)
            || (cursor.ValueKind != JsonValueKind.Null
                && (cursor.ValueKind != JsonValueKind.String || !ValidCursor(cursor.GetString()!)))
            || !query.TryGetProperty("parent_observation", out JsonElement parent))
            return false;
        if (!query.TryGetProperty("binding", out binding)
            || binding.ValueKind != JsonValueKind.Object
            || !StringEquals(binding, "locale", context.Locale)
            || !ValidBinding(binding))
            return false;

        string mode = binding.GetProperty("mode").GetString()!;
        string kind = queryKind.GetString()!;
        bool hasTarget = target.GetProperty("definition_ref").ValueKind != JsonValueKind.Null
            || target.GetProperty("instance_ref").ValueKind != JsonValueKind.Null;
        if (mode == "static")
        {
            if (binding.GetProperty("instance_ref").ValueKind != JsonValueKind.Null
                || binding.GetProperty("snapshot_ref").ValueKind != JsonValueKind.Null
                || parent.ValueKind != JsonValueKind.Null)
            {
                reason = "static_binding_fence";
                return false;
            }
            if ((kind is "get" or "detail") != hasTarget)
            {
                reason = "target_required";
                return false;
            }
            if (target.GetProperty("instance_ref").ValueKind != JsonValueKind.Null)
            {
                reason = "static_instance_target";
                return false;
            }
            if (filters.GetProperty("instance_ids").GetArrayLength() != 0)
            {
                reason = "unsupported_filter";
                return false;
            }
        }
        else
        {
            if (binding.GetProperty("instance_ref").ValueKind != JsonValueKind.Object
                || binding.GetProperty("snapshot_ref").ValueKind != JsonValueKind.Object
                || parent.ValueKind != JsonValueKind.Object)
            {
                reason = "live_binding_fence";
                return false;
            }
            if (kind != "detail" || !hasTarget
                || target.GetProperty("instance_ref").ValueKind != JsonValueKind.Object)
            {
                reason = "live_detail_target";
                return false;
            }
        }
        return true;
    }

    private static bool ValidBinding(JsonElement binding) =>
        (StringEquals(binding, "mode", "static") || StringEquals(binding, "mode", "live"))
        && StringIdentity(binding, "content_manifest_id")
        && StringIdentity(binding, "visibility_scope")
        && binding.TryGetProperty("instance_ref", out _)
        && binding.TryGetProperty("snapshot_ref", out _);

    private static bool ValidTarget(JsonElement target) =>
        target.ValueKind == JsonValueKind.Object
        && target.TryGetProperty("definition_ref", out JsonElement definition)
        && target.TryGetProperty("instance_ref", out JsonElement instance)
        && (definition.ValueKind == JsonValueKind.Null || ValidDefinitionRef(definition))
        && (instance.ValueKind == JsonValueKind.Null || ValidInstanceRef(instance));

    private static bool ValidDefinitionRef(JsonElement value) =>
        value.ValueKind == JsonValueKind.Object
        && StringIdentity(value, "content_manifest_id")
        && StringEquals(value, "entity_kind", "card")
        && StringIdentity(value, "namespaced_id")
        && value.TryGetProperty("variant", out JsonElement variant)
        && (variant.ValueKind == JsonValueKind.Null || ValidIdentity(variant));

    private static bool ValidInstanceRef(JsonElement value) =>
        value.ValueKind == JsonValueKind.Object
        && StringIdentity(value, "instance_id")
        && StringIdentity(value, "run_id")
        && value.TryGetProperty("epoch", out JsonElement epoch)
        && epoch.TryGetUInt64(out _)
        && StringEquals(value, "entity_kind", "card")
        && StringIdentity(value, "entity_id");

    private static bool ValidFilters(JsonElement filters)
    {
        if (filters.ValueKind != JsonValueKind.Object
            || !filters.TryGetProperty("display_name", out JsonElement display)
            || (display.ValueKind != JsonValueKind.Null && !ValidText(display))
            || !ValidIdentityArray(filters, "namespaced_ids")
            || !filters.TryGetProperty("definition_refs", out JsonElement refs)
            || refs.ValueKind != JsonValueKind.Array
            || refs.GetArrayLength() > 64
            || refs.EnumerateArray().Any(value => !ValidDefinitionRef(value))
            || !ValidIdentityArray(filters, "instance_ids"))
            return false;
        return true;
    }

    private static bool ValidIdentityArray(JsonElement parent, string name)
    {
        if (!parent.TryGetProperty(name, out JsonElement values)
            || values.ValueKind != JsonValueKind.Array
            || values.GetArrayLength() > 64)
            return false;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement value in values.EnumerateArray())
            if (!ValidIdentity(value) || !seen.Add(value.GetString()!))
                return false;
        return true;
    }

    private static bool ValidFields(JsonElement fields, out string reason)
    {
        reason = "invalid_fields";
        if (fields.ValueKind != JsonValueKind.Array || fields.GetArrayLength() > 32)
            return false;
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonElement field in fields.EnumerateArray())
        {
            if (field.ValueKind != JsonValueKind.String || !seen.Add(field.GetString()!))
                return false;
            if (field.GetString() is not ("cost" or "description" or "display_name"
                or "owner" or "rarity" or "source_id" or "tags"))
            {
                reason = "unsupported_field";
                return false;
            }
        }
        return true;
    }

    private static bool ValidLimits(JsonElement limits) =>
        limits.ValueKind == JsonValueKind.Object
        && limits.TryGetProperty("item_bytes", out JsonElement item)
        && item.TryGetInt32(out int itemValue) && itemValue is >= 1 and <= MaxItemBytes
        && limits.TryGetProperty("page_bytes", out JsonElement page)
        && page.TryGetInt32(out int pageValue) && pageValue is >= 1 and <= MaxPageBytes
        && limits.TryGetProperty("page_items", out JsonElement count)
        && count.TryGetInt32(out int countValue) && countValue is >= 1 and <= MaxPageItems
        && limits.TryGetProperty("text_bytes", out JsonElement text)
        && text.TryGetInt32(out int textValue) && textValue is >= 1 and <= MaxTextBytes;

    private static (int Status, string Response) ProcessStatic(
        RuntimeContext context, JsonElement query, JsonElement binding)
    {
        string requestedManifest = binding.GetProperty("content_manifest_id").GetString()!;
        string scope = binding.GetProperty("visibility_scope").GetString()!;
        if (scope != "public" && scope != "reference")
            return Error(context.CorrelationId, query, "denied_scope", "unsupported_visibility_scope", 403);

        string bindingKey = QueryBindingKey(query);
        string? cursor = query.GetProperty("cursor").ValueKind == JsonValueKind.Null
            ? null : query.GetProperty("cursor").GetString();
        if (_nativeLibrary != 0
            && TryNativeStaticQuery(context, query, binding, out (int Status, string Response) nativeResponse))
            return nativeResponse;
        NativeContentIndexSnapshot snapshot;
        IReadOnlyList<NativeContentIndexDefinition> entries;
        int offset;
        if (cursor is null)
        {
            if (!TryCaptureStaticIndex(context, out snapshot)
                || snapshot.ManifestId != requestedManifest
                || snapshot.Locale != context.Locale)
                return Error(context.CorrelationId, query, "missing_capability",
                    "content_index_unavailable", 503);
            if (!TrySelectStaticEntries(query, snapshot, scope, out entries, out string selectionError))
                return Error(context.CorrelationId, query,
                    selectionError is "unknown_id" or "denied_scope" ? selectionError : "malformed",
                    selectionError, selectionError == "denied_scope" ? 403 : 400);
            offset = 0;
        }
        else
        {
            lock (CursorGate)
            {
                if (!StaticCursors.Remove(cursor, out StaticCursor? state)
                    || state.Binding != bindingKey)
                    return Error(context.CorrelationId, query, "stale_cursor",
                        "cursor_binding_mismatch", 409);
                snapshot = state.Snapshot;
                entries = state.Entries;
                offset = state.Offset;
            }
            if (snapshot.ManifestId != requestedManifest || snapshot.Locale != context.Locale)
                return Error(context.CorrelationId, query, "stale_cursor",
                    "cursor_content_mismatch", 409);
        }

        int pageItems = query.GetProperty("limits").GetProperty("page_items").GetInt32();
        int end = Math.Min(entries.Count, checked(offset + pageItems));
        IReadOnlyList<NativeContentIndexDefinition> pageEntries =
            entries.Skip(offset).Take(end - offset).ToArray();
        bool final = end >= entries.Count;
        string? nextCursor = null;
        JsonElement? cursorBinding = null;
        if (!final)
        {
            lock (CursorGate)
            {
                do
                {
                    nextCursor = $"static-cursor:{_nextStaticCursor++}";
                } while (!ValidCursor(nextCursor) || StaticCursors.ContainsKey(nextCursor));
                StaticCursors[nextCursor] = new StaticCursor(bindingKey, snapshot, entries, end);
            }
            cursorBinding = WithoutCursor(query);
        }
        return BuildStaticResponse(context, query, snapshot.ManifestId, pageEntries, entries.Count, final,
            nextCursor, cursorBinding);
    }

    private static bool TryNativeStaticQuery(
        RuntimeContext context,
        JsonElement query,
        JsonElement binding,
        out (int Status, string Response) response)
    {
        response = default;
        try
        {
            nint export = NativeLibrary.GetExport(_nativeLibrary, "sts2_game_mod_content_index_query");
            NativeContentIndexQuery queryFunction =
                Marshal.GetDelegateForFunctionPointer<NativeContentIndexQuery>(export);
            string? cursor = query.GetProperty("cursor").ValueKind == JsonValueKind.Null
                ? null : query.GetProperty("cursor").GetString();
            if (!TryCaptureStaticIndexWithSource(
                    context, out NativeContentIndexCapture capture)
                || capture.Snapshot.Locale != context.Locale
                || capture.Snapshot.ManifestId
                    != binding.GetProperty("content_manifest_id").GetString())
            {
                response = Error(context.CorrelationId, query, cursor is null
                    ? "missing_capability" : "stale_cursor",
                    cursor is null ? "content_index_unavailable" : "cursor_content_mismatch",
                    cursor is null ? 503 : 409);
                return true;
            }
            string operation = query.GetProperty("query_kind").GetString()! switch
            {
                "availability" => "availability",
                "search" => "search",
                "get" => "get",
                "detail" => "detail",
                _ => "list"
            };
            JsonElement filters = query.GetProperty("filters");
            string? literal = filters.GetProperty("display_name").ValueKind == JsonValueKind.Null
                ? null : filters.GetProperty("display_name").GetString();
            string? namespacedId = query.GetProperty("target")
                .GetProperty("definition_ref").ValueKind == JsonValueKind.Object
                ? query.GetProperty("target").GetProperty("definition_ref")
                    .GetProperty("namespaced_id").GetString()
                : null;
            string customQuery = JsonSerializer.Serialize(new
            {
                operation,
                literal,
                entity_kind = "card",
                namespaced_id = namespacedId,
                limit = query.GetProperty("limits").GetProperty("page_items").GetInt32(),
                cursor,
                binding_key = QueryBindingKey(query)
            });
            string snapshotJson = cursor is null ? capture.SourceJson : string.Empty;
            byte[] inputBytes = Encoding.UTF8.GetBytes(snapshotJson);
            byte[] queryBytes = Encoding.UTF8.GetBytes(customQuery);
            byte[] outputBytes = new byte[1024 * 1024];
            GCHandle inputHandle = GCHandle.Alloc(inputBytes, GCHandleType.Pinned);
            GCHandle queryHandle = GCHandle.Alloc(queryBytes, GCHandleType.Pinned);
            GCHandle outputHandle = GCHandle.Alloc(outputBytes, GCHandleType.Pinned);
            try
            {
                int status = queryFunction(
                    inputHandle.AddrOfPinnedObject(), (nuint)inputBytes.Length,
                    queryHandle.AddrOfPinnedObject(), (nuint)queryBytes.Length,
                    outputHandle.AddrOfPinnedObject(), (nuint)outputBytes.Length,
                    out nuint outputLength);
                string nativeBody = Encoding.UTF8.GetString(outputBytes, 0, checked((int)outputLength));
                if (status != 200)
                {
                    response = status == 409
                        ? Error(context.CorrelationId, query, "stale_cursor", "cursor_binding_mismatch", 409)
                        : status == 404
                            ? Error(context.CorrelationId, query, "unknown_id",
                                "static_definition_not_found", 400)
                            : Error(context.CorrelationId, query, "missing_capability",
                                "content_index_query_unavailable", 503);
                    return true;
                }
                NativeCorePage? page = JsonSerializer.Deserialize<NativeCorePage>(
                    nativeBody, NativeJsonOptions);
                if (page is null)
                    return false;
                if (page.manifest_id != capture.Snapshot.ManifestId)
                {
                    response = Error(context.CorrelationId, query,
                        cursor is null ? "missing_capability" : "stale_cursor",
                        cursor is null ? "content_index_query_unavailable" : "cursor_content_mismatch",
                        cursor is null ? 503 : 409);
                    return true;
                }
                NativeContentIndexDefinition[] values = page.items.Select(value =>
                    new NativeContentIndexDefinition(
                        value.entity_kind, value.namespaced_id, value.display_name,
                        value.aliases, value.rendered_description, value.character_or_pool,
                        value.rarity, value.unlock_state, Array.Empty<string>())).ToArray();
                JsonElement? cursorBinding = page.final_page ? null : WithoutCursor(query);
                response = BuildStaticResponse(
                    context, query, page.manifest_id, values, page.total_count,
                    page.final_page, page.next_cursor, cursorBinding);
                return true;
            }
            finally
            {
                outputHandle.Free();
                queryHandle.Free();
                inputHandle.Free();
            }
        }
        catch (Exception)
        {
            response = Error(context.CorrelationId, query, "missing_capability",
                "content_index_query_unavailable", 503);
            return true;
        }
    }

    private static bool TrySelectStaticEntries(
        JsonElement query,
        NativeContentIndexSnapshot snapshot,
        string scope,
        out IReadOnlyList<NativeContentIndexDefinition> entries,
        out string error)
    {
        error = string.Empty;
        string kind = query.GetProperty("query_kind").GetString()!;
        JsonElement filters = query.GetProperty("filters");
        var selected = snapshot.Definitions
            .Where(value => value.EntityKind == "card" && value.UnlockState == "unlocked")
            .Where(value => scope == "reference" || value.UnlockState == "unlocked")
            .OrderBy(value => value.EntityKind, StringComparer.Ordinal)
            .ThenBy(value => value.NamespacedId, StringComparer.Ordinal)
            .ToList();
        string? display = filters.GetProperty("display_name").ValueKind == JsonValueKind.Null
            ? null : filters.GetProperty("display_name").GetString();
        string[] ids = filters.GetProperty("namespaced_ids").EnumerateArray()
            .Select(value => value.GetString()!).ToArray();
        JsonElement[] definitionRefs = filters.GetProperty("definition_refs").EnumerateArray().ToArray();
        if (display is not null)
            selected = selected.Where(value =>
                Fold(value.DisplayName).Contains(Fold(display), StringComparison.Ordinal)
                || value.Aliases.Any(alias => Fold(alias).Contains(Fold(display), StringComparison.Ordinal))
                || (value.RenderedDescription is not null
                    && Fold(value.RenderedDescription).Contains(Fold(display), StringComparison.Ordinal)))
                .ToList();
        if (ids.Length != 0)
            selected = selected.Where(value => ids.Contains(value.NamespacedId, StringComparer.Ordinal)).ToList();
        if (definitionRefs.Length != 0)
            selected = selected.Where(value => definitionRefs.Any(reference =>
                reference.GetProperty("content_manifest_id").GetString() == snapshot.ManifestId
                && reference.GetProperty("entity_kind").GetString() == value.EntityKind
                && reference.GetProperty("namespaced_id").GetString() == value.NamespacedId)).ToList();
        if (kind is "get" or "detail")
        {
            JsonElement target = query.GetProperty("target").GetProperty("definition_ref");
            selected = selected.Where(value =>
                target.GetProperty("content_manifest_id").GetString() == snapshot.ManifestId
                && target.GetProperty("namespaced_id").GetString() == value.NamespacedId).ToList();
            if (selected.Count == 0)
            {
                bool exists = snapshot.Definitions.Any(value =>
                    value.EntityKind == "card"
                    && value.NamespacedId == target.GetProperty("namespaced_id").GetString());
                error = exists ? "denied_scope" : "unknown_id";
                entries = Array.Empty<NativeContentIndexDefinition>();
                return false;
            }
        }
        entries = selected;
        return true;
    }

    private static (int Status, string Response) BuildStaticResponse(
        RuntimeContext context,
        JsonElement query,
        string manifestId,
        IReadOnlyList<NativeContentIndexDefinition> values,
        int total,
        bool final,
        string? nextCursor,
        JsonElement? cursorBinding)
    {
        var items = new List<Dictionary<string, object?>>(values.Count);
        foreach (NativeContentIndexDefinition value in values)
        {
            var fields = new List<Dictionary<string, object?>>();
            foreach (JsonElement field in query.GetProperty("fields").EnumerateArray())
                fields.Add(StaticField(field.GetString()!, value, manifestId));
            items.Add(new Dictionary<string, object?>
            {
                ["definition_ref"] = DefinitionRef(value, manifestId, null),
                ["instance_ref"] = null,
                ["fields"] = fields
            });
        }
        JsonElement limits = query.GetProperty("limits");
        int itemBytes = items.Sum(item => Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(item)));
        int payloadBytes = Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(items));
        int textBytes = StaticTextBytes(items);
        var page = new Dictionary<string, object?>
        {
            ["items"] = items, ["next_cursor"] = nextCursor,
            ["cursor_binding"] = cursorBinding, ["final_page"] = final,
            ["total_count_known"] = true, ["total_count"] = total, ["coverage"] = "complete",
            ["ordering"] = new Dictionary<string, object?>
            {
                ["key"] = "definition_ref", ["direction"] = "ascending",
                ["algorithm"] = "identity_bytes", ["deterministic"] = true
            },
            ["limits"] = limits.Clone()
        };
        int pageBytes = Encoding.UTF8.GetByteCount(JsonSerializer.Serialize(page));
        if (!LimitsAllow(limits, itemBytes, payloadBytes, pageBytes, textBytes))
            return Error(context.CorrelationId, query, "result_limit_exceeded",
                "requested_page_limit_exceeded", 413);
        page["accounting"] = new Dictionary<string, int>
        {
            ["item_count"] = items.Count, ["item_bytes"] = itemBytes,
            ["payload_bytes"] = payloadBytes, ["page_bytes"] = pageBytes, ["text_bytes"] = textBytes
        };
        string response = JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = context.CorrelationId,
            ["kind"] = "query_response", ["query"] = query.Clone(), ["capabilities"] = null,
            ["error"] = null, ["result"] = new Dictionary<string, object?>
            {
                ["page"] = page, ["result_generation"] = null,
                ["parent_observation"] = null, ["read_only"] = true
            }
        });
        return Encoding.UTF8.GetByteCount(response) > MaxMessageBytes
            ? Error(context.CorrelationId, query, "result_limit_exceeded", "response_too_large", 413)
            : (200, response);
    }

    private static Dictionary<string, object?> StaticField(
        string name, NativeContentIndexDefinition value, string sourceRef)
    {
        object? fieldValue = null;
        string kind = "text";
        string? unit = null;
        string availability = "not_observable";
        string? reason = "field_not_observed";
        switch (name)
        {
            case "display_name":
                fieldValue = value.DisplayName;
                availability = value.DisplayName is null ? "not_observable" : "available";
                break;
            case "description":
                fieldValue = value.RenderedDescription;
                availability = value.RenderedDescription is null ? "not_observable" : "available";
                break;
            case "rarity":
                fieldValue = value.Rarity;
                availability = value.Rarity is null ? "not_observable" : "available";
                break;
            case "tags":
                kind = "text_list"; fieldValue = value.Aliases.ToArray();
                availability = "available"; reason = null; break;
            case "source_id":
                fieldValue = value.NamespacedId; availability = "available"; reason = null; break;
            case "cost":
            case "owner":
                break;
        }
        if (availability == "available")
            reason = null;
        return new Dictionary<string, object?>
        {
            ["name"] = name, ["kind"] = kind, ["value"] = fieldValue, ["unit"] = unit,
            ["availability"] = availability,
            ["source"] = new Dictionary<string, object?> { ["kind"] = "content_manifest", ["ref"] = sourceRef },
            ["reason"] = reason
        };
    }

    private static Dictionary<string, object?> DefinitionRef(
        NativeContentIndexDefinition value, string manifestId, string? variant) =>
        new()
        {
            ["content_manifest_id"] = manifestId, ["entity_kind"] = value.EntityKind,
            ["namespaced_id"] = value.NamespacedId, ["variant"] = variant
        };

    private static int StaticTextBytes(IEnumerable<Dictionary<string, object?>> items)
    {
        int total = 0;
        foreach (Dictionary<string, object?> item in items)
        foreach (Dictionary<string, object?> field in (IEnumerable<Dictionary<string, object?>>)item["fields"]!)
        {
            if (field["value"] is string text)
                total = checked(total + Encoding.UTF8.GetByteCount(text));
            else if (field["value"] is string[] texts)
                foreach (string value in texts)
                    total = checked(total + Encoding.UTF8.GetByteCount(value));
        }
        return total;
    }

    private static string QueryBindingKey(JsonElement query) => WithoutCursor(query).GetRawText();

    private static JsonElement WithoutCursor(JsonElement query)
    {
        var value = new Dictionary<string, JsonElement>();
        foreach (JsonProperty property in query.EnumerateObject())
            if (property.Name != "cursor")
                value[property.Name] = property.Value.Clone();
        return JsonSerializer.SerializeToElement(value);
    }

    private static bool TryCaptureStaticIndex(
        RuntimeContext context, out NativeContentIndexSnapshot snapshot) =>
        NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndex(
            context.CorrelationId, context.Locale, out snapshot);

    private static bool TryCaptureStaticIndexWithSource(
        RuntimeContext context, out NativeContentIndexCapture capture) =>
        NativeContentCatalogManifestSource.TryCaptureCanonicalContentIndexWithSource(
            context.CorrelationId, context.Locale, out capture);

    private static string Fold(string? value) =>
        value is null ? string.Empty : value.ToLowerInvariant();

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

    private static string GameInformationCapabilities(string correlationId) =>
        JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = correlationId,
            ["kind"] = "capabilities_response", ["query"] = null, ["result"] = null, ["error"] = null,
            ["capabilities"] = new Dictionary<string, object?>
            {
                ["profile"] = GameInformationProtocol, ["query_kinds"] = QueryKinds,
                ["entity_kinds"] = EntityKinds, ["projections"] = Levels, ["detail_levels"] = Levels,
                ["fields"] = Fields,
                ["limits"] = new Dictionary<string, int>
                {
                    ["item_bytes"] = 4096, ["page_bytes"] = 65536,
                    ["page_items"] = 32, ["text_bytes"] = 4096
                },
                ["max_cursor_bytes"] = MaxCursorBytes, ["max_message_bytes"] = MaxMessageBytes,
                ["snapshot_policy"] = new Dictionary<string, object?>
                {
                    ["supports_live"] = true, ["lifetime_generations"] = 1,
                    ["max_retained_snapshots"] = 1, ["expiry_behavior"] = "reject_stale_snapshot",
                    ["invalidated_by"] = InvalidatedBy
                }
            }
        });

    private static (int Status, string Response) Error(
        string correlationId, JsonElement? query, string code, string reason, int status) =>
        (status, JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = GameInformationProtocol, ["schema_digest"] = GameInformationDigest,
            ["provenance"] = GameInformationProvenance(), ["correlation_id"] = correlationId,
            ["kind"] = "error_response", ["query"] = query, ["result"] = null,
            ["capabilities"] = null,
            ["error"] = new Dictionary<string, object?>
            {
                ["code"] = code, ["field"] = null, ["reason"] = reason,
                ["retryable"] = status is 409 or 503
            }
        }));

    private static Dictionary<string, string> GameInformationProvenance() => new()
    {
        ["artifact"] = "sts2-protocol/game-information-query-v1",
        ["source"] = "schemas/game-information-query-v1.schema.json",
        ["generator"] = "hand-authored"
    };

    private static bool HasDuplicateJsonKeys(byte[] utf8)
    {
        var reader = new Utf8JsonReader(utf8, new JsonReaderOptions { MaxDepth = 24 });
        var members = new Stack<HashSet<string>>();
        while (reader.Read())
        {
            if (reader.TokenType == JsonTokenType.StartObject)
                members.Push(new HashSet<string>(StringComparer.Ordinal));
            else if (reader.TokenType == JsonTokenType.EndObject)
                members.Pop();
            else if (reader.TokenType == JsonTokenType.PropertyName
                && (members.Count == 0 || !members.Peek().Add(reader.GetString() ?? string.Empty)))
                return true;
        }
        return false;
    }
}
