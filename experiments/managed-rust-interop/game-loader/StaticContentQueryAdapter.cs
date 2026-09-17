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
    private static (int Status, string Response) ProcessStatic(
        RuntimeContext context, JsonElement query, JsonElement binding)
    {
        string scope = binding.GetProperty("visibility_scope").GetString()!;
        if (scope != "public" && scope != "reference")
            return Error(context.CorrelationId, query, "denied_scope", "unsupported_visibility_scope", 403);
        if (_nativeLibrary == 0)
            return Error(context.CorrelationId, query, "missing_capability",
                "content_index_query_unavailable", 503);
        return TryNativeStaticQuery(context, query, binding,
            out (int Status, string Response) nativeResponse)
            ? nativeResponse
            : Error(context.CorrelationId, query, "missing_capability",
                "content_index_query_unavailable", 503);
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
            if (operation is "get" or "detail")
            {
                JsonElement targetReference = query.GetProperty("target").GetProperty("definition_ref");
                if (targetReference.GetProperty("content_manifest_id").GetString()
                        != capture.Snapshot.ManifestId)
                {
                    response = Error(context.CorrelationId, query, "malformed",
                        "definition_manifest_mismatch", 400);
                    return true;
                }
                if (targetReference.GetProperty("variant").ValueKind != JsonValueKind.Null)
                {
                    response = Error(context.CorrelationId, query, "unsupported_filter",
                        "definition_variant_unavailable", 400);
                    return true;
                }
            }
            JsonElement filters = query.GetProperty("filters");
            string? literal = filters.GetProperty("display_name").ValueKind == JsonValueKind.Null
                ? null : filters.GetProperty("display_name").GetString();
            string? namespacedId = query.GetProperty("target")
                .GetProperty("definition_ref").ValueKind == JsonValueKind.Object
                ? query.GetProperty("target").GetProperty("definition_ref")
                    .GetProperty("namespaced_id").GetString()
                : null;
            string[] namespacedIds = filters.GetProperty("namespaced_ids")
                .EnumerateArray().Select(value => value.GetString()!).ToArray();
            string[] definitionRefs = filters.GetProperty("definition_refs")
                .EnumerateArray().Select(value => value.GetRawText()).ToArray();
            string customQuery = JsonSerializer.Serialize(new
            {
                operation,
                literal,
                entity_kind = "card",
                namespaced_id = namespacedId,
                namespaced_ids = namespacedIds,
                definition_refs = definitionRefs,
                scope = binding.GetProperty("visibility_scope").GetString(),
                manifest_id = binding.GetProperty("content_manifest_id").GetString(),
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
                        : status == 403
                                ? Error(context.CorrelationId, query, "denied_scope",
                                    "definition_excluded_by_scope", 403)
                            : status == 400
                                ? Error(context.CorrelationId, query, "unsupported_filter",
                                    "bounded_filter_unavailable", 400)
                            : Error(context.CorrelationId, query, "missing_capability",
                                "content_index_query_unavailable", 503);
                    return true;
                }
                NativeCorePage? page = JsonSerializer.Deserialize<NativeCorePage>(
                    nativeBody, NativeJsonOptions);
                if (page is null)
                {
                    response = Error(context.CorrelationId, query, "missing_capability",
                        "malformed_native_query_response", 503);
                    return true;
                }
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
                        value.rarity, value.unlock_state, Array.Empty<string>(), value.tags)).ToArray();
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

}
