// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.TreasureRoomRelic;
using MegaCrit.Sts2.Core.Nodes.TreasureRooms;
using MegaCrit.Sts2.Core.Rooms;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private const string TreasureChestId = "treasure:chest";

    private static NTreasureRoom? CurrentTreasure() =>
        RunManager.Instance.DebugOnlyGetState()?.CurrentRoom is TreasureRoom
        && NRun.Instance?.TreasureRoom is { } room && GodotObject.IsInstanceValid(room)
        && room.IsVisibleInTree() ? room : null;

    private static NTreasureButton? TreasureChest(NTreasureRoom room)
    {
        var buttons = Descendants(room).OfType<NTreasureButton>().Where(Clickable).ToArray();
        return buttons.Length == 1 ? buttons[0] : null;
    }

    private static NTreasureRoomRelicHolder[] TreasureRelics(NTreasureRoom room) =>
        Descendants(room).OfType<NTreasureRoomRelicHolder>()
            .Where(holder => Clickable(holder) && holder.Relic?.Model != null).ToArray();

    private static string TreasureRelicId(NTreasureRoomRelicHolder holder) =>
        $"treasure:relic:{holder.Index}:{holder.Relic.Model.Id.Entry}";

    private static RuntimeV3GameplayObservation ProjectTreasure(RuntimeV3GameplayObservation observation,
        NTreasureRoom room)
    {
        var values = TreasureRelics(room).Select(TreasureRelicId).ToList();
        if (TreasureChest(room) != null) values.Add(TreasureChestId);
        return Surface(observation, RuntimeV3GameplayState.Reward, values,
            MegaCrit.Sts2.Core.Nodes.CommonUi.NModalContainer.Instance?.OpenModal == null);
    }

    private static LegalActionReference[] TreasureActions(RuntimeV3GameplayObservation observation,
        NTreasureRoom room)
    {
        var actions = new List<LegalActionReference>();
        foreach (string value in observation.StateValues)
            actions.Add(new($"choose_reward:{observation.Generation}:{value}", "choose_reward", value,
                null, observation.Generation));
        if (Clickable(room.ProceedButton))
            actions.Add(new($"proceed:{observation.Generation}", "proceed", null, null, observation.Generation));
        return actions.ToArray();
    }
}
