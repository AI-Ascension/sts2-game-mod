// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;
using AiAscension.Sts2GameMod.Runtime;

internal static class ProbeHelpers
{
    private const string Digest =
        "376845b0c86b4afcd2c79ffba753eb7e7e416f5410da26b4dae970cfee2221d9";

    internal static string Request(
        string kind,
        string mode,
        string manifest,
        string locale,
        int pageItems,
        string[] fields,
        string? cursor = null,
        object? target = null,
        object? parent = null,
        object? instance = null,
        object? snapshot = null,
        string visibility = "public",
        object? filters = null)
    {
        return JsonSerializer.Serialize(new
        {
            protocol_version = "game-information-query-v1",
            schema_digest = Digest,
            provenance = new
            {
                artifact = "sts2-protocol/game-information-query-v1",
                source = "schemas/game-information-query-v1.schema.json",
                generator = "hand-authored"
            },
            correlation_id = "corr",
            kind = "query_request",
            capabilities = (object?)null,
            error = (object?)null,
            result = (object?)null,
            query = new
            {
                cursor,
                detail_level = "summary",
                projection = "summary",
                query_kind = kind,
                entity_kind = "card",
                fields,
                filters = filters ?? new
                {
                    definition_refs = Array.Empty<object>(),
                    display_name = (string?)null,
                    instance_ids = Array.Empty<string>(),
                    namespaced_ids = Array.Empty<string>()
                },
                limits = new { item_bytes = 4096, page_bytes = 65536, page_items = pageItems, text_bytes = 4096 },
                binding = new
                {
                    mode,
                    content_manifest_id = manifest,
                    locale,
                    visibility_scope = visibility,
                    instance_ref = instance,
                    snapshot_ref = snapshot
                },
                target = target ?? new { definition_ref = (object?)null, instance_ref = (object?)null },
                parent_observation = parent
            }
        });
    }

    internal static void Check(bool condition, string message)
    {
        if (!condition)
            throw new InvalidOperationException(message);
        Console.WriteLine($"PASS: {message}");
    }

    internal static void ExpectStatus(string body, int expected, string message)
    {
        (int status, string response) = ModEntry.Invoke(body);
        Check(status == expected, $"{message} ({status})");
        CanonicalValidation.ValidateCanonicalResponse(response, message);
        if (expected == 200)
            Check(response.Contains("\"kind\":\"query_response\"", StringComparison.Ordinal),
                $"{message} has query response envelope");
    }

    internal static string CorruptAccounting(string response, string field)
    {
        int accounting = response.IndexOf("\"accounting\"", StringComparison.Ordinal);
        int property = response.IndexOf($"\"{field}\":", accounting, StringComparison.Ordinal);
        if (accounting < 0 || property < 0)
            throw new InvalidOperationException($"accounting field {field} is missing");
        int valueStart = property + field.Length + 3;
        int valueEnd = valueStart;
        while (valueEnd < response.Length && char.IsDigit(response[valueEnd]))
            valueEnd++;
        if (valueStart == valueEnd)
            throw new InvalidOperationException($"accounting field {field} is not numeric");
        int value = int.Parse(response[valueStart..valueEnd], CultureInfo.InvariantCulture);
        return string.Concat(response.AsSpan(0, valueStart),
            (value + 1).ToString(CultureInfo.InvariantCulture),
            response.AsSpan(valueEnd));
    }

}
