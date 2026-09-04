// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Text;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Creatures;
using MegaCrit.Sts2.Core.Entities.Players;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    private static RuntimeV3GameplayHostObservation ReadRuntimeV3GameplayObservation()
    {
        RuntimeV3GameplayHostObservation raw = ReadRuntimeV3GameplayHostObservation();
        if (!raw.HostReady)
        {
            return raw with { Generation = _runtimeV3GameplayGeneration };
        }

        if (!_runtimeV3GameplayBaseline)
        {
            _runtimeV3GameplayBaseline = true;
            _runtimeV3GameplayLastSignature = raw.Signature;
        }
        else if (!String.Equals(_runtimeV3GameplayLastSignature, raw.Signature, StringComparison.Ordinal))
        {
            _runtimeV3GameplayGeneration++;
            _runtimeV3GameplayLastSignature = raw.Signature;
        }

        return raw with { Generation = _runtimeV3GameplayGeneration };
    }

    private static RuntimeV3GameplayHostObservation ReadRuntimeV3GameplayHostObservation()
    {
        try
        {
            if (Engine.GetMainLoop() is not SceneTree tree || tree.Root == null)
            {
                return EmptyRuntimeV3GameplayObservation(false);
            }

            CombatManager? combatManager = CombatManager.Instance;
            if (combatManager == null || !combatManager.IsInProgress)
            {
                return EmptyRuntimeV3GameplayObservation(true);
            }

            CombatState? combatState = combatManager.DebugOnlyGetState();
            if (combatState == null
                || !TryGetRuntimeV3GameplayPlayer(out Player? player)
                || player == null
                || player.PlayerCombatState == null)
            {
                return EmptyRuntimeV3GameplayObservation(false);
            }

            PlayerCombatState playerCombat = player.PlayerCombatState;
            if (playerCombat.Hand.Cards.Count > RuntimeV3GameplayMaxCardIndex)
                return EmptyRuntimeV3GameplayObservation(false);
            List<RuntimeV3GameplayEnemyObservation> enemies = new();
            foreach (Creature enemy in combatState.Enemies)
            {
                if (enemy.CombatId is uint combatId && enemies.Count < RuntimeV3GameplayMaxEnemies)
                {
                    enemies.Add(new RuntimeV3GameplayEnemyObservation(
                        combatId.ToString(CultureInfo.InvariantCulture),
                        enemy.IsAlive,
                        enemy.IsHittable));
                }
            }

            ushort handCount = BoundedRuntimeV3GameplayCount(
                playerCombat.Hand?.Cards.Count ?? 0,
                RuntimeV3GameplayMaxCardIndex);
            ushort energy = BoundedRuntimeV3GameplayCount(
                playerCombat.Energy,
                RuntimeV3GameplayMaxEnergy);
            ushort drawCount = BoundedRuntimeV3GameplayCount(
                playerCombat.DrawPile?.Cards.Count ?? 0,
                RuntimeV3GameplayMaxPileCount);
            ushort discardCount = BoundedRuntimeV3GameplayCount(
                playerCombat.DiscardPile?.Cards.Count ?? 0,
                RuntimeV3GameplayMaxPileCount);
            ushort exhaustCount = BoundedRuntimeV3GameplayCount(
                playerCombat.ExhaustPile?.Cards.Count ?? 0,
                RuntimeV3GameplayMaxPileCount);
            string phase = combatManager.IsEnemyTurnStarted
                ? RuntimeV3GameplayEnemyTurn
                : RuntimeV3GameplayPlayerTurn;
            string signature = RuntimeV3GameplaySignature(
                phase,
                combatState.RoundNumber,
                handCount,
                energy,
                drawCount,
                discardCount,
                exhaustCount,
                enemies) + RuntimeV3GameplayHandSignature(combatState, player);
            return new RuntimeV3GameplayHostObservation(
                phase,
                BoundedRuntimeV3GameplayCount(combatState.RoundNumber, RuntimeV2MaxTurnIndex),
                true,
                0,
                handCount,
                energy,
                drawCount,
                discardCount,
                exhaustCount,
                enemies,
                signature);
        }
        catch
        {
            return EmptyRuntimeV3GameplayObservation(false);
        }
    }

    private static RuntimeV3GameplayHostObservation EmptyRuntimeV3GameplayObservation(bool hostReady)
    {
        return new RuntimeV3GameplayHostObservation(
            RuntimeV3GameplayOutsideCombat,
            0,
            hostReady,
            0,
            0,
            0,
            0,
            0,
            0,
            Array.Empty<RuntimeV3GameplayEnemyObservation>(),
            hostReady ? "ready" : "unavailable");
    }

    private static ushort BoundedRuntimeV3GameplayCount(int value, ushort maximum)
    {
        if (value <= 0)
        {
            return 0;
        }

        return value >= maximum ? maximum : (ushort)value;
    }

    private static string RuntimeV3GameplaySignature(
        string phase,
        int round,
        ushort hand,
        ushort energy,
        ushort draw,
        ushort discard,
        ushort exhaust,
        IReadOnlyList<RuntimeV3GameplayEnemyObservation> enemies)
    {
        StringBuilder builder = new();
        builder.Append(phase).Append('|').Append(round).Append('|').Append(hand).Append('|')
            .Append(energy).Append('|').Append(draw).Append('|').Append(discard).Append('|').Append(exhaust);
        foreach (RuntimeV3GameplayEnemyObservation enemy in enemies)
        {
            builder.Append('|').Append(enemy.TargetId).Append(':').Append(enemy.Alive).Append(':').Append(enemy.Hittable);
        }

        return builder.ToString();
    }
}
