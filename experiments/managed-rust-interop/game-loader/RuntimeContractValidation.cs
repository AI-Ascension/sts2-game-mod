// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const ulong RuntimeMaximumInteger = 9007199254740991;

    private static bool ValidRuntimeContext(RuntimeContext context) =>
        RuntimeIdentity(context.InstanceId) && RuntimeIdentity(context.CallerId)
        && RuntimeIdentity(context.SessionId) && RuntimeIdentity(context.LeaseId)
        && RuntimeIdentity(context.CorrelationId)
        && ulong.TryParse(context.LeaseEpoch, NumberStyles.None, CultureInfo.InvariantCulture, out ulong epoch)
        && epoch <= RuntimeMaximumInteger;

    private static bool RuntimeIdentity(string value)
    {
        if (string.IsNullOrEmpty(value) || value.Length > 128) return false;
        foreach (char item in value)
        {
            if (!(item is >= 'a' and <= 'z' or >= 'A' and <= 'Z' or >= '0' and <= '9'
                or '_' or '.' or ':' or '/' or '-')) return false;
        }
        return true;
    }

    private static bool ExactFields(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        HashSet<string> remaining = new(fields, StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            if (!remaining.Remove(property.Name)) return false;
        }
        return remaining.Count == 0;
    }

    private static bool BoundedInteger(JsonElement value, out ulong number)
    {
        number = 0;
        return value.ValueKind == JsonValueKind.Number && value.TryGetUInt64(out number)
            && number <= RuntimeMaximumInteger;
    }

    private static bool ValidRuntimeAction(JsonElement root, RuntimeContext context, out ulong generation)
    {
        generation = 0;
        if (!ExactFields(root, "protocol_version", "schema_digest", "provenance", "correlation_id",
            "instance_id", "session_id", "lease_id", "lease_epoch", "generation", "kind",
            "observation", "action", "status", "error_code", "effect_witness")) return false;
        if (StringField(root, "protocol_version") != "runtime-v1"
            || StringField(root, "schema_digest") != RuntimeSchemaDigest
            || StringField(root, "kind") != "action_request"
            || StringField(root, "instance_id") != context.InstanceId
            || StringField(root, "session_id") != context.SessionId
            || StringField(root, "lease_id") != context.LeaseId
            || StringField(root, "correlation_id") != context.CorrelationId
            || !BoundedInteger(root.GetProperty("lease_epoch"), out ulong epoch)
            || epoch != ParseEpoch(context.LeaseEpoch)
            || !BoundedInteger(root.GetProperty("generation"), out generation)) return false;
        JsonElement provenance = root.GetProperty("provenance");
        if (!ExactFields(provenance, "artifact", "source", "generator")
            || StringField(provenance, "artifact") != RuntimeArtifact
            || StringField(provenance, "source") != RuntimeSchemaSource
            || StringField(provenance, "generator") != RuntimeGenerator) return false;
        foreach (string field in new[] { "observation", "status", "error_code", "effect_witness" })
        {
            if (root.GetProperty(field).ValueKind != JsonValueKind.Null) return false;
        }
        JsonElement action = root.GetProperty("action");
        return ExactFields(action, "action_id") && StringField(action, "action_id") == "show_runtime_probe";
    }
}
