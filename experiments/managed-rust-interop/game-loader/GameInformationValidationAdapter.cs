// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static bool ValidEnvelope(
        JsonElement root,
        RuntimeContext context,
        out JsonElement query,
        out string reason)
    {
        query = default;
        reason = "invalid_query_envelope";
        if (root.ValueKind != JsonValueKind.Object
            || !ExactProperties(root, "protocol_version", "schema_digest", "provenance",
                "correlation_id", "kind", "query", "capabilities", "error", "result")
            || !StringEquals(root, "protocol_version", GameInformationProtocol)
            || !StringEquals(root, "schema_digest", GameInformationDigest)
            || !StringEquals(root, "kind", "query_request")
            || !StringEquals(root, "correlation_id", context.CorrelationId)
            || !root.TryGetProperty("provenance", out JsonElement provenance)
            || !ValidProvenance(provenance)
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
        if (!ExactProperties(query, "cursor", "detail_level", "query_kind", "entity_kind",
                "fields", "filters", "limits", "binding", "target", "projection",
                "parent_observation")
            || !query.TryGetProperty("query_kind", out JsonElement queryKind)
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
        if (parent.ValueKind != JsonValueKind.Null && !ValidParentObservation(parent))
        {
            reason = "invalid_parent_observation";
            return false;
        }
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
        ExactProperties(binding, "mode", "content_manifest_id", "locale", "visibility_scope",
            "instance_ref", "snapshot_ref")
        && (StringEquals(binding, "mode", "static") || StringEquals(binding, "mode", "live"))
        && StringIdentity(binding, "content_manifest_id")
        && StringIdentity(binding, "visibility_scope")
        && binding.TryGetProperty("instance_ref", out _)
        && binding.TryGetProperty("snapshot_ref", out _);

    private static bool ValidTarget(JsonElement target) =>
        target.ValueKind == JsonValueKind.Object
        && ExactProperties(target, "definition_ref", "instance_ref")
        && target.TryGetProperty("definition_ref", out JsonElement definition)
        && target.TryGetProperty("instance_ref", out JsonElement instance)
        && (definition.ValueKind == JsonValueKind.Null || ValidDefinitionRef(definition))
        && (instance.ValueKind == JsonValueKind.Null || ValidInstanceRef(instance));

    private static bool ValidDefinitionRef(JsonElement value) =>
        value.ValueKind == JsonValueKind.Object
        && ExactProperties(value, "content_manifest_id", "entity_kind", "namespaced_id", "variant")
        && StringIdentity(value, "content_manifest_id")
        && StringEquals(value, "entity_kind", "card")
        && StringIdentity(value, "namespaced_id")
        && value.TryGetProperty("variant", out JsonElement variant)
        && (variant.ValueKind == JsonValueKind.Null || ValidIdentity(variant));

    private static bool ValidInstanceRef(JsonElement value) =>
        value.ValueKind == JsonValueKind.Object
        && ExactProperties(value, "instance_id", "run_id", "epoch", "entity_kind", "entity_id")
        && StringIdentity(value, "instance_id")
        && StringIdentity(value, "run_id")
        && value.TryGetProperty("epoch", out JsonElement epoch)
        && epoch.TryGetUInt64(out _)
        && StringEquals(value, "entity_kind", "card")
        && StringIdentity(value, "entity_id");

    private static bool ValidFilters(JsonElement filters)
    {
        if (filters.ValueKind != JsonValueKind.Object
            || !ExactProperties(filters, "display_name", "namespaced_ids", "definition_refs",
                "instance_ids")
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
        && ExactProperties(limits, "item_bytes", "page_bytes", "page_items", "text_bytes")
        && limits.TryGetProperty("item_bytes", out JsonElement item)
        && item.TryGetInt32(out int itemValue) && itemValue is >= 1 and <= MaxItemBytes
        && limits.TryGetProperty("page_bytes", out JsonElement page)
        && page.TryGetInt32(out int pageValue) && pageValue is >= 1 and <= MaxPageBytes
        && limits.TryGetProperty("page_items", out JsonElement count)
        && count.TryGetInt32(out int countValue) && countValue is >= 1 and <= MaxPageItems
        && limits.TryGetProperty("text_bytes", out JsonElement text)
        && text.TryGetInt32(out int textValue) && textValue is >= 1 and <= MaxTextBytes;

    private static bool ValidProvenance(JsonElement provenance) =>
        provenance.ValueKind == JsonValueKind.Object
        && ExactProperties(provenance, "artifact", "source", "generator")
        && StringIdentity(provenance, "artifact")
        && StringIdentity(provenance, "source")
        && StringIdentity(provenance, "generator");

    private static bool ValidParentObservation(JsonElement parent) =>
        parent.ValueKind == JsonValueKind.Object
        && ExactProperties(parent, "state_generation", "snapshot_ref", "instance_ref")
        && parent.TryGetProperty("state_generation", out JsonElement generation)
        && generation.TryGetUInt64(out _)
        && parent.TryGetProperty("snapshot_ref", out JsonElement snapshot)
        && snapshot.ValueKind == JsonValueKind.Object
        && ExactProperties(snapshot, "snapshot_id", "state_generation", "instance_ref")
        && StringIdentity(snapshot, "snapshot_id")
        && snapshot.TryGetProperty("state_generation", out JsonElement snapshotGeneration)
        && snapshotGeneration.TryGetUInt64(out _)
        && parent.TryGetProperty("instance_ref", out JsonElement instance)
        && ValidInstanceRef(instance);

    private static bool ExactProperties(JsonElement value, params string[] names)
    {
        if (value.ValueKind != JsonValueKind.Object)
            return false;
        var allowed = new HashSet<string>(names, StringComparer.Ordinal);
        return value.EnumerateObject().All(property => allowed.Contains(property.Name));
    }

}
