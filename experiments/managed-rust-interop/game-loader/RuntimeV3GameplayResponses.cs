// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using System.Text.Json;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static string RuntimeV3GameplayStateResponse(
        RuntimeContext context,
        RuntimeV3GameplayHostObservation observation)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplaySchemaDigest,
            ["provenance"] = RuntimeV3GameplayProvenance(),
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            ["generation"] = observation.Generation,
            ["kind"] = "state_response",
            ["operation_id"] = null,
            ["observation"] = RuntimeV3GameplayObservationValue(observation),
            ["action"] = null,
            ["status"] = null,
            ["error_code"] = null,
            ["effect_witness"] = null
        });
    }

    private static string RuntimeV3GameplayOperationResponse(
        RuntimeContext context,
        RuntimeV3GameplayOperation operation)
    {
        bool dispatching = operation.Status == "dispatching";
        RuntimeV3GameplayHostObservation? observation = dispatching ? null : operation.Observation;
        ulong generation = observation?.Generation ?? operation.RequestGeneration;
        return RuntimeV3GameplayResultResponse(
            context,
            operation.OperationId,
            operation.CardIndex,
            operation.TargetId,
            generation,
            dispatching ? "unknown" : operation.Status,
            observation,
            dispatching ? "sts2.runtime/operation_in_progress" : operation.ErrorCode,
            operation.Status == "settled");
    }

    private static string RuntimeV3GameplayResultResponse(
        RuntimeContext context,
        string operationId,
        ushort cardIndex,
        string? targetId,
        ulong generation,
        string status,
        RuntimeV3GameplayHostObservation? observation,
        string? errorCode,
        bool witnessed)
    {
        return JsonSerializer.Serialize(new Dictionary<string, object?>
        {
            ["protocol_version"] = RuntimeV3GameplayProtocolVersion,
            ["schema_digest"] = RuntimeV3GameplaySchemaDigest,
            ["provenance"] = RuntimeV3GameplayProvenance(),
            ["correlation_id"] = context.CorrelationId,
            ["instance_id"] = context.InstanceId,
            ["session_id"] = context.SessionId,
            ["lease_id"] = context.LeaseId,
            ["lease_epoch"] = ParseEpoch(context.LeaseEpoch),
            ["generation"] = generation,
            ["kind"] = "action_response",
            ["operation_id"] = operationId,
            ["observation"] = observation.HasValue
                ? RuntimeV3GameplayObservationValue(observation.Value)
                : null,
            ["action"] = new Dictionary<string, object?>
            {
                ["action_id"] = RuntimeV3GameplayActionId,
                ["card_index"] = cardIndex,
                ["target_id"] = targetId
            },
            ["status"] = status,
            ["error_code"] = errorCode,
            ["effect_witness"] = witnessed
                ? new Dictionary<string, object?>
                {
                    ["kind"] = RuntimeV3GameplayEffectKind,
                    ["generation"] = generation,
                    ["card_index"] = cardIndex,
                    ["target_id"] = targetId
                }
                : null
        });
    }

    private static Dictionary<string, object?> RuntimeV3GameplayObservationValue(
        RuntimeV3GameplayHostObservation observation)
    {
        List<Dictionary<string, object?>> enemies = new();
        foreach (RuntimeV3GameplayEnemyObservation enemy in observation.Enemies)
        {
            enemies.Add(new Dictionary<string, object?>
            {
                ["target_id"] = enemy.TargetId,
                ["alive"] = enemy.Alive,
                ["hittable"] = enemy.Hittable
            });
        }

        return new Dictionary<string, object?>
        {
            ["combat_phase"] = observation.CombatPhase,
            ["turn_index"] = observation.TurnIndex,
            ["host_ready"] = observation.HostReady,
            ["generation"] = observation.Generation,
            ["hand_count"] = observation.HandCount,
            ["energy"] = observation.Energy,
            ["draw_pile_count"] = observation.DrawPileCount,
            ["discard_pile_count"] = observation.DiscardPileCount,
            ["exhaust_pile_count"] = observation.ExhaustPileCount,
            ["enemies"] = enemies
        };
    }

    private static Dictionary<string, string> RuntimeV3GameplayProvenance() => new()
    {
        ["artifact"] = RuntimeV3GameplayArtifact,
        ["source"] = RuntimeV3GameplaySchemaSource,
        ["generator"] = RuntimeV3GameplayGenerator
    };

    private static string RuntimeV3GameplayPlainError(string code) =>
        JsonSerializer.Serialize(new Dictionary<string, string> { ["error_code"] = code });

    private static int RuntimeV3GameplayStatusFor(string status) => status switch
    {
        "rejected" => RuntimeRejected,
        "unknown" => RuntimeUnavailable,
        "dispatching" => RuntimeUnavailable,
        _ => RuntimeAccepted
    };
}
