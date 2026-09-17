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
                if (StaticCursors.Count >= MaxStaticCursors
                    && StaticCursors.Keys.FirstOrDefault() is string oldest)
                    StaticCursors.Remove(oldest);
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
