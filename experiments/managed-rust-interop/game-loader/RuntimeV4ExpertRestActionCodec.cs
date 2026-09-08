// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Closed JSON projection for the inactive rest profile. Strict request and response checks are
/// split into companion files so each responsibility remains independently reviewable.
/// </summary>
internal static partial class RuntimeV4ExpertRestActionCodec
{
    internal static bool TrySerializeRequest(
        RuntimeV4ExpertRestRequest request,
        out string json,
        out string error)
    {
        json = string.Empty;
        if (!ValidateRequest(request, out error)) return false;
        json = JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV4ExpertRestActionContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV4ExpertRestActionContract.SchemaDigest,
            ["provenance"] = Provenance(),
            ["profile"] = RuntimeV4ExpertRestActionContract.Profile,
            ["correlation_id"] = request.Context.CorrelationId,
            ["instance_id"] = request.Context.InstanceId,
            ["session_id"] = request.Context.SessionId,
            ["lease_id"] = request.Context.LeaseId,
            ["lease_epoch"] = request.Context.LeaseEpoch,
            ["generation"] = request.Generation,
            ["state_id"] = request.StateId,
            ["operation_id"] = request.Operation.OperationId,
            ["kind"] = "action_request",
            ["action"] = ActionReference(request.Action),
            ["status"] = null,
            ["observation"] = null,
            ["transition"] = null,
            ["effect_witness"] = null,
            ["error_code"] = null
        });
        return true;
    }

    internal static bool TrySerializeResponse(
        RuntimeV4ExpertRestResponse response,
        out string json,
        out string error)
    {
        json = string.Empty;
        if (!ValidateResponse(response, out error)) return false;

        JsonElement? observation = null;
        if (response.Observation is not null)
        {
            if (!RuntimeV4ExpertGameplayCodec.TrySerialize(
                    response.Observation, out string observationJson, out string observationError))
            {
                error = observationError.Length == 0
                    ? "rest response observation is not serializable"
                    : observationError;
                return false;
            }
            using JsonDocument document = JsonDocument.Parse(observationJson);
            observation = document.RootElement.Clone();
        }

        RuntimeV4ExpertRestEffectWitness? witness = response.EffectWitness;
        var root = new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV4ExpertRestActionContract.ProtocolVersion,
            ["schema_digest"] = RuntimeV4ExpertRestActionContract.SchemaDigest,
            ["provenance"] = Provenance(),
            ["profile"] = RuntimeV4ExpertRestActionContract.Profile,
            ["correlation_id"] = response.Context.CorrelationId,
            ["instance_id"] = response.Context.InstanceId,
            ["session_id"] = response.Context.SessionId,
            ["lease_id"] = response.Context.LeaseId,
            ["lease_epoch"] = response.Context.LeaseEpoch,
            ["generation"] = response.Generation,
            ["state_id"] = response.StateId,
            ["operation_id"] = response.Operation.OperationId,
            ["kind"] = "action_response",
            ["action"] = response.Action is null ? null : ActionReference(response.Action),
            ["status"] = response.Status,
            ["observation"] = observation,
            ["transition"] = response.Transition is null ? null : Transition(response.Transition),
            ["effect_witness"] = witness is null ? null : EffectWitness(witness),
            ["error_code"] = response.ErrorCode
        };
        json = JsonSerializer.Serialize(root);
        return true;
    }
}
