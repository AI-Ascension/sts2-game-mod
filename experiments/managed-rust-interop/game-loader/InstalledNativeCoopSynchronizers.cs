// SPDX-License-Identifier: MIT

using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using MegaCrit.Sts2.Core.Entities.Actions;
using MegaCrit.Sts2.Core.Entities.Multiplayer;
using MegaCrit.Sts2.Core.Entities.Players;
using MegaCrit.Sts2.Core.Entities.TreasureRelicPicking;
using MegaCrit.Sts2.Core.Events;
using MegaCrit.Sts2.Core.GameActions;
using MegaCrit.Sts2.Core.GameActions.Multiplayer;
using MegaCrit.Sts2.Core.Map;
using MegaCrit.Sts2.Core.Models;
using MegaCrit.Sts2.Core.Multiplayer.Game;
using MegaCrit.Sts2.Core.Runs;

namespace AiAscension.Sts2GameMod.Runtime;

/// <summary>One native operation retained until its game-thread postcondition is observable.</summary>
internal sealed partial class NativePendingOperation
{
    private NativePendingOperation(string operationId, string effectKind,
        ulong beforeHostGeneration, ulong checksumOrdinalBefore, Func<bool> isComplete,
        bool canProduceEffect, GameAction? action, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds)
    {
        OperationId = operationId;
        EffectKind = effectKind;
        BeforeHostGeneration = beforeHostGeneration;
        ChecksumOrdinalBefore = checksumOrdinalBefore;
        IsComplete = isComplete;
        CanProduceEffect = canProduceEffect;
        Action = action;
        AuthorityEpoch = authorityEpoch;
        ParticipantNativeIds = participantNativeIds
            .Distinct()
            .OrderBy(id => id)
            .ToArray();
    }

    internal string OperationId { get; }
    internal string EffectKind { get; }
    internal ulong BeforeHostGeneration { get; }
    internal ulong ChecksumOrdinalBefore { get; }
    internal Func<bool> IsComplete { get; }
    internal bool CanProduceEffect { get; }
    internal GameAction? Action { get; private set; }
    internal string AuthorityEpoch { get; }
    internal IReadOnlyList<ulong> ParticipantNativeIds { get; }
    internal bool Completed { get; set; }

    /// <summary>
    /// Native checksum ordinal observed for this operation. The installed checksum callback has
    /// no operation ID, so it is accepted only after the exact action or semantic witness has
    /// already been recorded by the corresponding first-party callback.
    /// </summary>
    internal ulong? PassiveChecksumOrdinal { get; private set; }

    private NetChecksumData? LocalChecksum { get; set; }
    private readonly Dictionary<ulong, Dictionary<uint, NetChecksumData>> _remoteChecksums = new();

    private readonly EventSynchronizer? _eventSynchronizer;
    private readonly HashSet<EventModel> _eventModels = new();
    private readonly Player? _eventVoter;
    private readonly int _eventIndex;
    private bool _eventVoteObserved;

    private readonly TreasureRoomRelicSynchronizer? _relicSynchronizer;
    private readonly IReadOnlyList<RelicModel>? _relicSession;
    private readonly Player? _relicVoter;
    private readonly ulong _relicVoterId;
    private readonly int _relicIndex;
    private readonly bool _relicSkipped;
    private bool _relicAwardObserved;
    private readonly List<RelicAwardKey> _relicObtains = new();

    private readonly MapSelectionSynchronizer? _mapSynchronizer;
    private readonly RunState? _mapRunState;
    private readonly MapLocation _mapSource;
    private readonly MapVote _mapExpectedVote;
    private MapCoord? _mapDestination;
    private bool _mapTransitionObserved;
    private ulong _mapExpectedPlayerId;

    private readonly PlayerChoiceSynchronizer? _choiceSynchronizer;
    private readonly ulong _choicePlayerId;
    private readonly uint _choiceId;
    private readonly NetPlayerChoiceResult _choiceResult;
    private bool _choiceReceived;

    private bool SemanticReady { get; set; }
    private string? SemanticContext { get; set; }

    private readonly struct RelicAwardKey
    {
        internal RelicAwardKey(ulong playerId, ModelId relicId)
        {
            PlayerId = playerId;
            RelicId = relicId;
        }

        internal ulong PlayerId { get; }
        internal ModelId RelicId { get; }
    }

    private NativePendingOperation(string operationId, string effectKind,
        ulong beforeHostGeneration, ulong checksumOrdinalBefore, Func<bool> isComplete,
        bool canProduceEffect, GameAction? action, string authorityEpoch,
        IReadOnlyCollection<ulong> participantNativeIds,
        EventSynchronizer? eventSynchronizer = null,
        IEnumerable<EventModel>? eventModels = null,
        Player? eventVoter = null,
        int eventIndex = -1,
        TreasureRoomRelicSynchronizer? relicSynchronizer = null,
        IReadOnlyList<RelicModel>? relicSession = null,
        Player? relicVoter = null,
        ulong relicVoterId = 0,
        int relicIndex = -1,
        bool relicSkipped = false,
        MapSelectionSynchronizer? mapSynchronizer = null,
        RunState? mapRunState = null,
        MapLocation mapSource = default,
        MapVote mapExpectedVote = default,
        PlayerChoiceSynchronizer? choiceSynchronizer = null,
        ulong choicePlayerId = 0,
        uint choiceId = 0,
        NetPlayerChoiceResult choiceResult = default)
        : this(operationId, effectKind, beforeHostGeneration, checksumOrdinalBefore,
            isComplete, canProduceEffect, action, authorityEpoch, participantNativeIds)
    {
        _eventSynchronizer = eventSynchronizer;
        _eventVoter = eventVoter;
        _eventIndex = eventIndex;
        if (eventModels is not null)
        {
            foreach (EventModel eventModel in eventModels)
                _eventModels.Add(eventModel);
        }

        _relicSynchronizer = relicSynchronizer;
        _relicSession = relicSession;
        _relicVoter = relicVoter;
        _relicVoterId = relicVoterId;
        _relicIndex = relicIndex;
        _relicSkipped = relicSkipped;

        _mapSynchronizer = mapSynchronizer;
        _mapRunState = mapRunState;
        _mapSource = mapSource;
        _mapExpectedVote = mapExpectedVote;

        _choiceSynchronizer = choiceSynchronizer;
        _choicePlayerId = choicePlayerId;
        _choiceId = choiceId;
        _choiceResult = choiceResult;
    }

    /// <summary>
    /// Binds the checksum raised by the ordinary RunManager post-action callback. The action ID,
    /// rendered action text, and checksum context must all identify this exact action.
    /// </summary>
    internal bool TryRecordPassiveChecksum(
        ulong ordinal, string context, NetChecksumData checksum,
        NetFullCombatState fullState)
    {
        if (PassiveChecksumOrdinal.HasValue || Action is null)
            return false;
        if (!Action.Id.HasValue || fullState.lastExecutedActionId != Action.Id)
            return false;

        string expectedContext = "finished action execution " + (Action.ToString() ?? string.Empty);
        if (!string.Equals(context, expectedContext, StringComparison.Ordinal))
            return false;

        PassiveChecksumOrdinal = ordinal;
        LocalChecksum = checksum;
        return true;
    }

    /// <summary>
    /// Binds a semantic checksum generated after a non-action native effect. Null-action
    /// checksums cannot be correlated by the game's action ID, so the operation first has to
    /// prove the exact synchronizer callback and postcondition.
    /// </summary>
    internal bool TryRecordSemanticChecksum(
        ulong ordinal, string context, NetChecksumData checksum)
    {
        if (PassiveChecksumOrdinal.HasValue || Action is not null || !CanProduceEffect
            || !SemanticReady || SemanticContext is null
            || !string.Equals(context, SemanticContext, StringComparison.Ordinal))
        {
            return false;
        }

        PassiveChecksumOrdinal = ordinal;
        LocalChecksum = checksum;
        return true;
    }

    internal void RecordRemoteChecksum(ulong senderId, NetChecksumData checksum)
    {
        if (senderId != 0)
        {
            if (!_remoteChecksums.TryGetValue(senderId,
                    out Dictionary<uint, NetChecksumData>? byId))
            {
                byId = new Dictionary<uint, NetChecksumData>();
                _remoteChecksums[senderId] = byId;
            }
            byId[checksum.id] = checksum;
        }
    }

    internal bool HasMatchingRemoteChecksums(
        IEnumerable<ulong> connectedNativePeerIds, ulong localNativePeerId)
    {
        if (!LocalChecksum.HasValue)
            return false;

        NetChecksumData local = LocalChecksum.Value;
        HashSet<ulong> connected = connectedNativePeerIds.ToHashSet();
        bool foundRemote = false;
        foreach (ulong peerId in ParticipantNativeIds)
        {
            if (!connected.Contains(peerId))
                return false;
            if (peerId == localNativePeerId)
                continue;
            foundRemote = true;
            if (!_remoteChecksums.TryGetValue(peerId,
                    out Dictionary<uint, NetChecksumData>? byId)
                || !byId.TryGetValue(local.id, out NetChecksumData remote)
                || remote.checksum != local.checksum)
            {
                return false;
            }
        }
        return foundRemote;
    }

}

internal static class NativeSemanticContexts
{
    internal const string SharedEvent = "native semantic effect shared_event";
    internal const string TreasureRelic = "native semantic effect treasure_relic";
    internal const string MapTransition = "native semantic effect map_transition";
}
