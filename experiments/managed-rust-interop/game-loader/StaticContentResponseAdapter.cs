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
                kind = "text_list"; fieldValue = value.Tags?.ToArray();
                availability = value.Tags is null ? "not_observable" : "available";
                if (availability == "available")
                    reason = null;
                break;
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

}
