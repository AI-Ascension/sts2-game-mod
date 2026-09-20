// SPDX-License-Identifier: MIT

using System.Collections.Generic;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.MonsterMoves.Intents;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.Rooms;

namespace Godot
{
    internal class GodotObject
    {
        internal static bool IsInstanceValid(GodotObject? value) => value is not null;
    }
    internal class Node : GodotObject
    {
        internal List<Node> Children { get; } = new();
        internal bool Visible { get; init; } = true;
        internal bool IsVisibleInTree() => Visible;
        internal IEnumerable<Node> GetChildren(bool includeInternal) { _ = includeInternal; return Children; }
    }
    internal readonly record struct Color(float A);
    internal class Control : Node
    {
        internal Color Modulate { get; init; } = new(1f);
    }
}

namespace MegaCrit.Sts2.Core.Nodes.Combat
{
    internal static class NCombatUi { internal static bool IsDebugHidingIntent { get; set; } }
    internal sealed class NCreature : Control { internal Control IntentContainer { get; } = new(); }
    internal sealed class NIntent : Control
    {
        // Names reproduce only the fields inspected by production reflection. The probe does
        // not certify that any proprietary build has this layout.
        private readonly AbstractIntent _intent;
        private readonly Creature _owner;
        private readonly IReadOnlyList<Creature> _targets;
        private readonly bool _isFrozen;
        internal NIntent(AbstractIntent intent, Creature owner, IReadOnlyList<Creature> targets, bool frozen)
        { _intent = intent; _owner = owner; _targets = targets; _isFrozen = frozen; }
        internal (AbstractIntent Intent, Creature Owner, IReadOnlyList<Creature> Targets, bool Frozen) Layout =>
            (_intent, _owner, _targets, _isFrozen);
    }
}

namespace MegaCrit.Sts2.Core.Nodes.Rooms
{
    internal sealed class NCombatRoom
    {
        internal static NCombatRoom? Instance { get; set; }
        internal Dictionary<Creature, NCreature> Creatures { get; } = new(System.Collections.Generic.ReferenceEqualityComparer.Instance);
        internal NCreature? GetCreatureNode(Creature creature) => Creatures.GetValueOrDefault(creature);
    }
}

namespace AiAscension.Sts2GameMod.Runtime
{
    internal sealed class RenderedEnemyFixture
    {
        internal Creature Enemy { get; } = EnemyProjectionProbe.Enemy(1);
        internal AttackIntent Attack { get; } = new() { IntentType = IntentType.Attack };
        internal int DamageReads { get; private set; }
        internal static RenderedEnemyFixture Create(bool frozen = false, bool compound = false)
        {
            var result = new RenderedEnemyFixture();
            result.Attack.ReadDamage = () => { result.DamageReads++; return 7; };
            Creature player = EnemyProjectionProbe.Enemy(99);
            var combat = new CombatState { ReadEnemies = () => new[] { result.Enemy } };
            combat.Players.Add(new Player(player));
            result.Enemy.CombatState = combat;
            var intents = new List<AbstractIntent> { result.Attack };
            if (compound) intents.Add(new AbstractIntent { IntentType = IntentType.Debuff });
            result.Enemy.Monster = new Monster(new Move(intents));
            var node = new NCreature();
            foreach (AbstractIntent intent in intents)
                node.IntentContainer.Children.Add(new NIntent(intent, result.Enemy, new[] { player }, frozen));
            var room = new NCombatRoom();
            room.Creatures.Add(result.Enemy, node);
            room.Creatures.Add(player, new NCreature());
            NCombatRoom.Instance = room;
            NCombatUi.IsDebugHidingIntent = false;
            return result;
        }
    }
}
