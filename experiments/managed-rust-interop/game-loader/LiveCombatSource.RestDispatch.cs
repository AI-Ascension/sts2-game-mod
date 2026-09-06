// SPDX-License-Identifier: MIT

using System;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private bool PrepareRest(LegalActionReference action, RuntimeV3GameplayObservation before,
        out Func<Task> invoke, out Func<bool> postcondition, out string effect)
    {
        invoke = () => Task.CompletedTask;
        postcondition = () => false;
        effect = "";
        if (before.State != RuntimeV3GameplayState.Rest || CurrentRestSite() is not { } room
            || CurrentPlayer() is not { } player) return false;
        if (action.Kind == "rest" && RestHealButton(room) is { } heal)
        {
            invoke = () => { heal.ForceClick(); return Task.CompletedTask; };
            postcondition = () => !Clickable(heal) && Clickable(room.ProceedButton)
                && (player.Creature.CurrentHp > before.Player.Hp
                    || before.Player.Hp == before.Player.MaxHp
                        && player.Creature.CurrentHp >= before.Player.Hp);
            effect = "rest_healing_completed";
            return true;
        }
        if (action.Kind == "smith" && RestSmithButton(room) is { } smith)
        {
            var card = player.Deck.Cards.SingleOrDefault(card => CardId(card) == action.Value && CanSmith(card));
            if (card == null) return false;
            int previousLevel = card.CurrentUpgradeLevel;
            invoke = () => SmithCardAsync(smith, card, previousLevel);
            postcondition = () => card.CurrentUpgradeLevel > previousLevel
                && player.Deck.Cards.Contains(card) && !Clickable(smith);
            effect = "rest_card_upgraded";
            return true;
        }
        if (action.Kind == "proceed" && Clickable(room.ProceedButton))
        {
            invoke = () => { room.ProceedButton.ForceClick(); return Task.CompletedTask; };
            postcondition = () => NMapScreen.Instance is { IsOpen: true } map && map.IsVisibleInTree();
            effect = "rest_proceed_opened_map";
            return true;
        }
        return false;
    }
}
