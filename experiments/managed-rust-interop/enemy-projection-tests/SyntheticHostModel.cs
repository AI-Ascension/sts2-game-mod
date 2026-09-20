// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;

// Original, deliberately minimal API-shape doubles for the source-only probe. They are not a
// game simulator and cannot establish native member layout, visibility, or calculation semantics.
namespace MegaCrit.Sts2.Core.Combat
{
    internal sealed class CombatState
    {
        internal Func<IEnumerable<Entities.Creatures.Creature>> ReadEnemies { get; init; } =
            () => Array.Empty<Entities.Creatures.Creature>();
        internal IEnumerable<Entities.Creatures.Creature> Enemies => ReadEnemies();
        internal List<Entities.Players.Player> Players { get; } = new();
    }
}

namespace MegaCrit.Sts2.Core.Entities.Players
{
    internal sealed record Player(Creatures.Creature Creature);
}

namespace MegaCrit.Sts2.Core.Entities.Creatures
{
    internal sealed record Creature
    {
        internal string Id { get; init; } = "enemy:1";
        internal Func<string> ReadName { get; init; } = () => "Synthetic enemy";
        internal Func<int> ReadHp { get; set; } = () => 9;
        internal Func<int> ReadMaxHp { get; init; } = () => 20;
        internal string Name => ReadName();
        internal int CurrentHp => ReadHp();
        internal int MaxHp => ReadMaxHp();
        internal Combat.CombatState? CombatState { get; set; }
        internal Monster? Monster { get; set; }
    }
    internal sealed record Monster(Move NextMove);
    internal sealed record Move(IReadOnlyList<MonsterMoves.Intents.AbstractIntent> Intents);
}

namespace MegaCrit.Sts2.Core.MonsterMoves.Intents
{
    internal enum IntentType { Attack, DeathBlow, Defend, Buff, Debuff, DebuffStrong, CardDebuff, Unknown }
    internal class AbstractIntent
    {
        internal IntentType IntentType { get; init; } = IntentType.Unknown;
    }
    internal sealed class AttackIntent : AbstractIntent
    {
        internal Func<int> ReadDamage { get; set; } = () => 7;
        internal int Repeats { get; init; } = 2;
        internal int GetSingleDamage(Entities.Creatures.Creature[] targets, Entities.Creatures.Creature enemy)
        {
            _ = targets; _ = enemy;
            return ReadDamage();
        }
    }
}
