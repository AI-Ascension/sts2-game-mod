// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class RuntimeV3GameplaySupport
{
    private static string SerializeEnvelope(
        string kind,
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong generation,
        string? stateId,
        string? operationId,
        object? observation,
        object? legalActions,
        object? action,
        string? status,
        RuntimeV3TransitionWitness? witness,
        string? errorCode,
        int? waitForMillis,
        string? waitOutcome)
    {
        var envelope = new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplayContract.SchemaDigest,
            ["provenance"] = new Dictionary<string, string>
            {
                ["artifact"] = RuntimeV3GameplayContract.Artifact,
                ["source"] = RuntimeV3GameplayContract.SchemaSource,
                ["generator"] = RuntimeV3GameplayContract.Generator
            },
            ["correlation_id"] = correlationId,
            ["instance_id"] = instanceId,
            ["session_id"] = sessionId,
            ["lease_id"] = leaseId,
            ["lease_epoch"] = leaseEpoch,
            ["generation"] = generation,
            ["kind"] = kind,
            ["state_id"] = stateId,
            ["operation_id"] = operationId,
            ["observation"] = observation,
            ["legal_actions"] = legalActions,
            ["action"] = action,
            ["status"] = status,
            ["transition"] = witness is null
                ? null
                : new Dictionary<string, object?>
                {
                    ["from_generation"] = witness.FromGeneration,
                    ["to_generation"] = witness.ToGeneration,
                    ["state_id"] = witness.StateId,
                    ["effect_kind"] = witness.EffectKind
                },
            ["error_code"] = errorCode,
            ["wait_for_millis"] = waitForMillis,
            ["wait_outcome"] = waitOutcome,
            ["recovery"] = null
        };
        string json = JsonSerializer.Serialize(envelope);
        using JsonDocument document = JsonDocument.Parse(json);
        if (json.Length > MaxResponseBytes
            || !PrivilegedFieldGuard.IsSafeJson(document.RootElement, out _))
        {
            return "{\"error_code\":\"runtime_v3_response_unavailable\"}";
        }
        return json;
    }

    private static bool HasExactFields(JsonElement value, params string[] fields)
    {
        if (value.ValueKind != JsonValueKind.Object)
        {
            return false;
        }
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (JsonProperty property in value.EnumerateObject())
        {
            if (!names.Add(property.Name))
            {
                return false;
            }
        }
        if (names.Count != fields.Length)
        {
            return false;
        }
        foreach (string field in fields)
        {
            if (!names.Contains(field))
            {
                return false;
            }
        }
        return true;
    }

    private static string ErrorEnvelope(
        string correlationId,
        string instanceId,
        string sessionId,
        string leaseId,
        ulong leaseEpoch,
        ulong generation,
        string kind,
        string errorCode,
        out int status)
    {
        status = 400;
        return RecoveryState(
            instanceId, sessionId, leaseId, correlationId, leaseEpoch, generation,
            kind, errorCode, out _);
    }

    // The neutral catalog response has no failure variant. This is an owner-local HTTP
    // error, not a successful observation or another kind of gameplay response.
    private static string CatalogError(string correlationId, string errorCode, int httpStatus, out int status)
    {
        status = httpStatus;
        return JsonSerializer.Serialize(new { correlation_id = correlationId,
            error_code = errorCode, recovery = "reobserve" });
    }

}
