// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string RuntimeV2StateResponse(RuntimeContext context, RuntimeV2HostObservation observation)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV2ProtocolVersion,
            ["schema_digest"] = RuntimeV2SchemaDigest,
            ["provenance"] = RuntimeV2Provenance(),
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            ["generation"] = observation.Generation,
            ["kind"] = "state_response",
            ["operation_id"] = null,
            ["observation"] = RuntimeV2ObservationValue(observation),
            ["action"] = null,
            ["status"] = null,
            ["error_code"] = null,
            ["effect_witness"] = null
        });
    }

    private static string RuntimeV2OperationResponse(RuntimeContext context, RuntimeV2Operation operation)
    {
        bool dispatching = operation.Status == "dispatching";
        RuntimeV2HostObservation? observation = dispatching ? null : operation.Observation;
        ulong generation = observation?.Generation ?? operation.RequestGeneration;
        return RuntimeV2ResultResponse(
            context,
            operation.OperationId,
            generation,
            dispatching ? "unknown" : operation.Status,
            observation,
            dispatching ? "sts2.runtime/operation_in_progress" : operation.ErrorCode,
            operation.Status == "settled");
    }

    private static string RuntimeV2ResultResponse(
        RuntimeContext context,
        string operationId,
        ulong generation,
        string status,
        RuntimeV2HostObservation? observation,
        string? errorCode,
        bool witnessed)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV2ProtocolVersion,
            ["schema_digest"] = RuntimeV2SchemaDigest,
            ["provenance"] = RuntimeV2Provenance(),
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            ["generation"] = generation,
            ["kind"] = "action_response",
            ["operation_id"] = operationId,
            ["observation"] = observation.HasValue ? RuntimeV2ObservationValue(observation.Value) : null,
            ["action"] = new Dictionary<string, object?> { ["action_id"] = RuntimeV2ActionId },
            ["status"] = status,
            ["error_code"] = errorCode,
            ["effect_witness"] = witnessed
                ? new Dictionary<string, object?> { ["kind"] = RuntimeV2EffectKind, ["generation"] = generation }
                : null
        });
    }

    private static Dictionary<string, object?> RuntimeV2ObservationValue(RuntimeV2HostObservation observation) => new()
    {
        ["combat_phase"] = observation.CombatPhase,
        ["turn_index"] = observation.TurnIndex,
        ["host_ready"] = observation.HostReady,
        ["generation"] = observation.Generation
    };

    private static Dictionary<string, string> RuntimeV2Provenance() => new()
    {
        ["artifact"] = RuntimeV2Artifact,
        ["source"] = RuntimeV2SchemaSource,
        ["generator"] = RuntimeV2Generator
    };

    private static string RuntimeV2PlainError(string code) => JsonSerializer.Serialize(new Dictionary<string, string>
    {
        ["error_code"] = code
    });

    private static int RuntimeStatusFor(string status) => status switch
    {
        "rejected" => RuntimeRejected,
        "unknown" => RuntimeUnavailable,
        "dispatching" => RuntimeUnavailable,
        _ => RuntimeAccepted
    };

    private static ulong RuntimeV2GenerationFor(RuntimeV2HostObservation observation) => observation.Generation;
}
