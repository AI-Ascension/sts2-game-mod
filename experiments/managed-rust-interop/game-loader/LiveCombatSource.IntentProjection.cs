// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Reflection;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.MonsterMoves.Intents;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Nodes.Rooms;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static readonly FieldInfo? RenderedIntentField =
        typeof(NIntent).GetField("_intent", BindingFlags.Instance | BindingFlags.NonPublic);
    private static readonly FieldInfo? RenderedIntentOwnerField =
        typeof(NIntent).GetField("_owner", BindingFlags.Instance | BindingFlags.NonPublic);
    private static readonly FieldInfo? RenderedIntentTargetsField =
        typeof(NIntent).GetField("_targets", BindingFlags.Instance | BindingFlags.NonPublic);
    private static readonly FieldInfo? RenderedIntentFrozenField =
        typeof(NIntent).GetField("_isFrozen", BindingFlags.Instance | BindingFlags.NonPublic);

    private static RuntimeV3GameplayEnemy[] ProjectEnemiesSafely(CombatState hostCombat)
    {
        try
        {
            var result = new List<RuntimeV3GameplayEnemy>();
            var identities = new HashSet<string>(StringComparer.Ordinal);
            foreach (Creature enemy in hostCombat.Enemies)
            {
                if (result.Count >= RuntimeV3GameplayContract.MaxEntities)
                    throw new InvalidOperationException("enemy collection exceeds its bound");
                RuntimeV3GameplayEnemy projected = ProjectEnemy(enemy);
                if (!identities.Add(projected.EnemyId))
                    throw new InvalidOperationException("enemy identity is duplicated");
                result.Add(projected);
            }
            return result.ToArray();
        }
        catch (Exception)
        {
            // Required enemy facts are all-or-unavailable. Do not publish a partial collection,
            // invent a dead enemy, or retain host exception details in an inner exception.
            // The existing observation boundary maps this refusal to its unavailable response.
            throw new InvalidOperationException("live enemy observation unavailable");
        }
    }

    private static RuntimeV3GameplayEnemy ProjectEnemy(Creature enemy)
    {
        string id = EnemyId(enemy);
        string name = enemy.Name;
        int hp = enemy.CurrentHp;
        int maxHp = enemy.MaxHp;
        if (!RuntimeV3GameplayContract.IsIdentity(id)
            || !RuntimeV3GameplayContract.IsText(name)
            || hp < 0 || maxHp < 0 || maxHp > ushort.MaxValue || hp > maxHp)
            throw new InvalidOperationException("required enemy facts are invalid");
        RuntimeV3GameplayEnemy unknown = new(
            id, name, (ushort)hp, (ushort)maxHp,
            RuntimeV3GameplayIntent.Unknown, 0, 0);
        if (!TryVisibleIntentSet(enemy, out IReadOnlyList<AbstractIntent> intents,
                out IReadOnlyList<IReadOnlyList<Creature>> targetSets)) return unknown;
        if (intents.Count != 1) return unknown;

        AbstractIntent intent = intents[0];
        RuntimeV3GameplayIntent kind = ProjectIntent(intent.IntentType);
        if (kind != RuntimeV3GameplayIntent.Attack) return unknown with { Intent = kind };
        if (intent is not AttackIntent attack) return unknown;

        Creature[] targets = VisibleIntentTargets(enemy, targetSets[0]);
        if (targets.Length == 0) return unknown;
        if (!TryProjectAttack(attack, targets, enemy, out ushort damage, out byte hits)) return unknown;
        return unknown with
        {
            Intent = kind,
            IntentDamage = damage,
            IntentHits = hits
        };
    }

    private static RuntimeV3GameplayIntent ProjectIntent(IntentType intent) => intent switch
    {
        IntentType.Attack or IntentType.DeathBlow => RuntimeV3GameplayIntent.Attack,
        IntentType.Defend => RuntimeV3GameplayIntent.Defend,
        IntentType.Buff => RuntimeV3GameplayIntent.Buff,
        IntentType.Debuff or IntentType.DebuffStrong or IntentType.CardDebuff =>
            RuntimeV3GameplayIntent.Debuff,
        _ => RuntimeV3GameplayIntent.Unknown
    };

    private static bool TryVisibleIntentSet(Creature enemy, out IReadOnlyList<AbstractIntent> intents,
        out IReadOnlyList<IReadOnlyList<Creature>> targetSets)
    {
        intents = Array.Empty<AbstractIntent>();
        targetSets = Array.Empty<IReadOnlyList<Creature>>();
        try
        {
            if (NCombatUi.IsDebugHidingIntent) return false;
            NCreature? node = NCombatRoom.Instance?.GetCreatureNode(enemy);
            if (node is null || !GodotObject.IsInstanceValid(node) || !node.IsVisibleInTree()) return false;
            Control container = node.IntentContainer;
            if (!GodotObject.IsInstanceValid(container) || !container.IsVisibleInTree()
                || container.Modulate.A <= 0f) return false;

            IReadOnlyList<AbstractIntent>? current = enemy.Monster?.NextMove?.Intents;
            if (current is null || current.Count == 0) return false;
            NIntent[] rendered = ((Node)container).GetChildren(false).OfType<NIntent>().ToArray();
            if (rendered.Length != current.Count || rendered.Any(child =>
                !GodotObject.IsInstanceValid(child) || !child.IsVisibleInTree()
                || child.Modulate.A <= 0f)) return false;
            if (RenderedIntentField is null || RenderedIntentOwnerField is null
                || RenderedIntentTargetsField is null || RenderedIntentFrozenField is null) return false;
            var renderedTargetSets = new List<IReadOnlyList<Creature>>(rendered.Length);
            for (int index = 0; index < rendered.Length; index++)
            {
                NIntent child = rendered[index];
                if (RenderedIntentField.GetValue(child) is not AbstractIntent visible
                    || !ReferenceEquals(visible, current[index])
                    || !ReferenceEquals(RenderedIntentOwnerField.GetValue(child), enemy)
                    || RenderedIntentFrozenField.GetValue(child) is not bool isFrozen || isFrozen)
                    return false;
                if (RenderedIntentTargetsField.GetValue(child) is not IEnumerable<Creature> rawTargets)
                    return false;
                Creature[] childTargets = rawTargets.ToArray();
                if (childTargets.Any(target => target is null || !IsVisibleCreature(target))) return false;
                renderedTargetSets.Add(childTargets);
            }

            intents = current;
            targetSets = renderedTargetSets;
            return true;
        }
        catch (Exception)
        {
            intents = Array.Empty<AbstractIntent>();
            targetSets = Array.Empty<IReadOnlyList<Creature>>();
            return false;
        }
    }

    private static Creature[] VisibleIntentTargets(Creature enemy, IReadOnlyList<Creature> renderedTargets)
    {
        try
        {
            if (enemy.CombatState is not { } combat || NCombatRoom.Instance is not { } room)
                return Array.Empty<Creature>();
            Creature[] candidates = combat.Players.Select(player => player.Creature).ToArray();
            if (renderedTargets.Count == 0 || renderedTargets.Any(target => !candidates.Contains(target)))
                return Array.Empty<Creature>();
            return renderedTargets.Where(candidates.Contains)
                .Where(target => IsVisibleCreature(room, target)).ToArray();
        }
        catch (Exception)
        {
            return Array.Empty<Creature>();
        }
    }

    private static bool IsVisibleCreature(Creature target)
    {
        NCombatRoom? room = NCombatRoom.Instance;
        return room is not null && IsVisibleCreature(room, target);
    }

    private static bool IsVisibleCreature(NCombatRoom room, Creature target) =>
        room.GetCreatureNode(target) is { } node
            && GodotObject.IsInstanceValid(node) && node.IsVisibleInTree();

    private static bool TryProjectAttack(AttackIntent attack, Creature[] targets, Creature enemy,
        out ushort damage, out byte hits)
    {
        damage = 0;
        hits = 0;
        try
        {
            int singleDamage = attack.GetSingleDamage(targets, enemy);
            int repeats = attack.Repeats;
            if (singleDamage < 0 || singleDamage > ushort.MaxValue
                || repeats < 1 || repeats > byte.MaxValue) return false;
            damage = (ushort)singleDamage;
            hits = (byte)repeats;
            return true;
        }
        catch (Exception)
        {
            // Host calculations can be unavailable during a transition. Keep the entire
            // intent Unknown rather than publishing a partial or clamped projection.
            return false;
        }
    }
}
