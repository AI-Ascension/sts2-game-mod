// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class NativePendingOperation
{
    /// <summary>
    /// Binds the exact map action selected by the first-party host. The private destination field
    /// is intentionally not read; its stable public rendering is checked for the owner and a
    /// parseable coordinate, then the postcondition verifies the actual RunState transition.
    /// </summary>
    internal bool TryBindMapAction(GameAction action)
    {
        if (_mapSynchronizer is null || Action is not null || action is not MoveToMapCoordAction
            || action.OwnerId != _mapExpectedPlayerId)
        {
            return false;
        }
        if (!TryParseMapAction(action, out ulong ownerId, out MapCoord destination)
            || ownerId != _mapExpectedPlayerId)
        {
            return false;
        }
        if (!IsReachableMapDestination(destination))
            return false;
        Action = action;
        _mapDestination = destination;
        return true;
    }

    internal bool TryRecordMapTransition(
        MapSelectionSynchronizer synchronizer, RunState runState)
    {
        if (!ReferenceEquals(_mapSynchronizer, synchronizer) || _mapTransitionObserved
            || Action is not MoveToMapCoordAction || _mapDestination is not { } destination
            || Action.State != GameActionState.Finished)
        {
            return false;
        }
        if (synchronizer.MapGenerationCount != _mapExpectedVote.mapGenerationCount)
            return false;
        MapLocation location = runState.MapLocation;
        if (location.actIndex != _mapSource.actIndex || location.coord != destination
            || !runState.VisitedMapCoords.Contains(destination))
        {
            return false;
        }
        _mapTransitionObserved = true;
        SemanticReady = true;
        SemanticContext = NativeSemanticContexts.MapTransition;
        return true;
    }

    private bool IsReachableMapDestination(MapCoord destination)
    {
        if (_mapRunState is null || _mapSource.coord is not { } sourceCoord)
            return false;
        MapPoint? sourcePoint = _mapRunState.Map.GetPoint(sourceCoord);
        return sourcePoint is not null
            && MapTravel.GetTravelablePointsFrom(_mapRunState, sourcePoint)
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
        if (fields.Length != 2 || !ulong.TryParse(fields[0], NumberStyles.None,
                CultureInfo.InvariantCulture, out ownerId))
        {
            return false;
        }
        const string coordPrefix = "MapCoord (";
        if (!fields[1].StartsWith(coordPrefix, StringComparison.Ordinal)
            || !fields[1].EndsWith(')'))
        {
            return false;
        }
        string[] coordinates = fields[1][coordPrefix.Length..^1]
            .Split(',', StringSplitOptions.TrimEntries);
        if (coordinates.Length != 2
            || !int.TryParse(coordinates[0], NumberStyles.None,
                CultureInfo.InvariantCulture, out int column)
            || !int.TryParse(coordinates[1], NumberStyles.None,
                CultureInfo.InvariantCulture, out int row)
            || column < 0 || row < 0)
        {
            return false;
        }
        destination = new MapCoord(column, row);
        return true;
    }

    internal bool TryRecordPlayerChoice(Player player, uint choiceId,
        NetPlayerChoiceResult result, GameAction? action)
    {
        if (_choiceSynchronizer is null || _choiceReceived || player.NetId != _choicePlayerId
            || choiceId != _choiceId || action is null
            || action.OwnerId != player.NetId
            || action.State != GameActionState.GatheringPlayerChoice
            || !Matches(_choiceResult, result))
        {
            return false;
        }
        Action = action;
        _choiceReceived = true;
        return true;
    }

    private static bool Matches(NetPlayerChoiceResult expected,
        NetPlayerChoiceResult actual) =>
        expected.type == actual.type
        && expected.playerId == actual.playerId
        && expected.mutableCardOwner == actual.mutableCardOwner
        && SequenceEqual(expected.indexes, actual.indexes)
        && SequenceEqual(expected.combatCards, actual.combatCards)
        && SequenceEqual(expected.deckCards, actual.deckCards)
        && SequenceEqual(expected.canonicalCards, actual.canonicalCards)
        && SequenceEqual(expected.mutableCards, actual.mutableCards);

    private static bool SequenceEqual<T>(IReadOnlyList<T>? expected,
        IReadOnlyList<T>? actual) =>
        (expected ?? Array.Empty<T>()).SequenceEqual(actual ?? Array.Empty<T>());
}
