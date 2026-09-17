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
    private const int MaxStaticCursors = 128;
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
        string unlock_state,
        string[]? tags);

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

}
