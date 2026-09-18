// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Managed mirror of the owner boundary matrix in
/// <c>crates/game-mod/src/checkpoint/capability.rs</c>. Codes are identical so a managed
/// rejection maps one-to-one onto the Rust typed rejection.
/// </summary>
internal enum CheckpointCaptureBoundary
{
    SettledMapChoice,
    StablePlayerTurnCombat,
    LaterTurnCombat,
    RewardCardSelection,
    Event,
    Shop,
    Rest,
    EnemyTurn,
    Animation,
    Transition,
    Unknown
}

/// <summary>Why a boundary is not advertised as capture-capable (mirrors the Rust enum).</summary>
internal enum CheckpointUnavailableReason
{
    ExactHostEvidenceRequired,
    CoverageInventoryPending,
    UnsafeBoundary
}

/// <summary>
/// Fail-closed capability descriptor for one boundary. No managed boundary is advertised: the
/// seam exists so an authorized exact-host run can produce evidence, not to claim it.
/// </summary>
internal readonly record struct CheckpointCaptureCapability(
    CheckpointCaptureBoundary Boundary,
    CheckpointUnavailableReason Reason)
{
    /// <summary>A boundary is available only once no gate remains; every reason is a gate.</summary>
    internal bool IsAvailable => Reason is not (CheckpointUnavailableReason.ExactHostEvidenceRequired
        or CheckpointUnavailableReason.CoverageInventoryPending
        or CheckpointUnavailableReason.UnsafeBoundary);

    /// <summary>Only the three first-release boundaries have a <c>checkpoint-payload-v1</c> schema.</summary>
    internal bool HasPayloadSchema => CheckpointCaptureBoundaries.IsFirstRelease(Boundary);
}

internal static class CheckpointCaptureBoundaries
{
    internal static readonly IReadOnlyList<CheckpointCaptureBoundary> All = new[]
    {
        CheckpointCaptureBoundary.SettledMapChoice,
        CheckpointCaptureBoundary.StablePlayerTurnCombat,
        CheckpointCaptureBoundary.LaterTurnCombat,
        CheckpointCaptureBoundary.RewardCardSelection,
        CheckpointCaptureBoundary.Event,
        CheckpointCaptureBoundary.Shop,
        CheckpointCaptureBoundary.Rest,
        CheckpointCaptureBoundary.EnemyTurn,
        CheckpointCaptureBoundary.Animation,
        CheckpointCaptureBoundary.Transition,
        CheckpointCaptureBoundary.Unknown
    };

    internal static string Code(this CheckpointCaptureBoundary boundary) => boundary switch
    {
        CheckpointCaptureBoundary.SettledMapChoice => "settled_map_choice",
        CheckpointCaptureBoundary.StablePlayerTurnCombat => "stable_player_turn_combat",
        CheckpointCaptureBoundary.LaterTurnCombat => "later_turn_combat",
        CheckpointCaptureBoundary.RewardCardSelection => "reward_card_selection",
        CheckpointCaptureBoundary.Event => "event",
        CheckpointCaptureBoundary.Shop => "shop",
        CheckpointCaptureBoundary.Rest => "rest",
        CheckpointCaptureBoundary.EnemyTurn => "enemy_turn",
        CheckpointCaptureBoundary.Animation => "animation",
        CheckpointCaptureBoundary.Transition => "transition",
        CheckpointCaptureBoundary.Unknown => "unknown",
        _ => throw new ArgumentOutOfRangeException(nameof(boundary))
    };

    internal static string Code(this CheckpointUnavailableReason reason) => reason switch
    {
        CheckpointUnavailableReason.ExactHostEvidenceRequired => "exact_host_evidence_required",
        CheckpointUnavailableReason.CoverageInventoryPending => "coverage_inventory_pending",
        CheckpointUnavailableReason.UnsafeBoundary => "unsafe_boundary",
        _ => throw new ArgumentOutOfRangeException(nameof(reason))
    };

    /// <summary>The three boundaries with a closed payload schema (ADR 0057).</summary>
    internal static bool IsFirstRelease(CheckpointCaptureBoundary boundary) => boundary is
        CheckpointCaptureBoundary.SettledMapChoice
        or CheckpointCaptureBoundary.StablePlayerTurnCombat
        or CheckpointCaptureBoundary.LaterTurnCombat;

    /// <summary>Same disposition table as <c>CheckpointCapabilities::for_boundary</c>.</summary>
    internal static CheckpointCaptureCapability Capability(CheckpointCaptureBoundary boundary)
    {
        CheckpointUnavailableReason reason = boundary switch
        {
            CheckpointCaptureBoundary.SettledMapChoice
                or CheckpointCaptureBoundary.StablePlayerTurnCombat
                or CheckpointCaptureBoundary.LaterTurnCombat =>
                CheckpointUnavailableReason.ExactHostEvidenceRequired,
            CheckpointCaptureBoundary.RewardCardSelection
                or CheckpointCaptureBoundary.Event
                or CheckpointCaptureBoundary.Shop
                or CheckpointCaptureBoundary.Rest =>
                CheckpointUnavailableReason.CoverageInventoryPending,
            _ => CheckpointUnavailableReason.UnsafeBoundary
        };
        return new CheckpointCaptureCapability(boundary, reason);
    }
}
