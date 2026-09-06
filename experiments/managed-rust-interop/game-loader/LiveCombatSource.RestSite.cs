// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Entities.RestSite;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Nodes.RestSite;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static NRestSiteRoom? CurrentRestSite() =>
        RunManager.Instance.DebugOnlyGetState()?.CurrentRoom is RestSiteRoom
        && NRestSiteRoom.Instance is { } room && GodotObject.IsInstanceValid(room)
        && room.IsVisibleInTree() ? room : null;

    private static NRestSiteButton[] RestButtons(NRestSiteRoom room) => Descendants(room)
        .OfType<NRestSiteButton>().Where(button => Clickable(button)
            && button.Option is { IsEnabled: true }).ToArray();

    private static NRestSiteButton? RestHealButton(NRestSiteRoom room) => RestButtons(room)
        .SingleOrDefault(button => button.Option is HealRestSiteOption);

    private static NRestSiteButton? RestSmithButton(NRestSiteRoom room) => RestButtons(room)
        .SingleOrDefault(button => button.Option is SmithRestSiteOption { SmithCount: 1 });

    private static bool CanSmith(CardModel card) => card.CurrentUpgradeLevel < card.MaxUpgradeLevel;

    private static RuntimeV3GameplayObservation ProjectRestSite(RuntimeV3GameplayObservation observation,
        NRestSiteRoom room) => Surface(observation, RuntimeV3GameplayState.Rest,
            RestButtons(room).Select(button => button.Option!.OptionId).ToArray(),
            MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal == null);

    private LegalActionReference[] RestActions(RuntimeV3GameplayObservation observation)
    {
        if (CurrentRestSite() is not { } room) return Array.Empty<LegalActionReference>();
        var actions = new List<LegalActionReference>();
        if (RestHealButton(room) != null)
            actions.Add(new($"rest:{observation.Generation}", "rest", null, null, observation.Generation));
        if (RestSmithButton(room) != null && CurrentPlayer() is { } player)
            foreach (CardModel card in player.Deck.Cards.Where(CanSmith))
            {
                string id = CardId(card);
                actions.Add(new($"smith:{observation.Generation}:{id}", "smith", id, null,
                    observation.Generation));
            }
        if (Clickable(room.ProceedButton))
            actions.Add(new($"proceed:{observation.Generation}", "proceed", null, null,
                observation.Generation));
        return actions.ToArray();
    }
}
