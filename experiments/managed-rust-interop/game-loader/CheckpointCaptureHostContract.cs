// SPDX-License-Identifier: MIT

using System;
using System.Collections.Immutable;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Managed mirror of <c>CheckpointSettlement</c> (ADR 0055); codes are identical.</summary>
internal enum CheckpointSettlement
{
    Quiescent,
    MidEffect,
    EnemyExecution,
    PendingSelectionTransition,
    Unknown
}

internal static class CheckpointSettlementCodes
{
    internal static string Code(this CheckpointSettlement settlement) => settlement switch
    {
        CheckpointSettlement.Quiescent => "quiescent",
        CheckpointSettlement.MidEffect => "mid_effect",
        CheckpointSettlement.EnemyExecution => "enemy_execution",
        CheckpointSettlement.PendingSelectionTransition => "pending_selection_transition",
        CheckpointSettlement.Unknown => "unknown",
        _ => throw new ArgumentOutOfRangeException(nameof(settlement))
    };
}

internal enum CheckpointHostActionState
{
    None,
    Executing,
    GatheringPlayerChoice,
    Finished,
    Other
}

internal enum CheckpointHostRoomPhase
{
    Unknown,
    MapChoice,
    Combat,
    CombatReward,
    Other
}

internal enum CheckpointHostTurnPhase
{
    None,
    Play,
    End,
    Other
}

/// <summary>
/// Host-thread facts copied from public host members at one instant. Every member is a value;
/// the witness never references a host object, so two reads compare by value.
/// </summary>
internal sealed record CheckpointHostSettlementWitness(
    bool RunInProgress,
    bool RunStateAvailable,
    bool SinglePlayer,
    bool GameOver,
    bool Abandoned,
    bool SavePending,
    bool ActionQueueEmpty,
    bool ActionExecutorRunning,
    bool ActionExecutorPaused,
    CheckpointHostActionState CurrentAction,
    bool ModalOpen,
    CheckpointHostRoomPhase Room,
    bool CombatInProgress,
    bool CombatStarting,
    bool CombatEnding,
    bool CombatPaused,
    bool EnemyTurnStarted,
    bool PlayerTurnEnding,
    bool PlayerActionsDisabled,
    bool EnemySideActive,
    bool AnyMonsterPerformingMove,
    CheckpointHostTurnPhase TurnPhase,
    long TurnNumber);

/// <summary>
/// Read-only host access used by <see cref="CheckpointCaptureSource"/>. The interface exposes no
/// mutation, no RNG access, and no observation-generation access: a reader can only copy.
/// Implementations must be called on the host game thread and must return owned values only.
/// </summary>
internal interface ICheckpointCaptureHostReader
{
    CheckpointHostSettlementWitness ReadSettlement();

    CheckpointPayloadFamilies ReadFamilies(CheckpointCaptureBoundary boundary);
}

/// <summary>Mirrors the subset of <c>CheckpointCaptureRejection</c> a producer can raise.</summary>
internal enum CheckpointCaptureRejectionKind
{
    UnsupportedBoundary,
    UnsafeBoundary,
    Busy,
    UnsupportedCoverage
}

internal static class CheckpointCaptureRejectionCodes
{
    internal static string Code(this CheckpointCaptureRejectionKind kind) => kind switch
    {
        CheckpointCaptureRejectionKind.UnsupportedBoundary => "unsupported_boundary",
        CheckpointCaptureRejectionKind.UnsafeBoundary => "unsafe_boundary",
        CheckpointCaptureRejectionKind.Busy => "busy",
        CheckpointCaptureRejectionKind.UnsupportedCoverage => "unsupported_coverage",
        _ => throw new ArgumentOutOfRangeException(nameof(kind))
    };
}

/// <summary>Typed rejection; <c>Detail</c> is a stable token, never a host value.</summary>
internal sealed record CheckpointCaptureRejection(
    CheckpointCaptureRejectionKind Kind,
    CheckpointCaptureBoundary Boundary,
    CheckpointSettlement Settlement,
    string Detail)
{
    internal string Code => Kind.Code();
}

/// <summary>
/// Either one owned record plus its immutable UTF-8 payload bytes, or a typed rejection with no
/// artifact. Only the bytes are meant to cross a thread boundary.
/// </summary>
internal sealed record CheckpointCaptureOutcome(
    CheckpointPayloadRecord? Record,
    ImmutableArray<byte> PayloadUtf8,
    CheckpointCaptureRejection? Rejection)
{
    internal bool IsCaptured => Record is not null && Rejection is null;

    internal static CheckpointCaptureOutcome Captured(
        CheckpointPayloadRecord record, ImmutableArray<byte> payloadUtf8) =>
        new(record, payloadUtf8, null);

    internal static CheckpointCaptureOutcome Rejected(CheckpointCaptureRejection rejection) =>
        new(null, ImmutableArray<byte>.Empty, rejection);
}
