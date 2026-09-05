// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string RuntimeV2ProtocolVersion = "runtime-v2";
    private const string RuntimeV2Artifact = "sts2-protocol/runtime-v2";
    private const string RuntimeV2SchemaSource = "schemas/runtime-v2.schema.json";
    private const string RuntimeV2Generator = "hand-authored";
    private const string RuntimeV2SchemaDigest =
        "f7963b19c8ed5bbdc02c08e83c7a2e16c4771ed5eb798b29a8208d7a917a86c2";
    private const string RuntimeV2ActionId = "end_turn";
    private const string RuntimeV2EffectKind = "turn_end_settled";
    private const string RuntimeV2OutsideCombat = "outside_combat";
    private const string RuntimeV2PlayerTurn = "combat/player_turn";
    private const string RuntimeV2EnemyTurn = "combat/enemy_turn";
    private const ulong RuntimeV2MaxSafeInteger = 9007199254740991;
    private const ushort RuntimeV2MaxTurnIndex = 1024;
    private const int RuntimeV2OperationCapacity = 64;
    private static readonly string[] RuntimeV2TopLevelFields =
    {
        "protocol_version",
        "schema_digest",
        "provenance",
        "correlation_id",
        "instance_id",
        "session_id",
        "lease_id",
        "lease_epoch",
        "generation",
        "kind",
        "operation_id",
        "observation",
        "action",
        "status",
        "error_code",
        "effect_witness"
    };
    private static readonly HashSet<string> RuntimeV2TopLevelFieldSet =
        new(RuntimeV2TopLevelFields, StringComparer.Ordinal);
    private static readonly string[] RuntimeV2ProvenanceFields = { "artifact", "source", "generator" };
    private static readonly HashSet<string> RuntimeV2ProvenanceFieldSet =
        new(RuntimeV2ProvenanceFields, StringComparer.Ordinal);
    private static readonly string[] RuntimeV2ActionFields = { "action_id" };
    private static readonly HashSet<string> RuntimeV2ActionFieldSet =
        new(RuntimeV2ActionFields, StringComparer.Ordinal);
    private static readonly Dictionary<string, RuntimeV2Operation> RuntimeV2Operations = new(StringComparer.Ordinal);
    private static RuntimeV2Binding? _runtimeV2Binding;
    private static RuntimeV2Operation? _runtimeV2Pending;
    private static bool _runtimeV2HostBaseline;
    private static string _runtimeV2LastPhase = RuntimeV2OutsideCombat;
    private static ushort _runtimeV2LastTurnIndex;

    private sealed class RuntimeV2Binding
    {
        public RuntimeV2Binding(
            string instanceId,
            string callerId,
            string sessionId,
            string leaseId,
            ulong leaseEpoch)
        {
            InstanceId = instanceId;
            CallerId = callerId;
            SessionId = sessionId;
            LeaseId = leaseId;
            LeaseEpoch = leaseEpoch;
        }

        public string InstanceId { get; }
        public string CallerId { get; }
        public string SessionId { get; }
        public string LeaseId { get; }
        public ulong LeaseEpoch { get; }
    }

    private sealed class RuntimeV2Request
    {
        public RuntimeV2Request(string operationId, ulong generation, string canonicalBody)
        {
            OperationId = operationId;
            Generation = generation;
            CanonicalBody = canonicalBody;
        }

        public string OperationId { get; }
        public ulong Generation { get; }
        public string CanonicalBody { get; }
    }

    private sealed class RuntimeV2Operation
    {
        public RuntimeV2Operation(
            string operationId,
            string canonicalBody,
            ulong requestGeneration,
            ushort beforeTurnIndex,
            string status,
            RuntimeV2HostObservation? observation,
            string? errorCode)
        {
            OperationId = operationId;
            CanonicalBody = canonicalBody;
            RequestGeneration = requestGeneration;
            BeforeTurnIndex = beforeTurnIndex;
            Status = status;
            Observation = observation;
            ErrorCode = errorCode;
        }

        public string OperationId { get; }
        public string CanonicalBody { get; }
        public ulong RequestGeneration { get; }
        public ushort BeforeTurnIndex { get; }
        public string Status { get; set; }
        public RuntimeV2HostObservation? Observation { get; set; }
        public string? ErrorCode { get; set; }
    }

    private readonly record struct RuntimeV2HostObservation(
        string CombatPhase,
        ushort TurnIndex,
        bool HostReady,
        ulong Generation);
}
