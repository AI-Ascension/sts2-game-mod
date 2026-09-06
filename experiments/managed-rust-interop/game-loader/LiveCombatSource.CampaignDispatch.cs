// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using System.Threading.Tasks;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.Nodes.Screens.Map;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class LiveCombatSource
{
    private readonly Dictionary<RuntimeV3OperationKey, CampaignPending> _campaignPending = new();
    private sealed class CampaignPending(LegalActionReference action,
        RuntimeV3GameplayObservation before, Func<bool> postcondition, string effect)
    {
        internal LegalActionReference Action { get; } = action;
        internal RuntimeV3GameplayObservation Before { get; } = before;
        internal Func<bool> Postcondition { get; } = postcondition;
        internal string Effect { get; } = effect;
        internal Task? Work { get; set; }
    }

    private bool DispatchCampaign(RuntimeV3OperationKey operation, LegalActionReference action,
        RuntimeV3GameplayObservation before)
    {
        if (_campaignPending.ContainsKey(operation) || _campaignPending.Count >= 4096) return false;
        Func<Task> invoke;
        Func<bool> postcondition;
        string effect;
        if (action.Kind == "start_run" && action.Value == "ironclad")
        {
            invoke = async () => { await LiveCombatDemo.StartCampaignAsync(); };
            postcondition = () => CurrentPlayer() != null
                && (LiveCombatDemo.RunOptions.Practice
                    ? RunManager.Instance.DebugOnlyGetState()?.Rng.StringSeed == before.VisibleSeed
                    : RunManager.Instance.ShouldSave
                        && RunManager.Instance.DebugOnlyGetState()?.GameMode == GameMode.Standard);
            effect = "campaign_started";
        }
        else if (action.Kind == "select_map_node" && RunManager.Instance.DebugOnlyGetState() is { } run)
        {
            var point = TravelablePoints().SingleOrDefault(candidate => MapId(candidate, run) == action.Value);
            if (point == null || NMapScreen.Instance is not { IsTravelEnabled: true, IsTraveling: false } map)
                return false;
            var destination = point.Point.coord;
            var previousRoom = run.CurrentRoom;
            var queued = new MoveToMapCoordAction(CurrentPlayer()!, destination);
            invoke = () =>
            {
                RunManager.Instance.ActionQueueSynchronizer.RequestEnqueue(queued);
                return queued.CompletionTask;
            };
            postcondition = () => queued.State == GameActionState.Finished && queued.Exception == null
                && run.CurrentMapCoord == destination && run.CurrentRoom != previousRoom;
            effect = "map_room_entered";
        }
        else if (action.Kind == "event_choice")
        {
            var button = EventButtons().SingleOrDefault(candidate => EventId(candidate) == action.Value);
            if (button == null) return false;
            var option = button.Option;
            invoke = option.Chosen;
            postcondition = () => option.WasChosen;
            effect = "event_choice_completed";
        }
        else return false;
        // Retain identity before invoking host code. A synchronous failure remains unknown.
        var pending = new CampaignPending(action, before, postcondition, effect);
        _campaignPending.Add(operation, pending);
        pending.Work = invoke();
        return true;
    }

    private RuntimeV3HostCompletion? CampaignCompletion(RuntimeV3OperationKey operation,
        LegalActionReference action)
    {
        CampaignPending pending = _campaignPending[operation];
        if (pending.Action != action || pending.Work?.IsCompletedSuccessfully != true
            || !pending.Postcondition()) return null;
        RuntimeV3GameplayObservation after = Observe();
        if (after.Generation <= pending.Before.Generation
            || after.State is RuntimeV3GameplayState.Recovery or RuntimeV3GameplayState.Setup) return null;
        var witness = new RuntimeV3TransitionWitness(operation, action, pending.Before.Generation,
            after.Generation, after.StateId, pending.Effect);
        return new(after, witness, LegalActions(after));
    }
}
