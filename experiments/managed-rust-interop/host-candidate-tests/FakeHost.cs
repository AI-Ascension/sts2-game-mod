// SPDX-License-Identifier: MIT

// Handwritten synthetic host surface; not copied from or evidence of a proprietary host API.
namespace Godot
{
    internal sealed class SceneTree { public object Root { get; } = new(); }
    internal static class Engine { public static object GetMainLoop() => new SceneTree(); }
    internal static class GD
    {
        public static void Print(string value) { }
        public static void PrintErr(string value) { }
    }
}

namespace MegaCrit.Sts2.Core.Entities.Creatures
{
    internal sealed class Creature
    {
        public uint? CombatId { get; set; } = 1;
        public bool IsAlive { get; set; } = true;
        public bool IsHittable { get; set; } = true;
    }
}

namespace MegaCrit.Sts2.Core.Models
{
    internal sealed class CardModel
    {
        public string Identity { get; set; } = "first";
        public bool Playable { get; set; } = true;
        public bool Targetable { get; set; } = true;
        public bool CanPlay() => Playable;
        public bool CanPlayTargeting(Entities.Creatures.Creature target) => Targetable;
    }
}

namespace MegaCrit.Sts2.Core.Entities.Cards
{
    internal sealed class CardPile { public List<Models.CardModel> Cards { get; } = new(); }
}

namespace MegaCrit.Sts2.Core.Entities.Players
{
    internal sealed class PlayerCombatState
    {
        public Cards.CardPile Hand { get; } = new();
        public Cards.CardPile DrawPile { get; } = new();
        public Cards.CardPile DiscardPile { get; } = new();
        public Cards.CardPile ExhaustPile { get; } = new();
        public int Energy { get; set; } = 3;
        public int TurnNumber { get; set; } = 1;
    }

    internal sealed class Player
    {
        public PlayerCombatState PlayerCombatState { get; } = new();
        public Creatures.Creature Creature { get; } = new();
    }
}

namespace MegaCrit.Sts2.Core.Combat
{
    internal sealed class CombatState
    {
        public int RoundNumber { get; set; } = 1;
        public List<Entities.Creatures.Creature> Enemies { get; } = new();
    }

    internal sealed class CombatManager
    {
        public static CombatManager Instance { get; set; } = new();
        public bool IsInProgress { get; set; } = true;
        public bool IsEnemyTurnStarted { get; set; }
        public bool PlayerActionsDisabled { get; set; }
        public CombatState State { get; } = new();
        public CombatState DebugOnlyGetState() => State;
    }
}

namespace MegaCrit.Sts2.Core.Runs
{
    internal sealed class RunState { public Entities.Players.Player Player { get; } = new(); }

    internal sealed class RunManager
    {
        public static RunManager Instance { get; set; } = new();
        public bool IsInProgress { get; set; } = true;
        public RunState State { get; } = new();
        public QueueSync ActionQueueSynchronizer { get; } = new();
        public RunState DebugOnlyGetState() => State;
    }

    internal sealed class QueueSync
    {
        public List<object> Queued { get; } = new();
        public bool ThrowAfterEnqueue { get; set; }

        public void RequestEnqueue(object action)
        {
            Queued.Add(action);
            if (ThrowAfterEnqueue) throw new InvalidOperationException("accepted then failed");
        }
    }
}

namespace MegaCrit.Sts2.Core.Context
{
    internal static class LocalContext
    {
        public static Entities.Players.Player GetMe(Runs.RunState state) => state.Player;
    }
}

namespace MegaCrit.Sts2.Core.GameActions
{
    internal sealed class PlayCardAction
    {
        public Models.CardModel Card { get; }

        public PlayCardAction(Models.CardModel card, Entities.Creatures.Creature target)
        {
            Card = card;
        }
    }

    internal sealed class EndPlayerTurnAction
    {
        public EndPlayerTurnAction(Entities.Players.Player player, int turn) { }
    }
}
