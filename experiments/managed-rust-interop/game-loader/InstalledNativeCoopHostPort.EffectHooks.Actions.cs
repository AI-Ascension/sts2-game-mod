// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Models;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Helpers;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class InstalledNativeCoopHostPort
{
    private void OnNativeActionStarting(GameAction action)
    {
        if (action is not MoveToMapCoordAction
            || _effectMapSynchronizer is null
            || !TryParseMapAction(action, out _, out MapCoord destination))
        {
            return;
        }
        RunState? runState = RunManager.Instance?.DebugOnlyGetState();
        if (runState is null)
            return;
        MapLocation source = runState.MapLocation;
        if (!IsReachableMapDestination(runState, source, destination))
            return;
        NativeMapActionCandidate candidate = new(action, source,
            _effectMapSynchronizer.MapGenerationCount, destination, _effectMapSynchronizer);
        _nativeMapActions[action] = candidate;
        foreach (NativePendingOperation pending in _pending.Values)
            pending.TryBindMapAction(action);
    }

    private void OnNativeActionExecuted(GameAction action)
    {
        if (action is MoveToMapCoordAction)
        {
            // MoveToMapCoordAction starts GoToMapCoord fire-and-forget. Its AfterAction callback
            // therefore binds the action only as a fallback; the semantic checksum waits for
            // PlayerVotesCleared after the RunState transition.
            OnNativeActionStarting(action);
            return;
        }

        bool isChoiceAction = _nativeChoiceActions.Remove(action);
        bool isEndTurnAction = action is EndPlayerTurnAction;
        if (!isChoiceAction && !isEndTurnAction)
        {
            // Ordinary actions are marked when RunManager raises its post-action checksum. The
            // marker is only needed until this callback; discard it here so a long run cannot
            // retain every previously executed action object.
            _actionChecksummed.Remove(action);
            return;
        }

        // Ordinary combat actions receive RunManager's post-action checksum, but end-turn and
        // non-combat hook actions do not. Add the same first-party action checksum only when no
        // exact callback has been observed for this action.
        if (_actionChecksummed.Remove(action))
            return;
        RunManager? manager = RunManager.Instance;
        ChecksumTracker? tracker = manager?.ChecksumTracker;
        if (tracker is null || !tracker.IsEnabled)
            return;
        try
        {
            tracker.GenerateChecksum("finished action execution " + action, action);
        }
        catch
        {
            // Reconciliation remains pending until a matching checksum is observable.
        }
    }

    private void OnNativePlayerChoiceReceived(Player player, uint choiceId,
        NetPlayerChoiceResult result)
    {
        GameAction? action = RunManager.Instance?.ActionExecutor?.CurrentlyRunningAction;
        RunState? runState = RunManager.Instance?.DebugOnlyGetState();
        PlayerChoiceResult? choice;
        try
        {
            choice = runState is null
                ? null
                : PlayerChoiceResult.FromNetData(player, runState, result);
        }
        catch
        {
            choice = null;
        }
        if (choice is null || !IsNativePlayerChoiceResultLegal(runState, player, choice)
            || action is null
            || action.State != GameActionState.GatheringPlayerChoice
            || action.OwnerId != player.NetId)
        {
            return;
        }

        _nativeChoiceActions.Add(action);
        foreach (NativePendingOperation pending in _pending.Values)
            pending.TryRecordPlayerChoice(player, choiceId, result, action);
    }

    private void OnNativeMapVotesCleared()
    {
        RunState? runState = RunManager.Instance?.DebugOnlyGetState();
        MapSelectionSynchronizer? synchronizer = _effectMapSynchronizer;
        if (runState is null || synchronizer is null)
            return;

        foreach (NativeMapActionCandidate candidate in _nativeMapActions.Values.ToList())
        {
            if (!ReferenceEquals(candidate.Synchronizer, synchronizer))
                continue;
            if (!candidate.Action.CompletionTask.IsCompletedSuccessfully
                || candidate.Action.State != GameActionState.Finished
                || candidate.Source.actIndex != runState.MapLocation.actIndex
                || runState.MapLocation.coord != candidate.Destination
                || !runState.VisitedMapCoords.Contains(candidate.Destination)
                || candidate.MapGenerationCount != synchronizer.MapGenerationCount)
            {
                continue;
            }

            foreach (NativePendingOperation pending in _pending.Values)
                pending.TryRecordMapTransition(synchronizer, runState);
            if (!candidate.CheckpointGenerated)
            {
                candidate.CheckpointGenerated = true;
                GenerateSemanticChecksum(NativeSemanticContexts.MapTransition);
            }
        }
    }

    private static void GenerateSemanticChecksum(string context)
    {
        ChecksumTracker? tracker = RunManager.Instance?.ChecksumTracker;
        if (tracker is null || !tracker.IsEnabled)
            return;
        try
        {
            tracker.GenerateChecksum(context, null);
        }
        catch
        {
            // A semantic callback is observational. If the first-party tracker is unavailable,
            // no effect witness is exported.
        }
    }

    private void MarkActionCheckpoint(GameAction? action, string context)
    {
        if (action is not null
            && string.Equals(context, "finished action execution " + action,
                StringComparison.Ordinal))
        {
            _actionChecksummed.Add(action);
        }
    }

    private static bool IsNativePlayerChoiceResultLegal(
        RunState? runState, Player player, PlayerChoiceResult result)
    {
        if (runState is null || !runState.Players.Contains(player))
            return false;
        try
        {
            switch (result.ChoiceType)
            {
                case PlayerChoiceType.Index:
                    List<int> indexes = result.AsIndexes();
                    return indexes.Count <= 256
                        && indexes.Distinct().Count() == indexes.Count
                        && indexes.All(index => index >= 0 && index <= 4096);
                case PlayerChoiceType.Player:
                    ulong? playerId = result.AsPlayerId();
                    return playerId.HasValue
                        && runState.Players.Any(candidate => candidate.NetId == playerId.Value);
                default:
                    // The native wire intentionally admits only index/indexes/player. Card
                    // catalog entries are mutable combat state and must be selected through the
                    // first-party UI, which is the only public catalog owner.
                    return false;
            }
        }
        catch
        {
            return false;
        }
    }

    private static bool IsReachableMapDestination(
        RunState runState, MapLocation source, MapCoord destination)
    {
        if (source.coord is not { } sourceCoord)
            return false;
        MapPoint? sourcePoint = runState.Map.GetPoint(sourceCoord);
        return sourcePoint is not null
            && MapTravel.GetTravelablePointsFrom(runState, sourcePoint)
                .Any(point => point.coord == destination);
    }

    private static bool TryParseMapAction(
        GameAction action, out ulong ownerId, out MapCoord destination)
    {
        ownerId = 0;
        destination = default;
        string text = action.ToString() ?? string.Empty;
        const string prefix = "MoveToMapCoordAction ";
        if (!text.StartsWith(prefix, StringComparison.Ordinal))
            return false;
        string[] fields = text[prefix.Length..].Split(' ', 2, StringSplitOptions.None);
        if (fields.Length != 2 || !ulong.TryParse(fields[0], out ownerId))
            return false;
        const string coordPrefix = "MapCoord (";
        if (!fields[1].StartsWith(coordPrefix, StringComparison.Ordinal)
            || !fields[1].EndsWith(')'))
        {
            return false;
        }
        string[] coordinates = fields[1][coordPrefix.Length..^1]
            .Split(',', StringSplitOptions.TrimEntries);
        if (coordinates.Length != 2
            || !int.TryParse(coordinates[0], out int column)
            || !int.TryParse(coordinates[1], out int row)
            || column < 0 || row < 0)
        {
            return false;
        }
        destination = new MapCoord(column, row);
        return true;
    }
}
