// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private const string RuntimeV3GameplayProtocolVersion = "runtime-v3-gameplay";
    private const string RuntimeV3GameplayArtifact = "sts2-protocol/runtime-v3-gameplay";
    private const string RuntimeV3GameplaySchemaSource = "schemas/runtime-v3-gameplay.schema.json";
    private const string RuntimeV3GameplayGenerator = "hand-authored";
    private const string RuntimeV3GameplaySchemaDigest =
        "c961bbde893f0422f80233d14ea9ae8b648ee9032136e5370aa5f6b949f6575e";
    private const string RuntimeV3GameplayActionId = "play_card";
    private const string RuntimeV3GameplayEffectKind = "play_card_settled";
    private const string RuntimeV3GameplayOutsideCombat = "outside_combat";
    private const string RuntimeV3GameplayPlayerTurn = "combat/player_turn";
    private const string RuntimeV3GameplayEnemyTurn = "combat/enemy_turn";
    private const ulong RuntimeV3GameplayMaxSafeInteger = 9007199254740991;
    private const ushort RuntimeV3GameplayMaxCardIndex = 64;
    private const ushort RuntimeV3GameplayMaxEnergy = 999;
    private const ushort RuntimeV3GameplayMaxPileCount = 1024;
    private const int RuntimeV3GameplayMaxEnemies = 16;
    private static readonly string[] RuntimeV3GameplayTopLevelFields =
    {
        "protocol_version", "schema_digest", "provenance", "correlation_id", "instance_id",
        "session_id", "lease_id", "lease_epoch", "generation", "kind", "operation_id",
        "observation", "action", "status", "error_code", "effect_witness"
    };
    private static readonly HashSet<string> RuntimeV3GameplayTopLevelFieldSet =
        new(RuntimeV3GameplayTopLevelFields, StringComparer.Ordinal);
    private static readonly string[] RuntimeV3GameplayProvenanceFields = { "artifact", "source", "generator" };
    private static readonly HashSet<string> RuntimeV3GameplayProvenanceFieldSet =
        new(RuntimeV3GameplayProvenanceFields, StringComparer.Ordinal);
    private static readonly string[] RuntimeV3GameplayActionFields = { "action_id", "card_index", "target_id" };
    private static readonly HashSet<string> RuntimeV3GameplayActionFieldSet =
        new(RuntimeV3GameplayActionFields, StringComparer.Ordinal);
    private static readonly Dictionary<string, RuntimeV3GameplayOperation> RuntimeV3GameplayOperations = new(StringComparer.Ordinal);
    private static RuntimeV3GameplayBinding? _runtimeV3GameplayBinding;
    private static RuntimeV3GameplayOperation? _runtimeV3GameplayPending;
    private static bool _runtimeV3GameplayBaseline;
    private static string _runtimeV3GameplayLastSignature = String.Empty;
    private static ulong _runtimeV3GameplayGeneration;

    private sealed class RuntimeV3GameplayBinding
    {
        public RuntimeV3GameplayBinding(string instanceId, string callerId, string sessionId, string leaseId, ulong leaseEpoch)
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

    private sealed class RuntimeV3GameplayRequest
    {
        public RuntimeV3GameplayRequest(string operationId, ulong generation, ushort cardIndex, string? targetId, string canonicalBody)
        {
            OperationId = operationId;
            Generation = generation;
            CardIndex = cardIndex;
            TargetId = targetId;
            CanonicalBody = canonicalBody;
        }

        public string OperationId { get; }
        public ulong Generation { get; }
        public ushort CardIndex { get; }
        public string? TargetId { get; }
        public string CanonicalBody { get; }
    }

    private sealed class RuntimeV3GameplayOperation
    {
        public RuntimeV3GameplayOperation(
            RuntimeV3GameplayRequest request,
            RuntimeV3GameplayHostObservation before,
            string status,
            RuntimeV3GameplayHostObservation? observation,
            string? errorCode)
        {
            OperationId = request.OperationId;
            CanonicalBody = request.CanonicalBody;
            RequestGeneration = request.Generation;
            CardIndex = request.CardIndex;
            TargetId = request.TargetId;
            Before = before;
            Status = status;
            Observation = observation;
            ErrorCode = errorCode;
        }

        public string OperationId { get; }
        public string CanonicalBody { get; }
        public ulong RequestGeneration { get; }
        public ushort CardIndex { get; }
        public string? TargetId { get; }
        public RuntimeV3GameplayHostObservation Before { get; }
        public string Status { get; set; }
        public RuntimeV3GameplayHostObservation? Observation { get; set; }
        public string? ErrorCode { get; set; }
    }

    private readonly record struct RuntimeV3GameplayEnemyObservation(
        string TargetId,
        bool Alive,
        bool Hittable);

    private readonly record struct RuntimeV3GameplayHostObservation(
        string CombatPhase,
        ushort TurnIndex,
        bool HostReady,
        ulong Generation,
        ushort HandCount,
        ushort Energy,
        ushort DrawPileCount,
        ushort DiscardPileCount,
        ushort ExhaustPileCount,
        IReadOnlyList<RuntimeV3GameplayEnemyObservation> Enemies,
        string Signature);
}
