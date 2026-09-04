// SPDX-License-Identifier: MIT

using System;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static bool TryParseRuntimeV3GameplayActionRequest(
        RuntimeContext context,
        string body,
        out RuntimeV3GameplayRequest? request,
        out string error)
    {
        request = null;
        error = "invalid_runtime_v3_gameplay_request";
        try
        {
            using JsonDocument document = JsonDocument.Parse(
                body,
                new JsonDocumentOptions { MaxDepth = 8, AllowTrailingCommas = false });
            JsonElement root = document.RootElement;
            if (root.ValueKind != JsonValueKind.Object
                || !HasExactProperties(root, RuntimeV3GameplayTopLevelFieldSet, RuntimeV3GameplayTopLevelFields.Length)
                || StringField(root, "protocol_version") != RuntimeV3GameplayProtocolVersion
                || StringField(root, "schema_digest") != RuntimeV3GameplaySchemaDigest
                || StringField(root, "kind") != "action_request"
                || StringField(root, "instance_id") != context.InstanceId
                || StringField(root, "session_id") != context.SessionId
                || StringField(root, "lease_id") != context.LeaseId
                || StringField(root, "correlation_id") != context.CorrelationId
                || StringField(root, "provenance.artifact") != RuntimeV3GameplayArtifact
                || StringField(root, "provenance.source") != RuntimeV3GameplaySchemaSource
                || StringField(root, "provenance.generator") != RuntimeV3GameplayGenerator)
            {
                error = "invalid_runtime_v3_gameplay_envelope";
                return false;
            }

            JsonElement provenance = root.GetProperty("provenance");
            if (provenance.ValueKind != JsonValueKind.Object
                || !HasExactProperties(provenance, RuntimeV3GameplayProvenanceFieldSet, RuntimeV3GameplayProvenanceFields.Length))
            {
                error = "invalid_runtime_v3_gameplay_provenance";
                return false;
            }

            if (!UInt64Property(root, "lease_epoch", out ulong leaseEpoch)
                || !UInt64Property(root, "generation", out ulong generation)
                || leaseEpoch > RuntimeV3GameplayMaxSafeInteger
                || generation > RuntimeV3GameplayMaxSafeInteger
                || !UInt64FromHeader(context.LeaseEpoch, out ulong headerEpoch)
                || leaseEpoch != headerEpoch
                || !IsRuntimeV2Identity(StringField(root, "correlation_id"))
                || !IsRuntimeV2Identity(StringField(root, "instance_id"))
                || !IsRuntimeV2Identity(StringField(root, "session_id"))
                || !IsRuntimeV2Identity(StringField(root, "lease_id")))
            {
                error = "invalid_runtime_v3_gameplay_identity";
                return false;
            }

            if (root.GetProperty("operation_id").ValueKind != JsonValueKind.String
                || !IsRuntimeV2Identity(root.GetProperty("operation_id").GetString()))
            {
                error = "invalid_operation_id";
                return false;
            }

            JsonElement action = root.GetProperty("action");
            if (!IsNull(root.GetProperty("observation"))
                || !IsNull(root.GetProperty("status"))
                || !IsNull(root.GetProperty("error_code"))
                || !IsNull(root.GetProperty("effect_witness"))
                || !TryParseRuntimeV3GameplayAction(action, out ushort cardIndex, out string? targetId))
            {
                error = "invalid_runtime_v3_gameplay_action_shape";
                return false;
            }

            request = new RuntimeV3GameplayRequest(
                root.GetProperty("operation_id").GetString()!,
                generation,
                cardIndex,
                targetId,
                JsonSerializer.Serialize(new { generation, cardIndex, targetId }));
            return true;
        }
        catch (JsonException)
        {
            error = "invalid_runtime_v3_gameplay_json";
            return false;
        }
        catch (InvalidOperationException)
        {
            error = "invalid_runtime_v3_gameplay_value";
            return false;
        }
    }

    private static bool TryParseRuntimeV3GameplayAction(
        JsonElement action,
        out ushort cardIndex,
        out string? targetId)
    {
        cardIndex = 0;
        targetId = null;
        if (action.ValueKind != JsonValueKind.Object
            || !HasExactProperties(action, RuntimeV3GameplayActionFieldSet, RuntimeV3GameplayActionFields.Length)
            || StringField(action, "action_id") != RuntimeV3GameplayActionId
            || !UInt64Property(action, "card_index", out ulong cardIndexValue)
            || cardIndexValue > RuntimeV3GameplayMaxCardIndex)
        {
            return false;
        }

        JsonElement target = action.GetProperty("target_id");
        if (target.ValueKind == JsonValueKind.String)
        {
            targetId = target.GetString();
            if (!IsRuntimeV2Identity(targetId))
            {
                return false;
            }
        }
        else if (target.ValueKind != JsonValueKind.Null)
        {
            return false;
        }

        cardIndex = (ushort)cardIndexValue;
        return true;
    }

}
