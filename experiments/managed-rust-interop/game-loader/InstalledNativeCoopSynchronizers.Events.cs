// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.TreasureRelicPicking;
using MegaCrit.Sts2.Core.Events;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Multiplayer.Game;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class NativePendingOperation
{
    internal bool TryRecordEventVote(EventSynchronizer synchronizer)
    {
        if (!ReferenceEquals(_eventSynchronizer, synchronizer) || _eventModels.Count == 0)
            return false;
        try
        {
            if (!synchronizer.IsShared)
                return false;
            if (_eventVoter is null
                || synchronizer.GetPlayerVote(_eventVoter) != (uint)_eventIndex)
            {
                return false;
            }
        }
        catch
        {
            return false;
        }
        _eventVoteObserved = true;
        return true;
    }

    internal bool TryRecordEventState(EventSynchronizer synchronizer, EventModel eventModel)
    {
        if (!ReferenceEquals(_eventSynchronizer, synchronizer)
            || !_eventModels.Contains(eventModel) || !_eventVoteObserved)
        {
            return false;
        }

        _eventModels.Remove(eventModel);
        if (_eventModels.Count == 0)
        {
            SemanticReady = true;
            SemanticContext = NativeSemanticContexts.SharedEvent;
        }
        return true;
    }

    /// <summary>Records the authoritative relic award list emitted by the native synchronizer.</summary>
    internal bool TryRecordRelicAward(
        TreasureRoomRelicSynchronizer synchronizer,
        IReadOnlyList<RelicPickingResult> results)
    {
        if (!ReferenceEquals(_relicSynchronizer, synchronizer) || _relicAwardObserved
            || _relicSession is null)
        {
            return false;
        }

        IReadOnlyList<RelicModel>? currentRelics = synchronizer.CurrentRelics;
        if (currentRelics is null || !ReferenceEquals(currentRelics, _relicSession))
            return false;

        if (_relicVoter is null)
            return false;
        TreasureRoomRelicSynchronizer.PlayerVote vote;
        try
        {
            vote = synchronizer.GetPlayerVote(_relicVoter);
        }
        catch
        {
            return false;
        }
        if (!vote.voteReceived || vote.index.HasValue != !_relicSkipped
            || (!_relicSkipped && vote.index != _relicIndex))
        {
            return false;
        }

        foreach (RelicPickingResult result in results)
        {
            if (!_relicSession.Any(relic => ReferenceEquals(relic, result.relic)
                    || relic.Id == result.relic.Id))
            {
                return false;
            }
            if (result.type != RelicPickingResultType.Skipped && result.player is not null)
                _relicObtains.Add(new RelicAwardKey(result.player.NetId, result.relic.Id));
        }

        _relicAwardObserved = true;
        if (_relicObtains.Count == 0)
        {
            SemanticReady = true;
            SemanticContext = NativeSemanticContexts.TreasureRelic;
        }
        return true;
    }

    internal bool TryRecordRelicObtained(Player player, RelicModel relic)
    {
        if (_relicSynchronizer is null || !_relicAwardObserved)
            return false;
        int index = _relicObtains.FindIndex(item => item.PlayerId == player.NetId
            && item.RelicId == relic.Id);
        if (index < 0)
            return false;
        _relicObtains.RemoveAt(index);
        if (_relicObtains.Count == 0)
        {
            SemanticReady = true;
            SemanticContext = NativeSemanticContexts.TreasureRelic;
        }
        return true;
    }
}
