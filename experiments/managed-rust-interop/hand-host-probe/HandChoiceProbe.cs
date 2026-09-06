// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using Godot;
using MegaCrit.Sts2.Core.Combat;
using MegaCrit.Sts2.Core.Commands;
using MegaCrit.Sts2.Core.Context;
using MegaCrit.Sts2.Core.Entities.Cards;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Models.Cards;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Combat;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

// Opt-in host-dependent fixture, excluded from normal addon builds. This creates
// test combat and one generated card; it is never campaign or replay evidence.
internal static class HandChoiceProbe
{
    internal static async Task PrepareAsync()
    {
        if (System.Environment.GetEnvironmentVariable("STS2_HAND_CHOICE_PROBE") != "1"
            || !LiveCombatDemo.Campaign || !LiveCombatDemo.RunOptions.Practice
            || LiveCombatDemo.RunOptions.Seed != "AIASCENSIONHANDTEST1")
            throw new InvalidOperationException("hand probe requires its explicit isolated fixture");
        RunState run = await LiveCombatDemo.StartCampaignAsync();
        EncounterModel encounter = ModelDb.AllEncounters.Where(e => e.IsWeak
                && e.RoomType == RoomType.Monster && !e.IsDebugEncounter)
            .OrderBy(e => e.Id.ToString(), StringComparer.Ordinal).First();
        await RunManager.Instance.EnterRoomDebug(RoomType.Monster, MapPointType.Monster,
            encounter.MutableClone(), false);
        Player player = LocalContext.GetMe(run)
            ?? throw new InvalidOperationException("hand probe player unavailable");
        for (int frame = 0; frame < 600; frame++)
        {
            if (player.PlayerCombatState?.Phase == PlayerTurnPhase.Play
                && NPlayerHand.Instance is { } hand && hand.IsVisibleInTree()
                && hand.ActiveHolders.Count >= 5 && frame >= 120)
            {
                await CardPileCmd.AddToCombatAndPreview<Armaments>(player.Creature,
                    PileType.Hand, 1, player);
                GD.Print("[AI-ASCENSION HAND PROBE] fixture ready; generated Armaments in hand");
                return;
            }
            NGame game = NGame.Instance ?? throw new InvalidOperationException("hand probe scene unavailable");
            await game.ToSignal(game.GetTree(), SceneTree.SignalName.ProcessFrame);
        }
        throw new InvalidOperationException("hand probe combat readiness timed out");
    }
}
