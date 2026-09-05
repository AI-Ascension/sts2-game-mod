// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Text.Json;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>Opt-in single-player combat projection from the installed host.</summary>
internal sealed partial class LiveCombatSource : IRuntimeV3HostSource, IRuntimeV3HostThread
{
    private readonly int _threadId = Environment.CurrentManagedThreadId;
    private readonly Dictionary<CardModel, string> _cardIds = new();
    private string? _fingerprint;
    private ulong _generation;
    private int _nextCardId;

    public void Enqueue(Action work)
    {
        RequireThread();
        work();
    }

    private void RequireThread()
    {
        if (Environment.CurrentManagedThreadId != _threadId)
            throw new InvalidOperationException("live combat requires the host thread");
    }

    private static Player? CurrentPlayer()
    {
        RunManager manager = RunManager.Instance;
        if (!manager.IsInProgress) return null;
        RunState? state = manager.DebugOnlyGetState();
        if (state == null || state.Players.Count != 1 || manager.NetService.Type !=
            MegaCrit.Sts2.Core.Multiplayer.Game.NetGameType.Singleplayer) return null;
        return LocalContext.GetMe(state);
    }

    public RuntimeV3GameplayObservation Observe()
    {
        RequireThread();
        Player? player = CurrentPlayer();
        PlayerCombatState? combat = player?.PlayerCombatState;
        CombatManager manager = CombatManager.Instance;
        CombatState? hostCombat = manager.DebugOnlyGetState();
        bool active = combat != null && hostCombat != null && manager.IsInProgress;
        RuntimeV3GameplayPlayer projection = ProjectPlayer(player, combat);
        var enemies = active && hostCombat != null ? hostCombat.Enemies.Select(ProjectEnemy).ToArray()
            : Array.Empty<RuntimeV3GameplayEnemy>();
        bool modal = MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal != null;
        bool enabled = active && combat?.Phase == PlayerTurnPhase.Play && !modal
            && !manager.IsEnemyTurnStarted && !manager.PlayerActionsDisabled;
        bool rewards = RunManager.Instance.DebugOnlyGetState()?.CurrentRoom
            is MegaCrit.Sts2.Core.Rooms.CombatRoom { IsPreFinished: true };
        var state = player?.Creature.IsDead == true ? RuntimeV3GameplayState.Defeat
            : rewards ? RuntimeV3GameplayState.Reward
            : active ? RuntimeV3GameplayState.Combat : RuntimeV3GameplayState.Recovery;
        string[] values = state == RuntimeV3GameplayState.Recovery ? new[] { "outside_combat" }
            : Array.Empty<string>();
        string? seed = player == null ? null : RunManager.Instance.DebugOnlyGetState()?.Rng.StringSeed;
        var result = new RuntimeV3GameplayObservation("live", 0, seed, projection, state, values, enemies)
        {
            TurnIndex = (ushort)Math.Clamp(combat?.TurnNumber ?? 0, 0, 1024),
            IsActionable = enabled, InputEnabled = enabled, ModalBlocking = !enabled
        };
        string fingerprint = JsonSerializer.Serialize(result);
        if (_fingerprint != fingerprint) { _generation++; _fingerprint = fingerprint; }
        return result with { StateId = $"live:{_generation}", Generation = _generation };
    }

    private RuntimeV3GameplayPlayer ProjectPlayer(Player? player, PlayerCombatState? combat) => new(
        U16(player?.Creature.CurrentHp ?? 0), U16(player?.Creature.MaxHp ?? 0),
        (byte)Math.Clamp(combat?.Energy ?? 0, 0, 255), (uint)Math.Max(player?.Gold ?? 0, 0),
        ProjectCards(combat?.Hand.Cards), ProjectCards(player?.Deck.Cards),
        ProjectCards(combat?.DiscardPile.Cards), ProjectCards(combat?.ExhaustPile.Cards));

    private RuntimeV3GameplayCard[] ProjectCards(IEnumerable<CardModel>? cards) =>
        cards?.Select(card => new RuntimeV3GameplayCard(CardId(card), card.Title,
            (byte)Math.Clamp(card.EnergyCost.GetResolved(), 0, 255), card.IsUpgraded)).ToArray()
        ?? Array.Empty<RuntimeV3GameplayCard>();

    private string CardId(CardModel card)
    {
        if (!_cardIds.TryGetValue(card, out string? id))
        {
            id = $"card:{++_nextCardId}";
            _cardIds.Add(card, id);
        }
        return id;
    }

    private static ushort U16(int value) => (ushort)Math.Clamp(value, 0, ushort.MaxValue);
    private static string EnemyId(Creature enemy) => $"enemy:{enemy.CombatId}";
    private static RuntimeV3GameplayEnemy ProjectEnemy(Creature enemy) => new(
        EnemyId(enemy), enemy.Name, U16(enemy.CurrentHp), U16(enemy.MaxHp),
        RuntimeV3GameplayIntent.Unknown, 0, 0);

    public IReadOnlyList<LegalActionReference> LegalActions(RuntimeV3GameplayObservation observation)
    {
        RequireThread();
        var actions = new List<LegalActionReference>();
        Player? player = CurrentPlayer();
        if (!observation.InputEnabled || player?.PlayerCombatState is not { } combat) return actions;
        foreach (CardModel card in combat.Hand.Cards)
        {
            if (!card.CanPlay()) continue;
            if (card.TargetType == TargetType.AnyEnemy)
            {
                foreach (Creature enemy in CombatManager.Instance.DebugOnlyGetState()?.Enemies
                    ?? Array.Empty<Creature>())
                    if (card.CanPlayTargeting(enemy)) AddPlay(actions, card, enemy, observation.Generation);
            }
            else if (card.TargetType is TargetType.Self or TargetType.None or TargetType.AllEnemies)
            {
                Creature? target = null;
                if (card.CanPlayTargeting(target)) AddPlay(actions, card, target, observation.Generation);
            }
        }
        actions.Add(new($"end:{observation.Generation}", "end_turn", null, null, observation.Generation));
        return actions;
    }

    private void AddPlay(List<LegalActionReference> actions, CardModel card, Creature? target, ulong generation)
    {
        string? targetId = target?.IsEnemy == true ? EnemyId(target) : null;
        string cardId = CardId(card);
        actions.Add(new($"play:{generation}:{cardId}:{targetId ?? "none"}",
            "play_card", cardId, targetId, generation));
    }
}
