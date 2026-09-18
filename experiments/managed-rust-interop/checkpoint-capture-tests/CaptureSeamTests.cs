// SPDX-License-Identifier: MIT

using System;
using System.Collections;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Reflection;
using System.Text;
using System.Threading;

namespace AiAscension.Sts2GameMod.Runtime;

internal static class CaptureSeamTests
{
    private const CheckpointCaptureBoundary Combat = CheckpointCaptureBoundary.StablePlayerTurnCombat;

    private static readonly string[] OutstandingEffectIdentities = { "synthetic.effect" };

    private static readonly string[] DanglingCardIdentities = { "card:404" };

    internal static void CaptureRefusesOffHostThread()
    {
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        Exception? failure = null;
        CheckpointCaptureOutcome? outcome = null;
        var thread = new Thread(() =>
        {
            try
            {
                outcome = source.Capture(Combat);
            }
            catch (InvalidOperationException exception)
            {
                failure = exception;
            }
        });
        thread.Start();
        thread.Join();
        Program.Check(failure is not null && outcome is null && host.SettlementReads == 0,
            "capture off the host thread is refused before any host read");
        Program.Check(source.Capture(Combat).IsCaptured,
            "the same source still captures on its host thread");
    }

    internal static void CaptureRefusesMidEffectAndEnemyExecutionSettlement()
    {
        Expect(w => w with { ActionExecutorRunning = true }, CheckpointSettlement.MidEffect,
            "action_executing");
        Expect(w => w with { CurrentAction = CheckpointHostActionState.Executing },
            CheckpointSettlement.MidEffect, "action_executing");
        Expect(w => w with { ActionQueueEmpty = false }, CheckpointSettlement.MidEffect,
            "action_queue_pending");
        Expect(w => w with { SavePending = true }, CheckpointSettlement.MidEffect,
            "run_save_pending");
        Expect(w => w with { PlayerTurnEnding = true }, CheckpointSettlement.MidEffect,
            "player_turn_ending");
        Expect(w => w with { EnemyTurnStarted = true }, CheckpointSettlement.EnemyExecution,
            "enemy_turn_started");
        Expect(w => w with { EnemySideActive = true }, CheckpointSettlement.EnemyExecution,
            "enemy_side_active");
        Expect(w => w with { AnyMonsterPerformingMove = true },
            CheckpointSettlement.EnemyExecution, "monster_performing_move");
        Expect(w => w with { EnemyTurnStarted = true, ActionExecutorRunning = true },
            CheckpointSettlement.EnemyExecution, "enemy_turn_started");
        Expect(w => w with { RunInProgress = false }, CheckpointSettlement.Unknown,
            "run_not_active");
        Expect(w => w with { SinglePlayer = false }, CheckpointSettlement.Unknown,
            "not_single_player");
        Expect(w => w with { Room = CheckpointHostRoomPhase.CombatReward, CombatInProgress = false },
            CheckpointSettlement.Unknown, "host_phase_mismatch:combat_not_active");
        Expect(w => w with { TurnPhase = CheckpointHostTurnPhase.End }, CheckpointSettlement.MidEffect,
            "player_turn_ending");
        Expect(w => w with { TurnNumber = 1 }, CheckpointSettlement.Unknown,
            "turn_witness_below_minimum", CheckpointCaptureBoundary.LaterTurnCombat);
        Expect(w => w with { Room = CheckpointHostRoomPhase.Combat, CombatInProgress = true },
            CheckpointSettlement.Unknown, "host_phase_mismatch:combat_active",
            CheckpointCaptureBoundary.SettledMapChoice, SyntheticHost.QuiescentMapChoice());
    }

    internal static void CaptureRefusesPendingSelectionTransition()
    {
        Expect(w => w with { CurrentAction = CheckpointHostActionState.GatheringPlayerChoice },
            CheckpointSettlement.PendingSelectionTransition, "player_choice_gathering");
        Expect(w => w with { ModalOpen = true }, CheckpointSettlement.PendingSelectionTransition,
            "modal_open");
        Expect(w => w with { ModalOpen = true, EnemyTurnStarted = true },
            CheckpointSettlement.PendingSelectionTransition, "modal_open");
        Expect(w => w with { ModalOpen = true }, CheckpointSettlement.PendingSelectionTransition,
            "modal_open", CheckpointCaptureBoundary.SettledMapChoice,
            SyntheticHost.QuiescentMapChoice());
    }

    internal static void CaptureRefusesSettlementChangeAndReentryInsideTheWindow()
    {
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        host.DuringFamilyRead = () => host.State.Witness =
            host.State.Witness with { ActionQueueEmpty = false };
        CheckpointCaptureOutcome changed = source.Capture(Combat);
        Program.Check(!changed.IsCaptured && changed.PayloadUtf8.IsEmpty
            && changed.Rejection is { Kind: CheckpointCaptureRejectionKind.UnsafeBoundary,
                Detail: "settlement_changed_inside_window" },
            "a settlement change between validation and snapshot rejects without an artifact");

        host = new SyntheticHost();
        CheckpointCaptureSource reentrant = new(host);
        CheckpointCaptureOutcome? nested = null;
        host.DuringFamilyRead = () => nested = reentrant.Capture(Combat);
        CheckpointCaptureOutcome outer = reentrant.Capture(Combat);
        Program.Check(nested is { IsCaptured: false, Rejection.Kind: CheckpointCaptureRejectionKind.Busy }
            && outer.IsCaptured,
            "a nested capture inside the window is busy and the outer capture completes");
        Program.Check(reentrant.Capture(Combat).IsCaptured && host.SettlementReads == 4,
            "the window is released after the outer capture");
    }

    internal static void CaptureCopiesOwnedBytesAndReleasesHostReferences()
    {
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        CheckpointCaptureOutcome outcome = source.Capture(Combat);
        Program.Check(outcome.IsCaptured && outcome.Record is not null,
            "a quiescent combat boundary captures");
        ImmutableArray<byte> bytes = outcome.PayloadUtf8;
        string text = Encoding.UTF8.GetString(bytes.AsSpan());
        Program.Check(text.Contains("\"seed_and_rng\":{\"coverage\":\"captured\"", StringComparison.Ordinal)
            && text.Contains("\"cursor\":{\"kind\":\"uint64\",\"value\":\"3\"}", StringComparison.Ordinal),
            "payload bytes carry the captured families in the schema shape");

        host.State.Seed = "MUTATED-AFTER-CAPTURE";
        host.State.ShuffleCursor = 99;
        host.State.Hand.Clear();
        host.State.Hp = 1;
        Program.Check(text == Encoding.UTF8.GetString(outcome.PayloadUtf8.AsSpan())
            && outcome.Record!.Families.SeedAndRng.Value!.MasterSeed == "SYNTHETIC-SEED-0001"
            && outcome.Record.Families.DeckAndPiles.Value!.Piles[1].Cards.Count == 3
            && outcome.Record.Families.PlayerResources.Value!.CurrentHp == 80,
            "mutating host state after capture changes neither the record nor the bytes");

        var visited = new HashSet<object>(ReferenceEqualityComparer.Instance);
        Program.Check(!ReferencesHost(outcome, visited) && !ReferencesHost(outcome.Record, visited),
            "the outcome graph holds no synthetic host object");
        Program.Check(!ReferencesHost(source, new HashSet<object>(ReferenceEqualityComparer.Instance),
                allowReader: true),
            "the source retains only its reader, never a host state object");
    }

    internal static void CaptureDoesNotAdvanceObservationGenerationOrRng()
    {
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        ulong generation = host.State.ObservationGeneration;
        ulong draws = host.State.RngDraws;
        CheckpointCaptureOutcome first = source.Capture(Combat);
        CheckpointCaptureOutcome second = source.Capture(Combat);
        Program.Check(first.IsCaptured && second.IsCaptured
            && host.State.ObservationGeneration == generation && host.State.RngDraws == draws,
            "two captures leave the observation generation and RNG draw count untouched");
        Program.Check(first.PayloadUtf8.AsSpan().SequenceEqual(second.PayloadUtf8.AsSpan())
            && !ReferenceEquals(first.Record, second.Record),
            "the same quiescent boundary yields identical bytes and distinct occurrences");
        host.State.ShuffleCursor++;
        CheckpointCaptureOutcome advanced = source.Capture(Combat);
        Program.Check(!advanced.PayloadUtf8.AsSpan().SequenceEqual(first.PayloadUtf8.AsSpan()),
            "an advanced RNG cursor changes the bytes while the public projection is unchanged");
        Program.Check(typeof(ICheckpointCaptureHostReader).GetMethods().Length == 2,
            "the reader contract exposes copy-only reads");
    }

    internal static void CaptureRejectsRequiredUnknownAndMisplacedFamilies()
    {
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        CheckpointPayloadFamilies families = host.ReadFamilies(Combat);
        host.Override = families with
        {
            SeedAndRng = CheckpointPayloadFamily<CheckpointPayloadSeedAndRng>.Unknown()
        };
        Reject(source.Capture(Combat), CheckpointCaptureRejectionKind.UnsupportedCoverage,
            "required_family_unknown:seed_and_rng", "a required unknown family is refused");

        host.Override = null;
        host.State.Witness = SyntheticHost.QuiescentMapChoice();
        CheckpointPayloadFamilies mapChoice =
            host.ReadFamilies(CheckpointCaptureBoundary.SettledMapChoice);
        host.Override = mapChoice with { CombatTurn = families.CombatTurn };
        Reject(source.Capture(CheckpointCaptureBoundary.SettledMapChoice),
            CheckpointCaptureRejectionKind.UnsupportedCoverage, "family_not_applicable:combat_turn",
            "a combat family at a settled map choice is refused");
        host.Override = null;
        Program.Check(source.Capture(CheckpointCaptureBoundary.SettledMapChoice).IsCaptured,
            "the settled map choice captures with the nine non-combat families");

        host.State.Witness = SyntheticHost.QuiescentCombat(turn: 1);
        host.Override = families with
        {
            PendingEffects = SyntheticHost.Captured(new CheckpointPayloadPendingEffects(
                OutstandingEffectIdentities, Array.Empty<CheckpointPayloadExternalInput>()))
        };
        Reject(source.Capture(Combat), CheckpointCaptureRejectionKind.UnsafeBoundary,
            "pending_effects_outstanding:pending_effects", "an outstanding pending effect is unsafe");
        host.Override = families with
        {
            DeckAndPiles = SyntheticHost.Captured(new CheckpointPayloadDeckAndPiles(
                DanglingCardIdentities, Array.Empty<CheckpointPayloadCardPile>()))
        };
        Reject(source.Capture(Combat), CheckpointCaptureRejectionKind.UnsupportedCoverage,
            "unresolved_card_reference:deck_and_piles", "a dangling card reference is refused");
        host.Override = families with
        {
            Relics = SyntheticHost.Captured(new CheckpointPayloadRelics(
                new[] { new CheckpointPayloadRelic("bad relic id", 0) }))
        };
        Reject(source.Capture(Combat), CheckpointCaptureRejectionKind.UnsupportedCoverage,
            "identifier_invalid:relics", "a malformed identifier is refused");
    }

    internal static void CaptureRejectsBoundViolationsAndHostReadFailures()
    {
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        CheckpointPayloadFamilies families = host.ReadFamilies(Combat);
        host.Override = families with
        {
            PendingEffects = SyntheticHost.Captured(
                new CheckpointPayloadPendingEffects(null!, Array.Empty<CheckpointPayloadExternalInput>()))
        };
        Reject(source.Capture(Combat), CheckpointCaptureRejectionKind.UnsupportedCoverage,
            "bound_violated:pending_effects", "an unbounded collection is refused");

        host.Override = families with
        {
            SeedAndRng = SyntheticHost.Captured(new CheckpointPayloadSeedAndRng(
                "SYNTHETIC-SEED-0001", "synthetic-derivation-v1",
                new[]
                {
                    new CheckpointPayloadRngStream("synthetic.shuffle", "synthetic-counter-v1", 3,
                        SyntheticHost.ShuffleStreamTail),
                    new CheckpointPayloadRngStream("synthetic.shuffle", "synthetic-counter-v1", 4,
                        SyntheticHost.ShuffleStreamTail)
                }))
        };
        Reject(source.Capture(Combat), CheckpointCaptureRejectionKind.UnsupportedCoverage,
            "duplicate_stream_id:seed_and_rng", "a duplicate RNG stream id is refused");

        host.Override = null;
        Reject(new CheckpointCaptureSource(new ThrowingHost(new InvalidOperationException()))
            .Capture(Combat), CheckpointCaptureRejectionKind.UnsafeBoundary, "host_read_failed",
            "a host read that throws is a typed refusal, not an escaped exception");
        Reject(new CheckpointCaptureSource(new ThrowingHost(new FormatException("host read")))
            .Capture(Combat), CheckpointCaptureRejectionKind.UnsafeBoundary, "host_read_failed",
            "every host-read exception class is a typed refusal without an artifact");
    }

    internal static void EveryAdvertisedPhaseStillReportsUnavailable()
    {
        IReadOnlyList<CheckpointCaptureCapability> capabilities = CheckpointCaptureSource.Capabilities();
        Program.Check(capabilities.Count == 11, "the matrix has eleven boundaries");
        var host = new SyntheticHost();
        var source = new CheckpointCaptureSource(host);
        foreach (CheckpointCaptureCapability capability in capabilities)
        {
            Program.Check(!capability.IsAvailable, "no phase is advertised: " + capability.Boundary.Code());
            if (capability.HasPayloadSchema)
                continue;
            CheckpointCaptureOutcome outcome = source.Capture(capability.Boundary);
            bool unsafeBoundary = capability.Reason == CheckpointUnavailableReason.UnsafeBoundary;
            Program.Check(!outcome.IsCaptured && outcome.PayloadUtf8.IsEmpty && outcome.Rejection is { } r
                && r.Kind == (unsafeBoundary ? CheckpointCaptureRejectionKind.UnsafeBoundary
                    : CheckpointCaptureRejectionKind.UnsupportedBoundary)
                && r.Detail == capability.Reason.Code(),
                "unsupported boundary rejects without artifacts: " + capability.Boundary.Code());
        }
        Program.Check(host.SettlementReads == 0, "unsupported boundaries never read the host");
    }

    private static void Expect(
        Func<CheckpointHostSettlementWitness, CheckpointHostSettlementWitness> mutate,
        CheckpointSettlement settlement,
        string detail,
        CheckpointCaptureBoundary boundary = Combat,
        CheckpointHostSettlementWitness? baseline = null)
    {
        var host = new SyntheticHost();
        host.State.Witness = mutate(baseline ?? SyntheticHost.QuiescentCombat(turn: 2));
        CheckpointCaptureOutcome outcome = new CheckpointCaptureSource(host).Capture(boundary);
        Program.Check(!outcome.IsCaptured && outcome.Record is null && outcome.PayloadUtf8.IsEmpty
            && outcome.Rejection is { } rejection
            && rejection.Kind == CheckpointCaptureRejectionKind.UnsafeBoundary
            && rejection.Settlement == settlement && rejection.Detail == detail
            && host.SettlementReads == 1,
            settlement.Code() + " refuses with " + detail);
    }

    private static void Reject(
        CheckpointCaptureOutcome outcome, CheckpointCaptureRejectionKind kind, string detail,
        string message) =>
        Program.Check(!outcome.IsCaptured && outcome.PayloadUtf8.IsEmpty
            && outcome.Rejection is { } rejection && rejection.Kind == kind
            && rejection.Detail == detail, message);

    private static bool ReferencesHost(object? value, HashSet<object> visited, bool allowReader = false)
    {
        if (value is null || value is string || value.GetType().IsPrimitive || value is Enum
            || !visited.Add(value))
            return false;
        if (value is SyntheticHostState)
            return true;
        if (value is SyntheticHost)
            return !allowReader;
        if (value is IEnumerable sequence)
        {
            foreach (object? item in sequence)
                if (ReferencesHost(item, visited, allowReader))
                    return true;
            return false;
        }
        foreach (FieldInfo field in value.GetType().GetFields(
            BindingFlags.Instance | BindingFlags.Public | BindingFlags.NonPublic))
            if (ReferencesHost(field.GetValue(value), visited, allowReader))
                return true;
        return false;
    }
}
