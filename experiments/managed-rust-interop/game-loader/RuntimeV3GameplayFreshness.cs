// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Text;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

public static partial class ModEntry
{
    // Main-thread-only references fence index reuse; they never enter an observation or ABI.
    private static object? _runtimeV3PreviousRun;
    private static object? _runtimeV3PreviousCombat;
    private static object? _runtimeV3PreviousPlayer;
    private static readonly List<CardModel> RuntimeV3PreviousCards = new();
    private static ulong _runtimeV3HandRevision;

    private static string RuntimeV3GameplayHandSignature(CombatState combat, Player player)
    {
        object? run = RunManager.Instance.DebugOnlyGetState();
        var cards = player.PlayerCombatState.Hand.Cards;
        bool changed = !ReferenceEquals(run, _runtimeV3PreviousRun)
            || !ReferenceEquals(combat, _runtimeV3PreviousCombat)
            || !ReferenceEquals(player, _runtimeV3PreviousPlayer)
            || cards.Count != RuntimeV3PreviousCards.Count;
        if (!changed)
        {
            for (int index = 0; index < cards.Count; index++)
                changed |= !ReferenceEquals(cards[index], RuntimeV3PreviousCards[index]);
        }
        if (changed)
        {
            _runtimeV3HandRevision++;
            _runtimeV3PreviousRun = run;
            _runtimeV3PreviousCombat = combat;
            _runtimeV3PreviousPlayer = player;
            RuntimeV3PreviousCards.Clear();
            foreach (CardModel card in cards) RuntimeV3PreviousCards.Add(card);
        }
        StringBuilder signature = new();
        signature.Append('|').Append(_runtimeV3HandRevision);
        foreach (CardModel card in cards)
        {
            signature.Append('|').Append(card.CanPlay());
            signature.Append(':').Append(card.CanPlayTargeting(player.Creature));
            int count = 0;
            foreach (var enemy in combat.Enemies)
            {
                if (++count > RuntimeV3GameplayMaxEnemies) break;
                signature.Append(':').Append(card.CanPlayTargeting(enemy));
            }
        }
        return signature.ToString();
    }
}
