// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV4ExpertSupport
{
    private static string Response(
        RuntimeV4ExpertContext context,
        string stateId,
        ulong generation,
        string operationId,
        RuntimeV4ExpertGameplayAction? action,
        string status,
        RuntimeV4ExpertGameplayObservation? observation,
        bool settled,
        string? errorCode,
        ulong? beforeGeneration)
    {
        JsonElement? observationValue = null;
        if (observation is not null
            && RuntimeV4ExpertGameplayCodec.TrySerialize(observation, out string json, out _))
        {
            using JsonDocument document = JsonDocument.Parse(json);
            observationValue = document.RootElement.Clone();
        }
        var root = new Dictionary<string, object?>
        {
            ["protocol_version"] = ProtocolVersion,
            ["schema_digest"] = SchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = Artifact,
                ["source"] = SchemaSource,
                ["generator"] = Generator
            },
            ["profile"] = Profile,
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = context.LeaseEpoch,
            ["generation"] = generation,
            ["state_id"] = stateId,
            ["operation_id"] = operationId,
            ["kind"] = "action_response",
            ["action"] = action is null ? null : ActionValue(action),
            ["status"] = status,
            ["observation"] = observationValue,
            ["transition"] = settled ? new Dictionary<string, object?>
            {
                ["kind"] = "potion_use_settled",
                ["before_generation"] = beforeGeneration,
                ["after_generation"] = generation,
                ["potion_id"] = action?.Value,
                ["removed"] = true
            } : null,
            ["error_code"] = errorCode
        };
        return JsonSerializer.Serialize(root);
    }

    private static Dictionary<string, object?> ActionValue(
        RuntimeV4ExpertGameplayAction action) => new()
    {
        ["action_id"] = action.ActionId,
        ["action"] = new Dictionary<string, object?>
        {
            ["kind"] = action.Kind,
            ["potion_id"] = action.Value,
            ["target_id"] = action.TargetId
        }
    };

    private static bool TryParseRequest(
        RuntimeV4ExpertContext context,
        string body,
        out ParsedExpertRequest? request)
    {
        request = null;
        try
        {
            using JsonDocument document = JsonDocument.Parse(body,
                new JsonDocumentOptions { MaxDepth = 12 });
            JsonElement root = document.RootElement;
            if (!HasExactFields(root,
                    "protocol_version", "schema_digest", "provenance", "profile",
                    "correlation_id", "instance_id", "session_id", "lease_id", "lease_epoch",
                    "generation", "state_id", "operation_id", "kind", "action", "status",
                    "observation", "transition", "error_code")
                || StringField(root, "protocol_version") != ProtocolVersion
                || StringField(root, "schema_digest") != SchemaDigest
                || StringField(root, "profile") != Profile
                || StringField(root, "kind") != "action_request"
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || !EpochField(root, "lease_epoch", context.LeaseEpoch)
                || !UInt64Field(root, "generation", out ulong generation)
                || generation > RuntimeV3GameplayContract.MaxGeneration
                || !IdentityField(root, "state_id", out string? stateId)
                || !IdentityField(root, "operation_id", out string? operationId)
                || !NullFields(root, "status", "observation", "transition", "error_code")
                || !Provenance(root))
            {
                return false;
            }
            if (!TryAction(root, out RuntimeV4ExpertGameplayAction? action)) return false;
            request = new ParsedExpertRequest(
                new RuntimeV3OperationKey(context.InstanceId, context.SessionId, context.LeaseId,
                    context.LeaseEpoch, operationId!), action!, stateId!, generation);
            return true;
        }
        catch (JsonException)
        {
            return false;
        }
    }

    private static bool TryAction(JsonElement root, out RuntimeV4ExpertGameplayAction? action)
    {
        action = null;
        if (!root.TryGetProperty("action", out JsonElement reference)
            || !HasExactFields(reference, "action_id", "action")
            || !IdentityField(reference, "action_id", out string? actionId)
            || !reference.TryGetProperty("action", out JsonElement value)
            || !HasExactFields(value, "kind", "potion_id", "target_id")
            || StringField(value, "kind") != "use_potion"
            || !IdentityField(value, "potion_id", out string? potionId)
            || !OptionalIdentityField(value, "target_id", out string? targetId))
        {
            return false;
        }
        action = new RuntimeV4ExpertGameplayAction(actionId!, "use_potion", potionId!, targetId, null);
        return true;
    }

    private static bool Provenance(JsonElement root) =>
        root.TryGetProperty("provenance", out JsonElement value)
        && HasExactFields(value, "artifact", "source", "generator")
        && StringField(value, "artifact") == Artifact
        && StringField(value, "source") == SchemaSource
        && StringField(value, "generator") == Generator;

    private static bool NullFields(JsonElement root, params string[] fields)
    {
        foreach (string field in fields)
            if (!root.TryGetProperty(field, out JsonElement value)
                || value.ValueKind != JsonValueKind.Null) return false;
        return true;
    }

    private static bool EpochField(JsonElement root, string field, ulong expected) =>
        UInt64Field(root, field, out ulong value) && value == expected;

    private static bool UInt64Field(JsonElement root, string field, out ulong value)
    {
        value = 0;
        return root.TryGetProperty(field, out JsonElement element)
            && element.ValueKind == JsonValueKind.Number
            && element.TryGetUInt64(out value)
            && value <= RuntimeV3GameplayContract.MaxGeneration;
    }

    private static bool IdentityField(JsonElement root, string field, out string? value)
    {
        value = StringField(root, field);
        return value is not null && RuntimeV3GameplayContract.IsIdentity(value);
    }

    private static bool OptionalIdentityField(JsonElement root, string field, out string? value)
    {
        value = null;
        if (!root.TryGetProperty(field, out JsonElement element)) return false;
        return element.ValueKind == JsonValueKind.Null
            || element.ValueKind == JsonValueKind.String
                && RuntimeV3GameplayContract.IsIdentity(value = element.GetString() ?? string.Empty);
    }

    private static string? StringField(JsonElement root, string field) =>
        root.TryGetProperty(field, out JsonElement value)
            && value.ValueKind == JsonValueKind.String ? value.GetString() : null;

    private static bool HasExactFields(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object) return false;
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
            if (!names.Add(property.Name)) return false;
        if (names.Count != fields.Length) return false;
        foreach (string field in fields)
            if (!names.Contains(field)) return false;
        return true;
    }
}
