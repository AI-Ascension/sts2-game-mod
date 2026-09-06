// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using Godot;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Nodes;
using MegaCrit.Sts2.Core.Nodes.Events;
using MegaCrit.Sts2.Core.Nodes.Rooms;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Nodes.Screens.Overlays;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private static readonly string[] CampaignCharacters = { "ironclad" };

    private static RuntimeV3GameplayObservation ProjectCampaign(RuntimeV3GameplayObservation observation)
    {
        RunState? run = RunManager.Instance.DebugOnlyGetState();
        if (!RunManager.Instance.IsInProgress && !RunManager.Instance.IsGameOver)
            return Surface(observation, RuntimeV3GameplayState.Setup, CampaignCharacters, LiveCombatDemo.Ready)
                with { VisibleSeed = LiveCombatDemo.RunOptions.Seed };
        if (run == null || CurrentPlayer() == null) return observation;
        observation = observation with { NodeId = CurrentNodeId(run) };
        if (observation.State == RuntimeV3GameplayState.Defeat) return observation;
        if (NOverlayStack.Instance?.ScreenCount > 0) return observation;
        NMapScreen? map = NMapScreen.Instance;
        if (map?.IsOpen == true)
        {
            string[] points = TravelablePoints().Select(point => MapId(point, run)).ToArray();
            return Surface(observation, RuntimeV3GameplayState.Map, points,
                map.IsTravelEnabled && !map.IsTraveling && points.Length > 0);
        }
        NEventOptionButton[] options = EventButtons();
        if (options.Length > 0)
            return Surface(observation, RuntimeV3GameplayState.Event,
                options.Select(EventId).ToArray(), true);
        return observation;
    }

    private static RuntimeV3GameplayObservation Surface(RuntimeV3GameplayObservation observation,
        RuntimeV3GameplayState state, IReadOnlyList<string> values, bool enabled) => observation with
    {
        State = state, StateValues = values, IsActionable = enabled,
        InputEnabled = enabled, ModalBlocking = !enabled
    };

    private static string? CurrentNodeId(RunState run) => run.CurrentMapCoord is { } coord
        ? $"map:{run.CurrentActIndex}:{coord.row}:{coord.col}" : null;

    private static IEnumerable<Node> Descendants(Node node)
    {
        foreach (Node child in node.GetChildren())
        {
            yield return child;
            foreach (Node descendant in Descendants(child)) yield return descendant;
        }
    }

    private static NMapPoint[] TravelablePoints() => NMapScreen.Instance is { IsOpen: true } map
        ? Descendants(map).OfType<NMapPoint>().Where(point => point.IsVisibleInTree()
            && point.State == MapPointState.Travelable).ToArray() : Array.Empty<NMapPoint>();

    private static string MapId(NMapPoint point, RunState run) =>
        $"map:{run.CurrentActIndex}:{point.Point.coord.row}:{point.Point.coord.col}:{point.Point.PointType}";

    private static NEventOptionButton[] EventButtons() => NEventRoom.Instance?.Layout is { } layout
        ? layout.OptionButtons.Where(button => button.IsVisibleInTree() && button.IsEnabled
            && !button.Option.IsLocked && !button.Option.WasChosen).ToArray()
        : Array.Empty<NEventOptionButton>();

    private static string EventId(NEventOptionButton button) => "event:" + button.Option.TextKey;

    private static LegalActionReference[] CampaignActions(RuntimeV3GameplayObservation observation)
    {
        if (!observation.InputEnabled) return Array.Empty<LegalActionReference>();
        string? kind = observation.State switch
        {
            RuntimeV3GameplayState.Setup => "start_run",
            RuntimeV3GameplayState.Map => "select_map_node",
            RuntimeV3GameplayState.Event => "event_choice",
            _ => null
        };
        return kind == null ? Array.Empty<LegalActionReference>() : observation.StateValues.Select(value =>
            new LegalActionReference($"{kind}:{observation.Generation}:{value}", kind, value, null,
                observation.Generation)).ToArray();
    }
}
