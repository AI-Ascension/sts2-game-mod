// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static bool TryParseRuntimeV2ActionRequest(
        RuntimeContext context,
        string body,
        out RuntimeV2Request? request,
        out string error)
    {
        request = null;
        error = "invalid_runtime_v2_request";
        try
        {
            using JsonDocument document = JsonDocument.Parse(
                body,
                new JsonDocumentOptions { MaxDepth = 8, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object
                || !HasExactProperties(root, RuntimeV2TopLevelFieldSet, RuntimeV2TopLevelFields.Length)
                || StringField(root, "protocol_version") != RuntimeV2ProtocolVersion
                || StringField(root, "schema_digest") != RuntimeV2SchemaDigest
                || StringField(root, "kind") != "action_request"
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "provenance.artifact") != RuntimeV2Artifact
                || StringField(root, "provenance.source") != RuntimeV2SchemaSource
                || StringField(root, "provenance.generator") != RuntimeV2Generator)
            {
                error = "invalid_runtime_v2_envelope";
                return false;
            }

            JsonElement provenance = root.GetProperty("provenance");
            if (provenance.ValueKind != JsonValueKind.Object
                || !HasExactProperties(provenance, RuntimeV2ProvenanceFieldSet, RuntimeV2ProvenanceFields.Length))
            {
                error = "invalid_runtime_v2_provenance";
                return false;
            }

            if (!UInt64Property(root, "lease_epoch", out ulong leaseEpoch)
                || !UInt64Property(root, "generation", out ulong generation)
                || leaseEpoch > RuntimeV2MaxSafeInteger
                || generation > RuntimeV2MaxSafeInteger
                || !UInt64FromHeader(context.LeaseEpoch, out ulong headerEpoch)
                || leaseEpoch != headerEpoch
                || !IsRuntimeV2Identity(StringField(root, "correlation_id"))
                || !IsRuntimeV2Identity(StringField(root, "instance_id"))
                || !IsRuntimeV2Identity(StringField(root, "session_id"))
                || !IsRuntimeV2Identity(StringField(root, "lease_id")))
            {
                error = "invalid_runtime_v2_identity";
                return false;
            }

            if (root.GetProperty("operation_id").ValueKind != JsonValueKind.String
                || !IsRuntimeV2Identity(root.GetProperty("operation_id").GetString()))
            {
                error = "invalid_operation_id";
                return false;
            }

            if (!IsNull(root.GetProperty("observation"))
                || !IsNull(root.GetProperty("status"))
                || !IsNull(root.GetProperty("error_code"))
                || !IsNull(root.GetProperty("effect_witness"))
                || !TryParseRuntimeV2Action(root.GetProperty("action")))
            {
                error = "invalid_runtime_v2_action_shape";
                return false;
            }

            request = new RuntimeV2Request(
                root.GetProperty("operation_id").GetString()!,
                generation,
                generation.ToString(CultureInfo.InvariantCulture));
            return true;
        }
        catch (JsonException)
        {
            error = "invalid_runtime_v2_json";
            return false;
        }
        catch (InvalidOperationException)
        {
            error = "invalid_runtime_v2_value";
            return false;
        }
    }

    private static bool TryParseRuntimeV2Action(JsonElement action)
    {
        return action.ValueKind == JsonValueKind.Object
            && HasExactProperties(action, RuntimeV2ActionFieldSet, RuntimeV2ActionFields.Length)
            && StringField(action, "action_id") == RuntimeV2ActionId;
    }

    private static bool HasExactProperties(JsonElement value, HashSet<string> expected, int expectedCount)
    {
        int propertyCount = 0;
        foreach (JsonProperty _ in value.EnumerateObject())
        {
            propertyCount++;
        }

        if (propertyCount != expectedCount)
        {
            return false;
        }

        HashSet<string> actual = new(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            if (!actual.Add(property.Name) || !expected.Contains(property.Name))
            {
                return false;
            }
        }

        return actual.Count == expectedCount;
    }

    private static bool UInt64Property(JsonElement root, string name, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(name, out JsonElement element)
            && element.ValueKind == JsonValueKind.Number
            && element.TryGetUInt64(out value);
    }

    private static bool UInt64FromHeader(string value, out ulong parsed)
    {
        return UInt64.TryParse(value, NumberStyles.None, CultureInfo.InvariantCulture, out parsed);
    }

    private static bool IsNull(JsonElement value) => value.ValueKind == JsonValueKind.Null;

    private static bool IsRuntimeV2Identity(string? value)
    {
        if (String.IsNullOrEmpty(value)
            || value.Length > 128
            || value.Contains("..", StringComparison.Ordinal))
        {
            return false;
        }

        foreach (char character in value)
        {
            if (!(character is >= 'A' and <= 'Z')
                && !(character is >= 'a' and <= 'z')
                && !(character is >= '0' and <= '9')
                && character is not ('-' or '_' or '.' or ':' or '/'))
            {
                return false;
            }
        }

        return true;
    }
}
