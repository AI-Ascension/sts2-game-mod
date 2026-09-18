// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>
/// Mutable stand-in for host state. It is deliberately a reference type with mutable fields so a
/// test can prove the captured record is a copy. It is not a host assembly and proves nothing
/// about native behavior.
/// </summary>
internal sealed class SyntheticHostState
{
    internal CheckpointHostSettlementWitness Witness = SyntheticHost.QuiescentCombat(turn: 1);
    internal string Seed = "SYNTHETIC-SEED-0001";
    internal ulong ShuffleCursor = 3;
    internal int Hp = 80;
    internal int Gold = 99;
    internal List<string> Hand = new() { "card:1", "card:2", "card:5" };
    internal List<string> Draw = new() { "card:3", "card:4" };
    internal ulong ObservationGeneration = 42;
    internal ulong RngDraws = 7;
}

/// <summary>Synthetic reader; mirrors the pinned fixture values when left untouched.</summary>
internal sealed class SyntheticHost : ICheckpointCaptureHostReader
{
    internal SyntheticHostState State { get; } = new();

    internal int SettlementReads { get; private set; }

    internal Action? DuringFamilyRead { get; set; }

    internal CheckpointPayloadFamilies? Override { get; set; }

    public CheckpointHostSettlementWitness ReadSettlement()
    {
        SettlementReads++;
        return State.Witness;
    }

    public CheckpointPayloadFamilies ReadFamilies(CheckpointCaptureBoundary boundary)
    {
        DuringFamilyRead?.Invoke();
        if (Override is { } families)
            return families;
        bool combat = boundary != CheckpointCaptureBoundary.SettledMapChoice;
        var streams = new[]
        {
            new CheckpointPayloadRngStream("synthetic.shuffle", "synthetic-counter-v1",
                State.ShuffleCursor, ShuffleStreamTail),
            new CheckpointPayloadRngStream("synthetic.enemy_intent", "synthetic-counter-v1", 1,
                EnemyIntentStreamTail)
        };
        var cards = new[]
        {
            Card("card:1", "synthetic.card.strike", 0), Card("card:2", "synthetic.card.strike", 1),
            Card("card:3", "synthetic.card.defend", 0), Card("card:4", "synthetic.card.defend", 0),
            Card("card:5", "synthetic.card.bash", 0)
        };
        var piles = combat
            ? new[]
            {
                new CheckpointPayloadCardPile("draw", State.Draw.ToArray()),
                new CheckpointPayloadCardPile("hand", State.Hand.ToArray()),
                new CheckpointPayloadCardPile("discard", Array.Empty<string>()),
                new CheckpointPayloadCardPile("exhaust", Array.Empty<string>())
            }
            : Array.Empty<CheckpointPayloadCardPile>();
        return new CheckpointPayloadFamilies(
            Captured(new CheckpointPayloadSeedAndRng(State.Seed, "synthetic-derivation-v1", streams)),
            Captured(new CheckpointPayloadRunConfiguration("synthetic.character.a", 0, "standard",
                ActIdentities,
                Array.Empty<string>())),
            Captured(new CheckpointPayloadCampaignProgress(0, 1, "node:0:1:1",
                combat ? "monster" : "unknown_until_entered", NodePath)),
            Captured(new CheckpointPayloadPlayerResources(State.Hp, 80, State.Gold,
                Array.Empty<string>())),
            Captured(new CheckpointPayloadDeckAndPiles(
                DeckCardIdentities, piles)),
            Captured(new CheckpointPayloadCardInstances(cards)),
            Captured(new CheckpointPayloadRelics(
                new[] { new CheckpointPayloadRelic("synthetic.relic.starter", -1) })),
            Captured(new CheckpointPayloadPotions(3, Array.Empty<CheckpointPayloadPotionSlot>())),
            combat
                ? Captured(new CheckpointPayloadCombatTurn(State.Witness.TurnNumber, 3, 0))
                : CheckpointPayloadFamily<CheckpointPayloadCombatTurn>.NotApplicable(),
            combat
                ? Captured(new CheckpointPayloadPowers(Array.Empty<CheckpointPayloadPower>()))
                : CheckpointPayloadFamily<CheckpointPayloadPowers>.NotApplicable(),
            combat
                ? Captured(new CheckpointPayloadEnemiesAndIntents(new[]
                {
                    new CheckpointPayloadEnemy("enemy:1", 44, 44, 0,
                        new[] { new CheckpointPayloadIntent("synthetic.intent.attack", 11) })
                }))
                : CheckpointPayloadFamily<CheckpointPayloadEnemiesAndIntents>.NotApplicable(),
            Captured(new CheckpointPayloadPendingEffects(Array.Empty<string>(), new[]
            {
                new CheckpointPayloadExternalInput("wall_clock", "controlled"),
                new CheckpointPayloadExternalInput("network", "absent")
            })));
    }

    internal static CheckpointPayloadFamily<T> Captured<T>(T value) where T : class =>
        CheckpointPayloadFamily<T>.Captured(value);

    private static readonly string[] ActIdentities =
        { "synthetic.act.1", "synthetic.act.2", "synthetic.act.3" };

    private static readonly string[] NodePath = { "node:0:1:1" };

    private static readonly string[] DeckCardIdentities =
        { "card:1", "card:2", "card:3", "card:4", "card:5" };

    private static readonly ulong[] ShuffleStreamTail = { ulong.MaxValue, 42UL };

    private static readonly ulong[] EnemyIntentStreamTail = { 9007199254740993UL };

    private static CheckpointPayloadCardInstance Card(string id, string definition, long upgrade) =>
        new(id, definition, upgrade, Array.Empty<CheckpointPayloadTemporaryValue>());

    internal static CheckpointHostSettlementWitness QuiescentCombat(long turn) => new(
        RunInProgress: true, RunStateAvailable: true, SinglePlayer: true, GameOver: false,
        Abandoned: false, SavePending: false, ActionQueueEmpty: true,
        ActionExecutorRunning: false, ActionExecutorPaused: false,
        CurrentAction: CheckpointHostActionState.None, ModalOpen: false,
        Room: CheckpointHostRoomPhase.Combat, CombatInProgress: true, CombatStarting: false,
        CombatEnding: false, CombatPaused: false, EnemyTurnStarted: false,
        PlayerTurnEnding: false, PlayerActionsDisabled: false, EnemySideActive: false,
        AnyMonsterPerformingMove: false, TurnPhase: CheckpointHostTurnPhase.Play,
        TurnNumber: turn);

    internal static CheckpointHostSettlementWitness QuiescentMapChoice() =>
        QuiescentCombat(0) with
        {
            Room = CheckpointHostRoomPhase.MapChoice,
            CombatInProgress = false,
            TurnPhase = CheckpointHostTurnPhase.None
        };
}
