// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.TreasureRelicPicking;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

internal sealed partial class NativePendingOperation
{
    internal static NativePendingOperation ForAction(
        string operationId, string actionKind, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, GameAction action, Player _, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds)
    {
        NativePendingOperation? pending = null;
        pending = new NativePendingOperation(
            operationId,
            $"native_{actionKind}_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => pending!.IsActionFinished(),
            canProduceEffect: true,
            action,
            authorityEpoch,
            participantNativeIds);
        return pending;
    }

    internal static NativePendingOperation ForEvent(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player player, int index,
        EventSynchronizer synchronizer,
        string authorityEpoch, IReadOnlyCollection<ulong> participantNativeIds)
    {
        NativePendingOperation? pending = null;
        EventModel[] eventModels = synchronizer.Events.ToArray();
        pending = new NativePendingOperation(
            operationId,
            "native_shared_event_vote_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => pending!.SemanticReady,
            canProduceEffect: true,
            action: null,
            authorityEpoch,
            participantNativeIds,
            eventSynchronizer: synchronizer,
            eventModels: eventModels,
            eventVoter: player,
            eventIndex: index);
        return pending;
    }

    internal static NativePendingOperation ForRelic(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player player,
        TreasureRoomRelicSynchronizer synchronizer,
        IReadOnlyList<RelicModel> relicSession, int relicIndex, bool skipped,
        string authorityEpoch, IReadOnlyCollection<ulong> participantNativeIds)
    {
        NativePendingOperation? pending = null;
        pending = new NativePendingOperation(
            operationId,
            "native_treasure_relic_vote_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => pending!.SemanticReady,
            canProduceEffect: true,
            action: null,
            authorityEpoch,
            participantNativeIds,
            relicSynchronizer: synchronizer,
            relicSession: relicSession,
            relicVoter: player,
            relicVoterId: player.NetId,
            relicIndex: relicIndex,
            relicSkipped: skipped);
        return pending;
    }

    internal static NativePendingOperation ForMap(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player player,
        MapSelectionSynchronizer synchronizer, MapVote expectedVote,
        RunState runState, MapLocation source, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds)
    {
        NativePendingOperation? pending = null;
        pending = new NativePendingOperation(
            operationId,
            "native_map_vote_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => pending!.SemanticReady,
            canProduceEffect: true,
            action: null,
            authorityEpoch,
            participantNativeIds,
            mapSynchronizer: synchronizer,
            mapRunState: runState,
            mapSource: source,
            mapExpectedVote: expectedVote)
        {
            _mapExpectedPlayerId = player.NetId
        };
        return pending;
    }

    internal static NativePendingOperation ForPlayerChoice(
        string operationId, CoopVoteDomain domain, ulong beforeHostGeneration,
        ulong checksumOrdinalBefore, Player player,
        PlayerChoiceSynchronizer synchronizer, uint choiceId,
        PlayerChoiceResult expectedResult, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds)
    {
        NativePendingOperation? pending = null;
        pending = new NativePendingOperation(
            operationId,
            "native_player_choice_vote_settled",
            beforeHostGeneration,
            checksumOrdinalBefore,
            () => pending!._choiceReceived && pending.IsActionFinished(),
            canProduceEffect: true,
            action: null,
            authorityEpoch,
            participantNativeIds,
            choiceSynchronizer: synchronizer,
            choicePlayerId: player.NetId,
            choiceId: choiceId,
            choiceResult: expectedResult.ToNetData());
        return pending;
    }

    private bool IsActionFinished() =>
        Action is not null
        && Action.State == GameActionState.Finished
        && Action.CompletionTask.IsCompletedSuccessfully
        && Action.Exception is null;
}
